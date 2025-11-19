use super::error::{self as ex_error, Result};
use crate::query_types::{QueryId, QueryStatus};
use super::models::QueryResult;
use dashmap::DashMap;
use snafu::{OptionExt, ResultExt};
use std::sync::Arc;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug)]
pub struct RunningQuery {
    pub query_id: QueryId,
    pub request_id: Option<Uuid>,
    // save result handle here, so when query finishes caller will retrieve handle 
    // by removing RunningQuery from registry and get result using result handle
    pub result_handle: Option<JoinHandle<Result<QueryResult>>>,
    pub cancellation_token: CancellationToken,
    // user can be notified when query is finished
    tx: watch::Sender<QueryStatus>,
    rx: watch::Receiver<QueryStatus>,
}

#[derive(Debug, Clone)]
pub enum RunningQueryId {
    ByQueryId(QueryId),  // (query_id)
    ByRequestId(Uuid, String), // (request_id, sql_text)
}

impl RunningQuery {
    #[must_use]
    pub fn new(query_id: QueryId) -> Self {
        let (tx, rx) = watch::channel(QueryStatus::Running);
        Self {
            query_id,
            request_id: None,
            cancellation_token: CancellationToken::new(),
            result_handle: None,
            tx,
            rx,
        }
    }
    #[must_use]
    pub const fn with_request_id(mut self, request_id: Uuid) -> Self {
        self.request_id = Some(request_id);
        self
    }

    #[must_use]
    pub fn with_result_handle(mut self, result_handle: JoinHandle<Result<QueryResult>>) -> Self {
        self.result_handle = Some(result_handle);
        self
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
        status: QueryStatus,
    ) -> std::result::Result<(), watch::error::SendError<QueryStatus>> {
        self.tx.send(status)
    }

    #[tracing::instrument(
        name = "RunningQuery::wait",
        level = "trace",
        skip(self),
        err
    )]
    pub async fn wait(
        &self,
    ) -> std::result::Result<QueryStatus, watch::error::RecvError> {
        // use loop here to bypass default query status we posted at init
        // it should not go to the actual loop and should resolve as soon as results are ready
        let mut rx = self.rx.clone();
        loop {
            rx.changed().await?;
            let status = *rx.borrow();
            if status != QueryStatus::Running {
                break Ok(status);
            }
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
        name = "RunningQueriesRegistry::wait",
        level = "trace",
        skip(self),
        err
    )]
    pub async fn wait(&self, running_query_id: RunningQueryId) -> Result<QueryStatus> {
        let query_id = self.locate_query_id(running_query_id)?;
        let running_query = self
            .queries
            .get(&query_id)
            .context(ex_error::QueryIsntRunningSnafu { query_id })?;
        running_query
            .wait()
            .await
            .context(ex_error::QueryStatusRecvSnafu { query_id })
    }
}

// RunningQueries interface allows cancel queries by query_id or request_id
#[async_trait::async_trait]
pub trait RunningQueries: Send + Sync {
    fn add(&self, running_query: RunningQuery);
    fn remove(&self, running_query_id: RunningQueryId) -> Result<RunningQuery>;
    fn abort(&self, abort_query: RunningQueryId) -> Result<()>;
    fn notify_query_finished(&self, running_query_id: RunningQueryId, status: QueryStatus) -> Result<()>;
    fn locate_query_id(&self, running_query_id: RunningQueryId) -> Result<QueryId>;
    fn count(&self) -> usize;
}

impl RunningQueries for RunningQueriesRegistry {
    #[tracing::instrument(name = "RunningQueriesRegistry::add", level = "trace", skip(self))]
    fn add(&self, running_query: RunningQuery) {
        // map query_id to request_id
        if let Some(request_id) = running_query.request_id {
            self.requests_ids.insert(request_id, running_query.query_id);
        }

        // map RunningQuery to query_id
        self.queries
            .insert(running_query.query_id, running_query);
    }

    #[tracing::instrument(
        name = "RunningQueriesRegistry::remove",
        level = "trace",
        skip(self),
        err
    )]
    fn remove(&self, running_query_id: RunningQueryId) -> Result<RunningQuery> {
        let query_id = self.locate_query_id(running_query_id)?;
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
    fn abort(&self, abort_query: RunningQueryId) -> Result<()> {
        // Two phase mechanism:
        // 1 - cancel query using cancellation_token
        // 2 - ExecutionService removes RunningQuery from RunningQueriesRegistry
        let query_id = self.locate_query_id(abort_query)?;
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
    fn notify_query_finished(&self, running_query_id: RunningQueryId, status: QueryStatus) -> Result<()> {
        let query_id = self.locate_query_id(running_query_id)?;
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
            RunningQueryId::ByRequestId(request_id, _sql_text) => {
                Ok(*self
                    .requests_ids
                    .get(&request_id)
                    .context(ex_error::QueryByRequestIdIsntRunningSnafu { request_id })?)
            },
            RunningQueryId::ByQueryId(query_id) => Ok(query_id)
        }
    }

    fn count(&self) -> usize {
        self.queries.len()
    }
}
