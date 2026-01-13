//use super::datafusion::functions::geospatial::register_udfs as register_geo_udfs;
use super::datafusion::functions::register_udfs;
use super::datafusion::type_planner::CustomTypePlanner;
#[cfg(feature = "dedicated-executor")]
use super::dedicated_executor::DedicatedExecutor;
use super::error::{self as ex_error, Result};
// TODO: We need to fix this after geodatafusion is updated to datafusion 47
//use geodatafusion::udf::native::register_native as register_geo_native;
use crate::datafusion::logical_analyzer::analyzer_rules;
use crate::datafusion::logical_optimizer::split_ordered_aggregates::SplitOrderedAggregates;
use crate::datafusion::physical_optimizer::physical_optimizer_rules;
use crate::datafusion::query_planner::CustomQueryPlanner;
use crate::models::QueryContext;
use crate::query::UserQuery;
use crate::query_types::QueryId;
use crate::running_queries::RunningQueries;
use crate::utils::Config;
use catalog::catalog_list::{DEFAULT_CATALOG, EmbucketCatalogList};
use catalog_metastore::Metastore;
#[cfg(feature = "state-store")]
use chrono::{TimeZone, Utc};
use dashmap::DashMap;
use datafusion::config::ConfigOptions;
use datafusion::execution::runtime_env::RuntimeEnv;
use datafusion::execution::{SessionStateBuilder, SessionStateDefaults};
use datafusion::prelude::{SessionConfig, SessionContext};
use datafusion::sql::planner::IdentNormalizer;
use datafusion_functions_json::register_all as register_json_udfs;
use functions::expr_planner::CustomExprPlanner;
use functions::register_udafs;
use functions::session_params::{SessionParams, SessionProperty};
use functions::table::register_udtfs;
use snafu::ResultExt;
#[cfg(feature = "state-store")]
use state_store::{SessionRecord, StateStore, Variable};
use std::collections::{HashMap, VecDeque};
use std::num::NonZero;
use std::sync::atomic::AtomicI64;
use std::sync::{Arc, RwLock};
use std::thread::available_parallelism;
use time::{Duration, OffsetDateTime};

pub const SESSION_INACTIVITY_EXPIRATION_SECONDS: i64 = 5 * 60;
static MINIMUM_PARALLEL_OUTPUT_FILES: usize = 1;
static PARALLEL_ROW_GROUP_RATIO: usize = 4;

#[must_use]
pub const fn to_unix(t: OffsetDateTime) -> i64 {
    // unix_timestamp is enough, no need to use nanoseconds precision
    t.unix_timestamp()
}

pub struct UserSession {
    pub metastore: Arc<dyn Metastore>,
    #[cfg(feature = "state-store")]
    state_store: Arc<dyn StateStore>,
    // running_queries contains all the queries running across sessions
    pub running_queries: Arc<dyn RunningQueries>,
    pub ctx: SessionContext,
    pub ident_normalizer: IdentNormalizer,
    #[cfg(feature = "dedicated-executor")]
    pub executor: DedicatedExecutor,
    pub config: Arc<Config>,
    pub expiry: AtomicI64,
    pub session_params: Arc<SessionParams>,
    pub recent_queries: Arc<RwLock<VecDeque<QueryId>>>,
    pub session_id: String,
    pub attrs: DashMap<String, String>,
}

