use bytes::{Buf, Bytes};
use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::csv::ReaderBuilder;
use datafusion::arrow::csv::reader::Format;
use datafusion::catalog::CatalogProvider;
use datafusion::catalog::{MemoryCatalogProvider, MemorySchemaProvider};
use datafusion::common::runtime::set_join_set_tracer;
use datafusion::datasource::memory::MemTable;
use datafusion::execution::DiskManager;
use datafusion::execution::disk_manager::DiskManagerMode;
use datafusion::execution::memory_pool::{
    FairSpillPool, GreedyMemoryPool, MemoryPool, TrackConsumersPool,
};
use datafusion::execution::runtime_env::{RuntimeEnv, RuntimeEnvBuilder};
use datafusion_common::TableReference;
use snafu::ResultExt;
use std::num::NonZeroUsize;
use std::sync::atomic::Ordering;
use std::vec;
use std::{collections::HashMap, sync::Arc};
use time::{Duration as DateTimeDuration, OffsetDateTime};
use tokio::task;
use tokio_util::sync::CancellationToken;

use super::error::{self as ex_error, Result};
use super::models::{QueryContext, QueryResult};
use super::running_queries::{RunningQueries, RunningQueriesRegistry, RunningQuery};
use super::session::UserSession;
use crate::query_types::{QueryId, QueryStatus};
use crate::running_queries::RunningQueryId;
use crate::session::{SESSION_INACTIVITY_EXPIRATION_SECONDS, to_unix};
use crate::tracing::SpanTracer;
use crate::utils::{Config, MemPoolType};
use catalog::catalog_list::EmbucketCatalogList;
use catalog_metastore::{InMemoryMetastore, Metastore, TableIdent as MetastoreTableIdent};
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::Instrument;
use uuid::Uuid;

const DEFAULT_SCHEMA: &str = "public";

pub const TIMEOUT_SIGNAL_INTERVAL_SECONDS: u64 = 60;

#[async_trait::async_trait]
pub trait ExecutionService: Send + Sync {
    async fn create_session(&self, session_id: &str) -> Result<Arc<UserSession>>;
    async fn update_session_expiry(&self, session_id: &str) -> Result<bool>;
    async fn delete_expired_sessions(&self) -> Result<()>;
    async fn get_session(&self, session_id: &str) -> Result<Arc<UserSession>>;
    async fn session_exists(&self, session_id: &str) -> bool;
    async fn delete_session(&self, session_id: &str) -> Result<()>;
    fn get_sessions(&self) -> Arc<RwLock<HashMap<String, Arc<UserSession>>>>;

    /// Locates a query by `running_query_id`.
    ///
    /// # Arguments
    ///
    /// * `running_query_id` - The running query id.
    ///
    /// # Returns
    ///
    /// A `Result` of type `QueryId`. The `Ok` variant contains the query id,
    /// and the `Err` variant contains an `Error`.
    fn locate_query_id(&self, running_query_id: RunningQueryId) -> Result<QueryId>;

    /// Aborts a query by `query_id`.
    ///
    /// # Arguments
    ///
    /// * `query_id` - The query to abort.
    ///
    /// # Returns
    ///
    /// A `Result` of type `()`. The `Ok` variant contains an empty tuple,
    /// and the `Err` variant contains an `Error`.
    fn abort(&self, query_id: QueryId) -> Result<()>;

    /// Submits a query to be executed asynchronously. Query result can be consumed with
    /// `wait`.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The ID of the user session.
    /// * `query` - The SQL query to be executed.
    /// * `query_context` - The context of the query execution.
    ///
    /// # Returns
    ///
    /// A `Result` of type `QueryId`. The `Ok` variant contains the query id,
    /// to be used with `wait`. The `Err` variant contains submission `Error`.
    async fn submit(
        &self,
        session_id: &str,
        query: &str,
        query_context: QueryContext,
    ) -> Result<QueryId>;

    /// Wait while sabmitted query finished, it returns query result or real context rich error
    /// # Arguments
    ///
    /// * `query_id` - The id of the submitted query.
    ///
    /// # Returns
    ///
    /// A `Result` of type `QueryResult`. The `Ok` variant contains the query result,
    /// and the `Err` variant contains a real context rich error.
    async fn wait(&self, query_id: QueryId) -> Result<QueryResult>;

