use super::error::{self as ex_error, Result};
use super::models::QueryResult;
use crate::query_types::{ExecutionStatus, QueryId, QueryStats};
use dashmap::DashMap;
use snafu::{OptionExt, ResultExt};
use std::sync::Arc;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

// RunningQuery can't be cloned and most of the time lives in RunningQueriesRegistry,
// it can't be used directly and only accessible by reference from RunningQueriesRegistry
#[derive(Debug)]
pub struct RunningQuery {
    pub query_id: QueryId,
    pub request_id: Option<Uuid>,
    // save result handle here, so when query finishes caller will retrieve handle
    // by removing RunningQuery from registry and will get result using result handle
    pub result_handle: Option<JoinHandle<Result<QueryResult>>>,
    pub cancellation_token: CancellationToken,
    // user can be notified when query is finished
    tx: watch::Sender<Option<ExecutionStatus>>,
    rx: watch::Receiver<Option<ExecutionStatus>>,
    pub query_stats: QueryStats,
}

#[derive(Debug, Clone)]
pub enum RunningQueryId {
    ByQueryId(QueryId),        // (query_id)
    ByRequestId(Uuid, String), // (request_id, sql_text)
}

impl RunningQuery {
    #[must_use]
    pub fn new(query_id: QueryId) -> Self {
        let (tx, rx) = watch::channel(None);
        Self {
            query_id,
            request_id: None,
            cancellation_token: CancellationToken::new(),
            result_handle: None,
            tx,
            rx,
            query_stats: QueryStats::default(),
        }
    }
    #[must_use]
    pub fn with_request_id(self, request_id: Option<Uuid>) -> Self {
        Self { request_id, ..self }
    }

    #[must_use]
    pub fn with_result_handle(self, result_handle: JoinHandle<Result<QueryResult>>) -> Self {
        Self {
            result_handle: Some(result_handle),
            ..self
        }
    }

    #[must_use]
    pub fn with_cancellation_token(self, cancellation_token: CancellationToken) -> Self {
        Self {
            cancellation_token,
            ..self
        }
    }

    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    #[tracing::instrument(
        name = "RunningQuery::notify_query_finished",
        level = "trace",
        skip(self),
        err
    )]
    pub fn notify_query_finished(
        &self,
        status: ExecutionStatus,
    ) -> std::result::Result<(), watch::error::SendError<Option<ExecutionStatus>>> {
        self.tx.send(Some(status))
    }

    #[tracing::instrument(name = "RunningQuery::wait_query_finished", level = "trace", err)]
    pub async fn wait_query_finished(
        mut rx: watch::Receiver<Option<ExecutionStatus>>,
    ) -> std::result::Result<ExecutionStatus, watch::error::RecvError> {
        // use loop here to bypass default query status we posted at init
        // it should not go to the actual loop and should resolve as soon as results are ready
        loop {
            rx.changed().await?;
            let status = *rx.borrow();
            if let Some(status) = status {
                break Ok(status);
            }
        }
    }

    #[tracing::instrument(name = "RunningQuery::update_query_stats", level = "trace", skip(self))]
    pub fn update_query_stats(&mut self, stats: &QueryStats) {
        if self.query_stats.query_type.is_none() {
            self.query_stats.query_type = stats.query_type.clone();
        }
    }
}

pub struct RunningQueriesRegistry {
    // <query_id, RunningQuery>
    queries: Arc<DashMap<QueryId, RunningQuery>>,
    // <request_id, QueryId> To associate request_id with query_id
    requests_ids: Arc<DashMap<Uuid, QueryId>>,
}

impl Default for RunningQueriesRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RunningQueriesRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            queries: Arc::new(DashMap::new()),
            requests_ids: Arc::new(DashMap::new()),
        }
    }

    #[tracing::instrument(
        name = "RunningQueriesRegistry::wait_query_finished",
        level = "trace",
        skip(self),
        err
    )]
    pub async fn wait_query_finished(&self, query_id: QueryId) -> Result<ExecutionStatus> {
        // Should not keep reference to RunningQuery during `wait_query_finished`
        // as it causes locking issues when accessing `queries` map during the run
        // outside of this call.
        let rx = {
            let running_query = self
                .queries
                .get(&query_id)
                .context(ex_error::QueryIsntRunningSnafu { query_id })?;
            running_query.rx.clone()
        };
        RunningQuery::wait_query_finished(rx)
            .await
            .context(ex_error::ExecutionStatusRecvSnafu { query_id })
    }
}