impl UserSession {
    #[allow(clippy::unused_async)]
    pub async fn new(
        metastore: Arc<dyn Metastore>,
        running_queries: Arc<dyn RunningQueries>,
        config: Arc<Config>,
        catalog_list: Arc<EmbucketCatalogList>,
        runtime_env: Arc<RuntimeEnv>,
        session_id: &str,
        #[cfg(feature = "state-store")] state_store: Arc<dyn StateStore>,
    ) -> Result<Self> {
        let sql_parser_dialect = config
            .sql_parser_dialect
            .clone()
            .unwrap_or_else(|| "snowflake".to_string());

        let mut expr_planners = SessionStateDefaults::default_expr_planners();
        // That's a hack to use our custom expr planner first and default ones later. We probably need to get rid of default planners at some point.
        expr_planners.insert(0, Arc::new(CustomExprPlanner));

        let parallelism_opt = available_parallelism().ok().map(NonZero::get);

        let session_params = SessionParams::default();
        #[cfg(feature = "state-store")]
        let session_params_arc =
            Arc::new(Self::session_params(session_id, state_store.clone()).await);
        #[cfg(not(feature = "state-store"))]
        let session_params_arc = Arc::new(session_params.clone());
        let mut config_options = ConfigOptions::from_env().context(ex_error::DataFusionSnafu)?;

        // Only set minimum_parallel_output_files if environment variable wasn't set
        if std::env::var_os("DATAFUSION_EXECUTION_MINIMUM_PARALLEL_OUTPUT_FILES").is_none() {
            config_options.execution.minimum_parallel_output_files = MINIMUM_PARALLEL_OUTPUT_FILES;
        }

        let state = SessionStateBuilder::new()
            .with_config(
                SessionConfig::from(config_options)
                    .with_option_extension(session_params)
                    .with_information_schema(true)
                    // Cannot create catalog (database) automatic since it requires default volume
                    .with_create_default_catalog_and_schema(false)
                    .set_str("datafusion.sql_parser.dialect", &sql_parser_dialect)
                    .set_str("datafusion.catalog.default_catalog", DEFAULT_CATALOG)
                    .set_bool(
                        "datafusion.execution.skip_physical_aggregate_schema_check",
                        true,
                    )
                    .set_bool("datafusion.sql_parser.parse_float_as_decimal", true)
                    .set_usize(
                        "datafusion.execution.parquet.maximum_parallel_row_group_writers",
                        parallelism_opt.map_or(1, |x| (x / PARALLEL_ROW_GROUP_RATIO).max(1)),
                    )
                    .set_usize(
                        "datafusion.execution.parquet.maximum_buffered_record_batches_per_stream",
                        parallelism_opt.map_or(1, |x| 1 + (x / PARALLEL_ROW_GROUP_RATIO).max(1)),
                    ),
            )
            .with_default_features()
            .with_runtime_env(runtime_env)
            .with_catalog_list(catalog_list)
            .with_query_planner(Arc::new(CustomQueryPlanner::default()))
            .with_type_planner(Arc::new(CustomTypePlanner::default()))
            .with_analyzer_rules(analyzer_rules(session_params_arc.clone()))
            .with_optimizer_rule(Arc::new(SplitOrderedAggregates::new()))
            .with_physical_optimizer_rules(physical_optimizer_rules())
            .with_expr_planners(expr_planners)
            .build();
        let mut ctx = SessionContext::new_with_state(state);
        register_udfs(&mut ctx, &session_params_arc).context(ex_error::RegisterUDFSnafu)?;
        register_udafs(&mut ctx).context(ex_error::RegisterUDAFSnafu)?;
        register_udtfs(&ctx);
        register_json_udfs(&mut ctx).context(ex_error::RegisterUDFSnafu)?;
        //register_geo_native(&ctx);
        //register_geo_udfs(&ctx);

        let enable_ident_normalization = ctx.enable_ident_normalization();
        let session = Self {
            metastore,
            #[cfg(feature = "state-store")]
            state_store,
            running_queries,
            ctx,
            ident_normalizer: IdentNormalizer::new(enable_ident_normalization),
            #[cfg(feature = "dedicated-executor")]
            executor: DedicatedExecutor::builder().build(),
            config,
            expiry: AtomicI64::new(to_unix(
                OffsetDateTime::now_utc()
                    + Duration::seconds(SESSION_INACTIVITY_EXPIRATION_SECONDS),
            )),
            session_params: session_params_arc,
            recent_queries: Arc::new(RwLock::new(VecDeque::new())),
            session_id: session_id.to_string(),
            attrs: DashMap::new(),
        };
        Ok(session)
    }

    #[cfg(feature = "state-store")]
    pub async fn session_params(
        session_id: &str,
        state_store: Arc<dyn StateStore>,
    ) -> SessionParams {
        let session_params = SessionParams::default();
        #[cfg(feature = "state-store")]
        if let Some(params) = Self::get_session_state_params(session_id, state_store).await {
            session_params.set_properties(params);
        }
        session_params
    }

    #[cfg(feature = "state-store")]
    pub async fn get_session_state_params(
        session_id: &str,
        state_store: Arc<dyn StateStore>,
    ) -> Option<HashMap<String, SessionProperty>> {
        state_store.get_session(session_id).await.ok().map(|sr| {
            sr.variables
                .into_iter()
                .map(|(n, v)| (n, state_store_variable_to_property(v, session_id)))
                .collect()
        })
    }

    #[cfg(feature = "state-store")]
    pub async fn set_session_state_params(
        &self,
        set: bool,
        params: HashMap<String, SessionProperty>,
    ) -> Result<()> {
        let mut session_record =
            if let Ok(session) = self.state_store.get_session(&self.session_id).await {
                session
            } else {
                SessionRecord::new(&self.session_id)
            };
        let current_params: HashMap<String, SessionProperty> = session_record
            .variables
            .into_iter()
            .map(|(n, v)| (n, state_store_variable_to_property(v, &self.session_id)))
            .collect();
        let session_params = SessionParams::default();
        session_params.set_properties(current_params);

        if set {
            session_params.set_properties(params);
        } else {
            session_params.remove_properties(params);
        }
        session_record.variables = session_params_to_state_variables(&session_params);
        self.state_store
            .put_session(session_record)
            .await
            .context(ex_error::StateStoreSnafu)
    }