    /// Synchronously executes a query and returns the result.
    /// It is a wrapper around `submit` and `wait`.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The ID of the user session.
    /// * `query` - The SQL query to be executed.
    /// * `query_context` - The context of the query execution.
    ///
    /// # Returns
    ///
    /// A `Result` of type `QueryResult`. The `Ok` variant contains the query result,
    /// and the `Err` variant contains a real context rich error.
    async fn query(
        &self,
        session_id: &str,
        query: &str,
        query_context: QueryContext,
    ) -> Result<QueryResult>;

    async fn upload_data_to_table(
        &self,
        session_id: &str,
        table_ident: &MetastoreTableIdent,
        data: Bytes,
        file_name: &str,
        format: Format,
    ) -> Result<usize>;

    async fn timeout_signal(&self, interval: Duration, idle_timeout: Duration) -> ();
}

pub struct CoreExecutionService {
    metastore: Arc<dyn Metastore>,
    df_sessions: Arc<RwLock<HashMap<String, Arc<UserSession>>>>,
    config: Arc<Config>,
    catalog_list: Arc<EmbucketCatalogList>,
    runtime_env: Arc<RuntimeEnv>,
    queries: Arc<RunningQueriesRegistry>,
}

impl CoreExecutionService {
    #[tracing::instrument(
        name = "CoreExecutionService::new",
        level = "debug",
        skip(metastore, config),
        err
    )]
    pub async fn new(metastore: Arc<dyn Metastore>, config: Arc<Config>) -> Result<Self> {
        Self::initialize_datafusion_tracer();

        let catalog_list = Self::catalog_list(metastore.clone(), &config).await?;
        let runtime_env = Self::runtime_env(&config, catalog_list.clone())?;
        Ok(Self {
            metastore,
            df_sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
            catalog_list,
            runtime_env,
            queries: Arc::new(RunningQueriesRegistry::new()),
        })
    }

    #[tracing::instrument(
        name = "CoreExecutionService::catalog_list",
        level = "debug",
        skip(metastore),
        err
    )]
    pub async fn catalog_list(
        metastore: Arc<dyn Metastore>,
        config: &Config,
    ) -> Result<Arc<EmbucketCatalogList>> {
        let catalog_list = Arc::new(EmbucketCatalogList::new(metastore.clone(), config.into()));
        catalog_list
            .register_catalogs()
            .await
            .context(ex_error::RegisterCatalogSnafu)?;
        catalog_list
            .refresh()
            .await
            .context(ex_error::RefreshCatalogListSnafu)?;
        catalog_list
            .refresh_internal_catalogs()
            .await
            .context(ex_error::RefreshCatalogListSnafu)?;
        Ok(catalog_list)
    }

    #[allow(clippy::unwrap_used, clippy::as_conversions)]
    pub fn runtime_env(
        config: &Config,
        catalog_list: Arc<EmbucketCatalogList>,
    ) -> Result<Arc<RuntimeEnv>> {
        let mut rt_builder = RuntimeEnvBuilder::new().with_object_store_registry(catalog_list);

        if let Some(memory_limit_mb) = config.mem_pool_size_mb {
            const NUM_TRACKED_CONSUMERS: usize = 5;

            // set memory pool type
            let memory_limit = memory_limit_mb * 1024 * 1024;
            let enable_track = config.mem_enable_track_consumers_pool.unwrap_or(false);

            let memory_pool: Arc<dyn MemoryPool> = match config.mem_pool_type {
                MemPoolType::Fair => {
                    let pool = FairSpillPool::new(memory_limit);
                    if enable_track {
                        Arc::new(TrackConsumersPool::new(
                            pool,
                            NonZeroUsize::new(NUM_TRACKED_CONSUMERS).unwrap(),
                        ))
                    } else {
                        Arc::new(FairSpillPool::new(memory_limit))
                    }
                }
                MemPoolType::Greedy => {
                    let pool = GreedyMemoryPool::new(memory_limit);
                    if enable_track {
                        Arc::new(TrackConsumersPool::new(
                            pool,
                            NonZeroUsize::new(NUM_TRACKED_CONSUMERS).unwrap(),
                        ))
                    } else {
                        Arc::new(GreedyMemoryPool::new(memory_limit))
                    }
                }
            };
            rt_builder = rt_builder.with_memory_pool(memory_pool);
        }

        // set disk limit
        if let Some(disk_limit) = config.disk_pool_size_mb {
            let disk_limit_bytes = (disk_limit as u64) * 1024 * 1024;
            let disk_builder = DiskManager::builder()
                .with_mode(DiskManagerMode::OsTmpDirectory)
                .with_max_temp_directory_size(disk_limit_bytes);
            rt_builder = rt_builder.with_disk_manager_builder(disk_builder);
        }

        rt_builder.build_arc().context(ex_error::DataFusionSnafu)
    }

    fn initialize_datafusion_tracer() {
        let _ = set_join_set_tracer(&SpanTracer);
    }
}