// RunningQueries interface allows cancel queries by query_id or request_id
#[async_trait::async_trait]
pub trait RunningQueries: Send + Sync {
    fn add(&self, running_query: RunningQuery);
    fn remove(&self, query_id: QueryId) -> Result<RunningQuery>;
    fn abort(&self, query_id: QueryId) -> Result<()>;
    fn notify_query_finished(&self, query_id: QueryId, status: ExecutionStatus) -> Result<()>;
    fn locate_query_id(&self, running_query_id: RunningQueryId) -> Result<QueryId>;
    fn count(&self) -> usize;
    fn cloned_stats(&self, query_id: QueryId) -> Option<QueryStats>;
    fn update_stats(&self, query_id: QueryId, stats: &QueryStats);
}

impl RunningQueries for RunningQueriesRegistry {
    #[tracing::instrument(name = "RunningQueriesRegistry::add", level = "trace", skip(self))]
    fn add(&self, running_query: RunningQuery) {
        // map query_id to request_id
        if let Some(request_id) = running_query.request_id {
            self.requests_ids.insert(request_id, running_query.query_id);
        }

        // map RunningQuery to query_id
        self.queries.insert(running_query.query_id, running_query);
    }

    #[tracing::instrument(name = "RunningQueriesRegistry::remove", level = "trace", skip(self))]
    fn remove(&self, query_id: QueryId) -> Result<RunningQuery> {
        let (_, running_query) = self
            .queries
            .remove(&query_id)
            .context(ex_error::QueryIsntRunningSnafu { query_id })?;
        Ok(running_query)
    }

    #[tracing::instrument(
        name = "RunningQueriesRegistry::abort",
        level = "trace",
        skip(self),
        fields(running_queries_count = self.count()),
        err
    )]
    fn abort(&self, query_id: QueryId) -> Result<()> {
        let running_query = self
            .queries
            .get(&query_id)
            .context(ex_error::QueryIsntRunningSnafu { query_id })?;
        running_query.cancel();
        Ok(())
    }

    #[tracing::instrument(
        name = "RunningQueriesRegistry::notify_query_finished",
        level = "trace",
        skip(self),
        err
    )]
    fn notify_query_finished(&self, query_id: QueryId, status: ExecutionStatus) -> Result<()> {
        let running_query = self
            .queries
            .get(&query_id)
            .context(ex_error::QueryIsntRunningSnafu { query_id })?;
        let _ = running_query.notify_query_finished(status);
        Ok(())
    }

    #[tracing::instrument(
        name = "RunningQueriesRegistry::locate_query_id",
        level = "trace",
        skip(self),
        ret
    )]
    fn locate_query_id(&self, running_query_id: RunningQueryId) -> Result<QueryId> {
        match running_query_id {
            RunningQueryId::ByRequestId(request_id, _sql_text) => Ok(*self
                .requests_ids
                .get(&request_id)
                .context(ex_error::QueryByRequestIdIsntRunningSnafu { request_id })?),
            RunningQueryId::ByQueryId(query_id) => Ok(query_id),
        }
    }

    fn count(&self) -> usize {
        self.queries.len()
    }

    fn cloned_stats(&self, query_id: QueryId) -> Option<QueryStats> {
        if let Some(running_query) = self.queries.get(&query_id) {
            Some(running_query.query_stats.clone())
        } else {
            None
        }
    }

    #[tracing::instrument(
        name = "RunningQueriesRegistry::update_stats",
        level = "trace",
        skip(self),
        fields(query_id),
        ret
    )]
    fn update_stats(&self, query_id: QueryId, stats: &QueryStats) {
        if let Some(mut running_query) = self.queries.get_mut(&query_id) {
            running_query.update_query_stats(stats);
        }
    }
}