    #[tracing::instrument(
        name = "api_snowflake_rest::session::set_variable",
        level = "info",
        skip(self),
        err
    )]
    #[allow(dead_code)]
    async fn set_variable(&self, key: &str, value: &str) -> Result<()> {
        if key.is_empty() || value.is_empty() {
            return ex_error::OnyUseWithVariablesSnafu.fail();
        }
        let params = HashMap::from([(
            key.to_string(),
            SessionProperty::from_str_value(
                key.to_string(),
                value.to_string(),
                Some(self.session_id.clone()),
            ),
        )]);
        self.set_session_variable(true, params).await
    }

    pub fn query<S>(self: &Arc<Self>, query: S, query_context: QueryContext) -> UserQuery
    where
        S: Into<String>,
    {
        UserQuery::new(self.clone(), query.into(), query_context)
    }

    #[allow(clippy::unused_async)]
    pub async fn set_session_variable(
        &self,
        set: bool,
        params: HashMap<String, SessionProperty>,
    ) -> Result<()> {
        #[cfg(feature = "state-store")]
        self.set_session_state_params(set, params.clone()).await?;

        let state = self.ctx.state_ref();
        let mut write = state.write();

        let mut datafusion_params = Vec::new();
        let mut session_params = HashMap::new();

        for (key, prop) in params {
            if key.to_lowercase().starts_with("datafusion.") {
                datafusion_params.push((key.to_ascii_lowercase(), prop.value));
            } else {
                session_params.insert(key.to_ascii_lowercase(), prop);
            }
        }
        let options = write.config_mut().options_mut();
        for (key, value) in datafusion_params {
            options
                .set(&key, &value)
                .context(ex_error::DataFusionSnafu)?;
        }

        let config = options.extensions.get_mut::<SessionParams>();
        if let Some(cfg) = config {
            if set {
                cfg.set_properties(session_params);
            } else {
                cfg.remove_properties(session_params);
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn get_session_variable(&self, variable: &str) -> Option<String> {
        let state = self.ctx.state();
        let config = state.config().options().extensions.get::<SessionParams>();
        if let Some(cfg) = config {
            return cfg.properties.get(variable).map(|v| v.value.clone());
        }
        None
    }

    #[must_use]
    pub fn get_session_variable_bool(&self, variable: &str) -> bool {
        let state = self.ctx.state();
        let config = state.config().options().extensions.get::<SessionParams>();

        if let Some(cfg) = config
            && let Some(prop) = cfg.properties.get(variable)
            && let Some(parsed) = parse_bool(&prop.value)
        {
            return parsed;
        }
        false
    }

    pub fn record_query_id(&self, query_id: QueryId) {
        const MAX_QUERIES: usize = 64;
        if let Ok(mut guard) = self.recent_queries.write() {
            guard.push_front(query_id);
            while guard.len() > MAX_QUERIES {
                guard.pop_back();
            }
        }
    }
}

#[must_use]
pub fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

#[cfg(feature = "state-store")]
#[allow(clippy::as_conversions, clippy::cast_possible_wrap)]
pub fn state_store_variable_to_property(var: Variable, session_id: &str) -> SessionProperty {
    SessionProperty {
        session_id: Some(session_id.to_string()),
        created_on: Utc
            .timestamp_opt(var.created_at as i64, 0)
            .single()
            .unwrap_or_else(Utc::now),
        updated_on: var.updated_at.map_or_else(Utc::now, |ts| {
            Utc.timestamp_opt(ts as i64, 0)
                .single()
                .unwrap_or_else(Utc::now)
        }),
        value: var.value,
        property_type: var.value_type,
        comment: var.comment,
        name: var.name,
    }
}

#[cfg(feature = "state-store")]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
#[must_use]
pub fn session_params_to_state_variables(
    session_params: &SessionParams,
) -> HashMap<String, Variable> {
    session_params
        .properties
        .iter()
        .map(|entry| {
            let key = entry.key().clone();
            let prop = entry.value().clone();

            let created_at = prop.created_on.timestamp() as u64;
            let updated_at = Some(prop.updated_on.timestamp() as u64);

            let var = Variable {
                name: prop.name,
                value: prop.value,
                value_type: prop.property_type,
                comment: prop.comment,
                created_at,
                updated_at,
            };

            (key, var)
        })
        .collect()
}