#[async_trait::async_trait]
impl ExecutionService for CoreExecutionService {
    #[tracing::instrument(
        name = "ExecutionService::create_session",
        level = "debug",
        skip(self),
        fields(new_sessions_count),
        err
    )]
    async fn create_session(&self, session_id: &str) -> Result<Arc<UserSession>> {
        {
            let sessions = self.df_sessions.read().await;
            if let Some(session) = sessions.get(session_id) {
                return Ok(session.clone());
            }
        }
        let user_session: Arc<UserSession> = Arc::new(UserSession::new(
            self.metastore.clone(),
            self.queries.clone(),
            self.config.clone(),
            self.catalog_list.clone(),
            self.runtime_env.clone(),
        )?);
        {
            tracing::trace!("Acquiring write lock for df_sessions");
            let mut sessions = self.df_sessions.write().await;
            tracing::trace!("Acquired write lock for df_sessions");
            sessions.insert(session_id.to_string(), user_session.clone());

            // Record the result as part of the current span.
            tracing::Span::current().record("new_sessions_count", sessions.len());
        }
        Ok(user_session)
    }

    #[tracing::instrument(
        name = "ExecutionService::update_session_expiry",
        level = "debug",
        skip(self),
        fields(old_sessions_count, new_sessions_count, now),
        err
    )]
    async fn update_session_expiry(&self, session_id: &str) -> Result<bool> {
        let mut sessions = self.df_sessions.write().await;

        let res = if let Some(session) = sessions.get_mut(session_id) {
            let now = OffsetDateTime::now_utc();
            let new_expiry =
                to_unix(now + DateTimeDuration::seconds(SESSION_INACTIVITY_EXPIRATION_SECONDS));
            session.expiry.store(new_expiry, Ordering::Relaxed);

            // Record the result as part of the current span.
            tracing::Span::current().record("sessions_count", sessions.len());
            true
        } else {
            false
        };
        Ok(res)
    }

    #[tracing::instrument(
        name = "ExecutionService::delete_expired_sessions",
        level = "trace",
        skip(self),
        fields(old_sessions_count, new_sessions_count, now),
        err
    )]
    async fn delete_expired_sessions(&self) -> Result<()> {
        let now = to_unix(OffsetDateTime::now_utc());
        let mut sessions = self.df_sessions.write().await;

        let old_sessions_count = sessions.len();

        sessions.retain(|session_id, session| {
            let expiry = session.expiry.load(Ordering::Relaxed);
            if expiry <= now {
                let running_queries_count = session.running_queries.count();
                // prevent deleting sessions when session is expired but query is running
                if running_queries_count == 0 {
                    let _ = tracing::debug_span!(
                        "ExecutionService::delete_expired_session",
                        session_id,
                        expiry,
                        running_queries_count,
                        now
                    )
                    .entered();
                    false
                } else {
                    let _ = tracing::debug_span!(
                        "ExecutionService::skip_delete_expired_session",
                        session_id,
                        expiry,
                        running_queries_count,
                        now
                    )
                    .entered();
                    true
                }
            } else {
                true
            }
        });

        // Record the result as part of the current span.
        tracing::Span::current()
            .record("old_sessions_count", old_sessions_count)
            .record("new_sessions_count", sessions.len())
            .record("now", now);
        Ok(())
    }

    #[tracing::instrument(
        name = "ExecutionService::get_session",
        level = "debug",
        skip(self),
        err
    )]
    async fn get_session(&self, session_id: &str) -> Result<Arc<UserSession>> {
        let sessions = self.df_sessions.read().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| ex_error::MissingDataFusionSessionSnafu { id: session_id }.build())?;
        Ok(session.clone())
    }

    #[tracing::instrument(name = "ExecutionService::session_exists", level = "debug", skip(self))]
    async fn session_exists(&self, session_id: &str) -> bool {
        let sessions = self.df_sessions.read().await;
        sessions.contains_key(session_id)
    }

    #[tracing::instrument(
        name = "ExecutionService::delete_session",
        level = "debug",
        skip(self),
        fields(new_sessions_count),
        err
    )]
    async fn delete_session(&self, session_id: &str) -> Result<()> {
        let mut session_list = self.df_sessions.write().await;
        session_list.remove(session_id);

        // Record the result as part of the current span.
        tracing::Span::current().record("session_id", session_id);
        tracing::Span::current().record("new_sessions_count", session_list.len());
        Ok(())
    }
    fn get_sessions(&self) -> Arc<RwLock<HashMap<String, Arc<UserSession>>>> {
        self.df_sessions.clone()
    }

    #[tracing::instrument(
        name = "ExecutionService::query",
        level = "debug",
        skip(self),
        fields(query_id),
        err
    )]
    #[allow(clippy::large_futures)]
    async fn query(
        &self,
        session_id: &str,
        query: &str,
        query_context: QueryContext,
    ) -> Result<QueryResult> {
        let query_id = self.submit(session_id, query, query_context).await?;
        self.wait(query_id).await
    }

    #[tracing::instrument(name = "ExecutionService::wait", level = "debug", skip(self), err)]
    async fn wait(&self, query_id: QueryId) -> Result<QueryResult> {
        let _query_status = self.queries.wait_query_finished(query_id).await?;
        let running_query = self.queries.remove(query_id)?;
        if let Some(result_handle) = running_query.result_handle {
            result_handle
                .await
                .context(ex_error::AsyncResultTaskJoinSnafu { query_id })?
        } else {
            Err(ex_error::NoJoinHandleSnafu { query_id }.build())
        }
    }

    #[tracing::instrument(
        name = "ExecutionService::locate_query_id",
        level = "debug",
        skip(self)
    )]
    fn locate_query_id(&self, running_query_id: RunningQueryId) -> Result<QueryId> {
        self.queries.locate_query_id(running_query_id)
    }

    #[tracing::instrument(
        name = "ExecutionService::abort",
        level = "debug",
        skip(self),
        fields(old_queries_count = self.queries.count()),
        err
    )]
    fn abort(&self, query_id: QueryId) -> Result<()> {
        self.queries.abort(query_id)
    }

    #[tracing::instrument(
        name = "ExecutionService::submit",
        level = "debug",
        skip(self),
        fields(query_id, with_timeout_secs, old_queries_count = self.queries.count()),
        err
    )]
    async fn submit(
        &self,
        session_id: &str,
        query: &str,
        query_context: QueryContext,
    ) -> Result<QueryId> {
        let user_session = self.get_session(session_id).await?;

        if self.queries.count() >= self.config.max_concurrency_level {
            return ex_error::ConcurrencyLimitSnafu.fail();
        }

        let query_id = Uuid::new_v4();

        // Record the result as part of the current span.
        tracing::Span::current()
            .record("query_id", query_id.to_string())
            .record("with_timeout_secs", self.config.query_timeout_secs);

        let request_id = query_context.request_id;
        let query = query.to_string();
        let query_timeout_secs = self.config.query_timeout_secs;
        let queries_clone = self.queries.clone();
        let query_token = CancellationToken::new();
        let query_token_clone = query_token.clone();

        let task_span = tracing::info_span!("spawn_query_task");

        let alloc_span = tracing::info_span!(
            target: "alloc",
            "query_alloc",
            query_id = %query_id,
            session_id = %session_id
        );
        let handle = tokio::spawn(async move {
            let sub_task_span = tracing::info_span!("spawn_query_sub_task");
            let mut query_obj = user_session.query(query, query_context.with_query_id(query_id));

            // Create nested task so in case of abort/timeout it can be aborted
            // and result is handled properly (status / query result saved)
            let subtask_fut = task::spawn(async move {
                query_obj.execute().instrument(sub_task_span).await
            });
            let subtask_abort_handle = subtask_fut.abort_handle();

            // wait for any future to be resolved
            let (query_result, query_status) = tokio::select! {
                finished = subtask_fut => {
                    match finished {
                        Ok(inner_result) => {
                            // set query execution status to successful or failed
                            let status = inner_result.as_ref().map_or_else(|_| QueryStatus::Failed, |_| QueryStatus::Successful);
                            (inner_result.context(ex_error::QueryExecutionSnafu {
                                query_id,
                            }), status)
                        },
                        Err(error) => {
                            tracing::error!("Query {query_id} sub task join error: {error:?}");
                            (Err(ex_error::Error::QuerySubtaskJoin { error, location: snafu::location!() }).context(ex_error::QueryExecutionSnafu {
                                query_id,
                            }), QueryStatus::Failed)
                        },
                    }
                },
                () = query_token.cancelled() => {
                    tracing::info_span!("query_cancelled_do_abort");
                    subtask_abort_handle.abort();
                    (ex_error::QueryCancelledSnafu { query_id }.fail().context(ex_error::QueryExecutionSnafu {
                        query_id,
                    }), QueryStatus::Cancelled)
                },
                // Execute the query with a timeout to prevent long-running or stuck queries
                // from blocking system resources indefinitely. If the timeout is exceeded,
                // convert the timeout into a standard QueryTimeout error so it can be handled
                // and recorded like any other execution failure
                () = tokio::time::sleep(Duration::from_secs(query_timeout_secs)) => {
                    tracing::info_span!("query_timeout_received_do_abort");
                    subtask_abort_handle.abort();
                    (ex_error::QueryTimeoutSnafu.fail().context(ex_error::QueryExecutionSnafu {
                        query_id,
                    }), QueryStatus::TimedOut)
                }
            };

            let _ = tracing::info_span!("finished_query_status",
                query_id = query_id.to_string(),
                query_status = format!("{query_status:?}"),
            )
            .entered();

            user_session.record_query_id(query_id);

            // Notify subscribers query finishes and result is ready. 
            // Do not immediately remove query from running queries registry
            // as RunningQuery contains result handle that caller should consume.
            queries_clone.notify_query_finished(query_id, query_status)?;

            // Discard results after short timeout, to prevent memory leaks
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let running_query = queries_clone.remove(query_id);
                if let Ok(RunningQuery {result_handle: Some(result_handle), ..}) = running_query {
                    tracing::debug!("Discarding '{query_status:?}' result for query {query_id}");
                    let _ = result_handle.await;
                }
            });

            query_result
        }.instrument(alloc_span).instrument(task_span));

        self.queries.add(
            RunningQuery::new(query_id)
                .with_request_id(request_id)
                .with_result_handle(handle)
                .with_cancellation_token(query_token_clone),
        );

        Ok(query_id)
    }

    #[tracing::instrument(
        name = "ExecutionService::upload_data_to_table",
        level = "debug",
        skip(self, data),
        err,
        ret
    )]
    async fn upload_data_to_table(
        &self,
        session_id: &str,
        table_ident: &MetastoreTableIdent,
        data: Bytes,
        file_name: &str,
        format: Format,
    ) -> Result<usize> {
        // TODO: is there a way to avoid temp table approach altogether?
        // File upload works as follows:
        // 1. Convert incoming data to a record batch
        // 2. Create a temporary table in memory
        // 3. Use Execution service to insert data into the target table from the temporary table
        // 4. Drop the temporary table

        // use unique name to support simultaneous uploads
        let unique_id = Uuid::new_v4().to_string().replace('-', "_");
        let user_session = {
            let sessions = self.df_sessions.read().await;
            sessions
                .get(session_id)
                .ok_or_else(|| {
                    ex_error::MissingDataFusionSessionSnafu {
                        id: session_id.to_string(),
                    }
                    .build()
                })?
                .clone()
        };

        let source_table =
            TableReference::full("tmp_db", "tmp_schema", format!("tmp_table_{unique_id}"));
        let target_table = TableReference::full(
            table_ident.database.clone(),
            table_ident.schema.clone(),
            table_ident.table.clone(),
        );
        let inmem_catalog = MemoryCatalogProvider::new();
        inmem_catalog
            .register_schema(
                source_table.schema().unwrap_or_default(),
                Arc::new(MemorySchemaProvider::new()),
            )
            .context(ex_error::DataFusionSnafu)?;
        user_session.ctx.register_catalog(
            source_table.catalog().unwrap_or_default(),
            Arc::new(inmem_catalog),
        );
        // If target table already exists, we need to insert into it
        // otherwise, we need to create it
        let exists = user_session
            .ctx
            .table_exist(target_table.clone())
            .context(ex_error::DataFusionSnafu)?;

        let schema = if exists {
            let table = user_session
                .ctx
                .table(target_table)
                .await
                .context(ex_error::DataFusionSnafu)?;
            table.schema().as_arrow().to_owned()
        } else {
            let (schema, _) = format
                .infer_schema(data.clone().reader(), None)
                .context(ex_error::ArrowSnafu)?;
            schema
        };
        let schema = Arc::new(schema);

        // Here we create an arrow CSV reader that infers the schema from the entire dataset
        // (as `None` is passed for the number of rows) and then builds a record batch
        // TODO: This partially duplicates what Datafusion does with `CsvFormat::infer_schema`
        let csv = ReaderBuilder::new(schema.clone())
            .with_format(format)
            .build_buffered(data.reader())
            .context(ex_error::ArrowSnafu)?;

        let batches: std::result::Result<Vec<_>, _> = csv.collect();
        let batches = batches.context(ex_error::ArrowSnafu)?;

        let rows_loaded = batches
            .iter()
            .map(|batch: &RecordBatch| batch.num_rows())
            .sum();

        let table = MemTable::try_new(schema, vec![batches]).context(ex_error::DataFusionSnafu)?;
        user_session
            .ctx
            .register_table(source_table.clone(), Arc::new(table))
            .context(ex_error::DataFusionSnafu)?;

        let table = source_table.clone();
        let query = if exists {
            format!("INSERT INTO {table_ident} SELECT * FROM {table}")
        } else {
            format!("CREATE TABLE {table_ident} AS SELECT * FROM {table}")
        };

        let mut query = user_session.query(&query, QueryContext::default());
        Box::pin(query.execute()).await?;

        user_session
            .ctx
            .deregister_table(source_table)
            .context(ex_error::DataFusionSnafu)?;

        Ok(rows_loaded)
    }

    async fn timeout_signal(&self, interval: Duration, idle_timeout: Duration) -> () {
        let mut interval = tokio::time::interval(interval);
        interval.tick().await; // The first tick completes immediately; skip.
        let mut idle_since: Option<std::time::Instant> = None;
        loop {
            interval.tick().await;
            let sessions_empty = {
                let sessions = self.df_sessions.read().await;
                sessions.is_empty()
            };
            let queries_empty = self.queries.count() == 0;
            let idle_now = sessions_empty && queries_empty;
            match (idle_now, idle_since) {
                (true, None) => {
                    // just entered idle
                    idle_since = Some(std::time::Instant::now());
                }
                (true, Some(since)) => {
                    if since.elapsed() >= idle_timeout {
                        // stayed idle long enough
                        return;
                    }
                }
                (false, _) => {
                    // became active again, reset the idle window
                    idle_since = None;
                }
            }
        }
    }
}

//Test environment
#[allow(clippy::expect_used)]
pub async fn make_test_execution_svc() -> Arc<CoreExecutionService> {
    let metastore = Arc::new(InMemoryMetastore::new());
    Arc::new(
        CoreExecutionService::new(metastore, Arc::new(Config::default()))
            .await
            .expect("Failed to create a execution service"),
    )
}
