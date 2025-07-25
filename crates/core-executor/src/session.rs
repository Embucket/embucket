//use super::datafusion::functions::geospatial::register_udfs as register_geo_udfs;
use super::datafusion::functions::register_udfs;
use super::datafusion::type_planner::CustomTypePlanner;
use super::dedicated_executor::DedicatedExecutor;
use super::error::{self as ex_error, Result};
use crate::datafusion::logical_analyzer::iceberg_types_analyzer::IcebergTypesAnalyzer;
// TODO: We need to fix this after geodatafusion is updated to datafusion 47
//use geodatafusion::udf::native::register_native as register_geo_native;
use crate::datafusion::logical_analyzer::cast_analyzer::CastAnalyzer;
use crate::datafusion::physical_optimizer::physical_optimizer_rules;
use crate::datafusion::query_planner::CustomQueryPlanner;
use crate::models::QueryContext;
use crate::query::UserQuery;
use crate::utils::Config;
use core_history::history_store::HistoryStore;
use core_metastore::Metastore;
use datafusion::execution::runtime_env::RuntimeEnvBuilder;
use datafusion::execution::{SessionStateBuilder, SessionStateDefaults};
use datafusion::prelude::{SessionConfig, SessionContext};
use datafusion::sql::planner::IdentNormalizer;
use datafusion_functions_json::register_all as register_json_udfs;
use df_catalog::catalog_list::{DEFAULT_CATALOG, EmbucketCatalogList};
use df_catalog::information_schema::session_params::{SessionParams, SessionProperty};
use embucket_functions::expr_planner::CustomExprPlanner;
use embucket_functions::register_udafs;
use embucket_functions::table::register_udtfs;
use snafu::ResultExt;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use tokio::sync::Mutex;

pub const SESSION_INACTIVITY_EXPIRATION_SECONDS: i64 = 5 * 60;

pub struct UserSession {
    pub metastore: Arc<dyn Metastore>,
    pub history_store: Arc<dyn HistoryStore>,
    pub ctx: SessionContext,
    pub ident_normalizer: IdentNormalizer,
    pub executor: DedicatedExecutor,
    pub config: Arc<Config>,
    pub expiry: Arc<Mutex<OffsetDateTime>>,
}

impl UserSession {
    pub fn new(
        metastore: Arc<dyn Metastore>,
        history_store: Arc<dyn HistoryStore>,
        config: Arc<Config>,
        catalog_list: Arc<EmbucketCatalogList>,
    ) -> Result<Self> {
        let sql_parser_dialect =
            env::var("SQL_PARSER_DIALECT").unwrap_or_else(|_| "snowflake".to_string());

        let runtime_config = RuntimeEnvBuilder::new()
            .with_object_store_registry(catalog_list.clone())
            .build()
            .context(ex_error::DataFusionSnafu)?;

        let mut expr_planners = SessionStateDefaults::default_expr_planners();
        // That's a hack to use our custom expr planner first and default ones later. We probably need to get rid of default planners at some point.
        expr_planners.insert(0, Arc::new(CustomExprPlanner));

        let state = SessionStateBuilder::new()
            .with_config(
                SessionConfig::new()
                    .with_option_extension(SessionParams::default())
                    .with_information_schema(true)
                    // Cannot create catalog (database) automatic since it requires default volume
                    .with_create_default_catalog_and_schema(false)
                    .set_str("datafusion.sql_parser.dialect", &sql_parser_dialect)
                    .set_str("datafusion.catalog.default_catalog", DEFAULT_CATALOG)
                    .set_bool(
                        "datafusion.execution.skip_physical_aggregate_schema_check",
                        true,
                    )
                    .set_bool("datafusion.sql_parser.parse_float_as_decimal", true),
            )
            .with_default_features()
            .with_runtime_env(Arc::new(runtime_config))
            .with_catalog_list(catalog_list)
            .with_query_planner(Arc::new(CustomQueryPlanner::default()))
            .with_type_planner(Arc::new(CustomTypePlanner::default()))
            .with_analyzer_rule(Arc::new(IcebergTypesAnalyzer {}))
            .with_analyzer_rule(Arc::new(CastAnalyzer::new()))
            .with_physical_optimizer_rules(physical_optimizer_rules())
            .with_expr_planners(expr_planners)
            .build();
        let mut ctx = SessionContext::new_with_state(state);
        register_udfs(&mut ctx).context(ex_error::RegisterUDFSnafu)?;
        register_udafs(&mut ctx).context(ex_error::RegisterUDAFSnafu)?;
        register_udtfs(&ctx, history_store.clone());
        register_json_udfs(&mut ctx).context(ex_error::RegisterUDFSnafu)?;
        //register_geo_native(&ctx);
        //register_geo_udfs(&ctx);

        let enable_ident_normalization = ctx.enable_ident_normalization();
        let session = Self {
            metastore,
            history_store,
            ctx,
            ident_normalizer: IdentNormalizer::new(enable_ident_normalization),
            executor: DedicatedExecutor::builder().build(),
            config,
            expiry: Arc::from(Mutex::from(
                OffsetDateTime::now_utc()
                    + Duration::seconds(SESSION_INACTIVITY_EXPIRATION_SECONDS),
            )),
        };
        Ok(session)
    }

    pub fn query<S>(self: &Arc<Self>, query: S, query_context: QueryContext) -> UserQuery
    where
        S: Into<String>,
    {
        UserQuery::new(self.clone(), query.into(), query_context)
    }

    pub fn set_session_variable(
        &self,
        set: bool,
        params: HashMap<String, SessionProperty>,
    ) -> Result<()> {
        let state = self.ctx.state_ref();
        let mut write = state.write();

        let mut datafusion_params = Vec::new();
        let mut session_params = HashMap::new();

        for (key, prop) in params {
            if key.to_lowercase().starts_with("datafusion.") {
                datafusion_params.push((key, prop.value));
            } else {
                session_params.insert(key, prop);
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
                cfg.set_properties(session_params)
                    .context(ex_error::DataFusionSnafu)?;
            } else {
                cfg.remove_properties(session_params)
                    .context(ex_error::DataFusionSnafu)?;
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
}
