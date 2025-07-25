use super::catalog::information_schema::information_schema::{
    INFORMATION_SCHEMA, InformationSchemaProvider,
};
use super::catalog::{
    catalog_list::EmbucketCatalogList, catalogs::embucket::catalog::EmbucketCatalog,
};
use super::datafusion::planner::ExtendedSqlToRel;
use super::error::{
    self as ex_error, Error, ObjectType as ExistingObjectType, RefreshCatalogListSnafu, Result,
};
use super::session::UserSession;
use super::utils::{NormalizedIdent, is_logical_plan_effectively_empty};
use crate::datafusion::logical_plan::merge::MergeIntoCOWSink;
use crate::datafusion::physical_plan::merge::{
    DATA_FILE_PATH_COLUMN, MANIFEST_FILE_PATH_COLUMN, SOURCE_EXISTS_COLUMN, TARGET_EXISTS_COLUMN,
};
use crate::datafusion::rewriters::session_context::SessionContextExprRewriter;
use crate::error::{InvalidColumnIdentifierSnafu, MergeSourceNotSupportedSnafu};
use crate::models::{QueryContext, QueryResult};
use arrow_schema::SchemaBuilder;
use core_history::HistoryStore;
use core_metastore::{
    Metastore, SchemaIdent as MetastoreSchemaIdent,
    TableCreateRequest as MetastoreTableCreateRequest, TableFormat as MetastoreTableFormat,
    TableIdent as MetastoreTableIdent,
};
use datafusion::arrow::array::{Int64Array, RecordBatch};
use datafusion::arrow::datatypes::{DataType, Field, Schema as ArrowSchema, SchemaRef};
use datafusion::catalog::MemoryCatalogProvider;
use datafusion::catalog::{CatalogProvider, SchemaProvider};
use datafusion::datasource::DefaultTableSource;
use datafusion::datasource::default_table_source::provider_as_source;
use datafusion::execution::session_state::SessionContextProvider;
use datafusion::execution::session_state::SessionState;
use datafusion::logical_expr::{self, col};
use datafusion::logical_expr::{LogicalPlan, TableSource};
use datafusion::prelude::{CsvReadOptions, DataFrame};
use datafusion::scalar::ScalarValue;
use datafusion::sql::parser::{CreateExternalTable, Statement as DFStatement};
use datafusion::sql::planner::ParserOptions;
use datafusion::sql::resolve::resolve_table_references;
use datafusion::sql::sqlparser::ast::{
    CreateTable as CreateTableStatement, Expr, Ident, ObjectName, Query, SchemaName, Statement,
    TableFactor,
};
use datafusion::sql::statement::object_name_to_string;
use datafusion_common::{
    DFSchema, DataFusionError, ResolvedTableReference, SchemaReference, TableReference,
    plan_datafusion_err,
};
use datafusion_expr::conditional_expressions::CaseBuilder;
use datafusion_expr::logical_plan::dml::{DmlStatement, InsertOp, WriteOp};
use datafusion_expr::planner::ContextProvider;
use datafusion_expr::{
    BinaryExpr, CreateMemoryTable, DdlStatement, Expr as DFExpr, Extension, JoinType,
    LogicalPlanBuilder, Operator, Projection, ScalarUDF, SubqueryAlias, TryCast, and,
    build_join_schema, is_null, lit, or, when,
};
use datafusion_iceberg::DataFusionTable;
use datafusion_iceberg::catalog::catalog::IcebergCatalog;
use datafusion_iceberg::catalog::mirror::Mirror;
use datafusion_iceberg::catalog::schema::IcebergSchema;
use datafusion_iceberg::table::DataFusionTableConfigBuilder;
use df_catalog::catalog::CachingCatalog;
use df_catalog::catalog_list::CachedEntity;
use df_catalog::information_schema::session_params::SessionProperty;
use df_catalog::table::CachingTable;
use embucket_functions::conversion::to_timestamp::ToTimestampFunc;
use embucket_functions::semi_structured::variant::visitors::visit_all;
use embucket_functions::visitors::{
    copy_into_identifiers, fetch_to_limit, functions_rewriter, inline_aliases_in_query,
    json_element, qualify_in_query, select_expr_aliases, table_functions,
    table_functions_cte_relation, timestamp, top_limit,
    unimplemented::functions_checker::visit as unimplemented_functions_checker,
};
use iceberg_rust::catalog::Catalog;
use iceberg_rust::catalog::create::CreateTableBuilder;
use iceberg_rust::catalog::identifier::Identifier;
use iceberg_rust::catalog::tabular::Tabular;
use iceberg_rust::error::Error as IcebergError;
use iceberg_rust::spec::arrow::schema::new_fields_with_ids;
use iceberg_rust::spec::namespace::Namespace;
use iceberg_rust::spec::schema::Schema;
use iceberg_rust::spec::snapshot::Snapshot;
use iceberg_rust::spec::table_metadata::TableMetadata;
use iceberg_rust::spec::types::StructType;
use iceberg_rust::spec::values::Value as IcebergValue;
use iceberg_rust::table::manifest_list::snapshot_partition_bounds;
use object_store::ObjectStore;
use object_store::aws::AmazonS3Builder;
use snafu::ResultExt;
use sqlparser::ast::{
    AssignmentTarget, MergeAction, MergeClause, MergeClauseKind, MergeInsertKind, ObjectNamePart,
    ObjectType, PivotValueSource, ShowObjects, ShowStatementFilter, ShowStatementIn,
    TruncateTableTarget, Use, Value, visit_relations_mut,
};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::ops::ControlFlow;
use std::result::Result as StdResult;
use std::sync::Arc;
use tracing_attributes::instrument;
use url::Url;

pub struct UserQuery {
    pub metastore: Arc<dyn Metastore>,
    pub history_store: Arc<dyn HistoryStore>,
    pub raw_query: String,
    pub query: String,
    pub session: Arc<UserSession>,
    pub query_context: QueryContext,
}

pub enum IcebergCatalogResult {
    Catalog(Arc<dyn Catalog>),
    Result(Result<QueryResult>),
}

impl UserQuery {
    pub(super) fn new<S>(session: Arc<UserSession>, query: S, query_context: QueryContext) -> Self
    where
        S: Into<String> + Clone,
    {
        Self {
            metastore: session.metastore.clone(),
            history_store: session.history_store.clone(),
            raw_query: query.clone().into(),
            query: query.into(),
            session,
            query_context,
        }
    }

    pub fn parse_query(&self) -> std::result::Result<DFStatement, DataFusionError> {
        let state = self.session.ctx.state();
        let dialect = state.config().options().sql_parser.dialect.as_str();
        let mut statement = state.sql_to_statement(&self.raw_query, dialect)?;
        // it is designed to return DataFusionError, this is the reason why we
        // create DataFusionError manually. This is also requires manual unboxing.
        Self::postprocess_query_statement_with_validation(&mut statement).map_err(|e| match e {
            Error::DataFusion { error, .. } => *error,
            _ => DataFusionError::NotImplemented(e.to_string()),
        })?;
        Ok(statement)
    }

    pub async fn plan(&self) -> std::result::Result<LogicalPlan, DataFusionError> {
        let statement = self.parse_query()?;
        self.session.ctx.state().statement_to_plan(statement).await
    }

    fn current_database(&self) -> String {
        self.query_context
            .database
            .clone()
            .or_else(|| self.session.get_session_variable("database"))
            .unwrap_or_else(|| "embucket".to_string())
    }

    fn current_schema(&self) -> String {
        self.query_context
            .schema
            .clone()
            .or_else(|| self.session.get_session_variable("schema"))
            .unwrap_or_else(|| "public".to_string())
    }

    #[instrument(
        name = "UserQuery::refresh_catalog_partially",
        level = "debug",
        skip(self),
        err
    )]
    async fn refresh_catalog_partially(&self, entity: CachedEntity) -> Result<()> {
        if let Some(catalog_list_impl) = self
            .session
            .ctx
            .state()
            .catalog_list()
            .as_any()
            .downcast_ref::<EmbucketCatalogList>()
        {
            catalog_list_impl
                .invalidate_cache(entity.normalized())
                .await
                .context(RefreshCatalogListSnafu)?;
        }
        Ok(())
    }

    #[instrument(name = "UserQuery::drop_catalog", level = "debug", skip(self), err)]
    async fn drop_catalog(&self, catalog: &str, cascade: bool) -> Result<()> {
        if let Some(catalog_list_impl) = self
            .session
            .ctx
            .state()
            .catalog_list()
            .as_any()
            .downcast_ref::<EmbucketCatalogList>()
        {
            catalog_list_impl
                .drop_catalog(catalog, cascade)
                .await
                .context(ex_error::DropDatabaseSnafu)?;
        }
        Ok(())
    }

    #[instrument(name = "UserQuery::create_catalog", level = "debug", skip(self), err)]
    async fn create_catalog(&self, catalog: &str, volume: &str) -> Result<()> {
        if let Some(catalog_list_impl) = self
            .session
            .ctx
            .state()
            .catalog_list()
            .as_any()
            .downcast_ref::<EmbucketCatalogList>()
        {
            catalog_list_impl
                .create_catalog(catalog, volume)
                .await
                .context(ex_error::CreateDatabaseSnafu)?;
        }
        Ok(())
    }

    fn session_context_expr_rewriter(&self) -> SessionContextExprRewriter {
        let current_database = self.current_database();
        let schemas: Vec<String> = self
            .session
            .ctx
            .catalog(&current_database)
            .map(|c| {
                c.schema_names()
                    .iter()
                    .map(|schema| format!("{current_database}.{schema}"))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        SessionContextExprRewriter {
            database: current_database,
            schema: self.current_schema(),
            schemas,
            warehouse: "default".to_string(),
            session_id: self.session.ctx.session_id(),
            version: self.session.config.embucket_version.clone(),
            query_context: self.query_context.clone(),
            history_store: self.history_store.clone(),
        }
    }

    fn register_session_udfs(&self) {
        // TO_TIMESTAMP
        let format = self
            .session
            .get_session_variable("timestamp_input_format")
            .unwrap_or_else(|| "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string());
        let tz = self
            .session
            .get_session_variable("timezone")
            .unwrap_or_else(|| "America/Los_Angeles".to_string());

        let mapping = self
            .session
            .get_session_variable("timestamp_input_mapping")
            .unwrap_or_else(|| "timestamp_ntz".to_string());

        let funcs = [
            (
                if mapping == "timestamp_ntz" {
                    None
                } else {
                    Some(Arc::from(tz.clone()))
                },
                false,
                "to_timestamp".to_string(),
            ),
            (
                if mapping == "timestamp_ntz" {
                    None
                } else {
                    Some(Arc::from(tz.clone()))
                },
                true,
                "try_to_timestamp".to_string(),
            ),
            (None, false, "to_timestamp_ntz".to_string()),
            (None, true, "try_to_timestamp_ntz".to_string()),
            (
                Some(Arc::from(tz.clone())),
                false,
                "to_timestamp_tz".to_string(),
            ),
            (
                Some(Arc::from(tz.clone())),
                true,
                "try_to_timestamp_tz".to_string(),
            ),
            (
                Some(Arc::from(tz.clone())),
                false,
                "to_timestamp_ltz".to_string(),
            ),
            (
                Some(Arc::from(tz)),
                true,
                "try_to_timestamp_ltz".to_string(),
            ),
        ];

        for (tz, r#try, name) in funcs {
            self.session
                .ctx
                .register_udf(ScalarUDF::from(ToTimestampFunc::new(
                    tz,
                    format.clone(),
                    r#try,
                    name,
                )));
        }
    }

    #[instrument(name = "UserQuery::postprocess_query_statement", level = "trace", err)]
    pub fn postprocess_query_statement_with_validation(statement: &mut DFStatement) -> Result<()> {
        if let DFStatement::Statement(value) = statement {
            json_element::visit(value);
            functions_rewriter::visit(value);
            top_limit::visit(value);
            unimplemented_functions_checker(value).context(ex_error::UnimplementedFunctionSnafu)?;
            copy_into_identifiers::visit(value);
            select_expr_aliases::visit(value);
            inline_aliases_in_query::visit(value);
            fetch_to_limit::visit(value).context(ex_error::SqlParserSnafu)?;
            table_functions::visit(value);
            qualify_in_query::visit(value);
            timestamp::visit(value);
            table_functions_cte_relation::visit(value);
            visit_all(value);
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    #[instrument(
        name = "UserQuery::execute",
        level = "debug",
        skip(self),
        fields(statement),
        err
    )]
    pub async fn execute(&mut self) -> Result<QueryResult> {
        let statement = self.parse_query().context(ex_error::DataFusionSnafu)?;
        self.query = statement.to_string();
        self.register_session_udfs();

        // Record the result as part of the current span.
        tracing::Span::current().record("statement", format!("{statement:#?}"));

        // TODO: Code should be organized in a better way
        // 1. Single place to parse SQL strings into AST
        // 2. Single place to update AST
        // 3. Single place to construct Logical plan from this AST
        // 4. Single place to rewrite-optimize-adjust logical plan
        // etc
        if let DFStatement::Statement(s) = statement {
            match *s {
                Statement::AlterSession {
                    set,
                    session_params,
                } => {
                    let params = session_params
                        .options
                        .into_iter()
                        .map(|v| {
                            (
                                v.option_name.clone(),
                                SessionProperty::from_key_value(&v, self.session.ctx.session_id()),
                            )
                        })
                        .collect();

                    self.session.set_session_variable(set, params)?;
                    return self.status_response();
                }
                Statement::Use(entity) => {
                    let (variable, value) = match entity {
                        Use::Catalog(n) => ("catalog", n.to_string()),
                        Use::Schema(n) => ("schema", n.to_string()),
                        Use::Database(n) => ("database", n.to_string()),
                        Use::Warehouse(n) => ("warehouse", n.to_string()),
                        Use::Role(n) => ("role", n.to_string()),
                        Use::Object(n) => ("object", n.to_string()),
                        Use::SecondaryRoles(sr) => ("secondary_roles", sr.to_string()),
                        Use::Default => ("", String::new()),
                    };
                    if variable.is_empty() | value.is_empty() {
                        return ex_error::OnyUseWithVariablesSnafu.fail();
                    }
                    let params = HashMap::from([(
                        variable.to_string(),
                        SessionProperty::from_str_value(value, Some(self.session.ctx.session_id())),
                    )]);
                    self.session.set_session_variable(true, params)?;
                    return self.status_response();
                }
                Statement::SetVariable {
                    variables, value, ..
                } => {
                    let values: Vec<SessionProperty> = value
                        .iter()
                        .map(|v| match v {
                            Expr::Value(v) => Ok(SessionProperty::from_value(
                                &v.value,
                                self.session.ctx.session_id(),
                            )),
                            _ => ex_error::OnlyPrimitiveStatementsSnafu.fail(),
                        })
                        .collect::<std::result::Result<_, _>>()?;
                    let params = variables
                        .iter()
                        .map(ToString::to_string)
                        .zip(values.into_iter())
                        .collect();
                    self.session.set_session_variable(true, params)?;
                    return self.status_response();
                }
                Statement::CreateTable { .. } => {
                    return Box::pin(self.create_table_query(*s)).await;
                }
                Statement::CreateDatabase {
                    db_name,
                    if_not_exists,
                    external_volume,
                    ..
                } => {
                    return self
                        .create_database(db_name, if_not_exists, external_volume)
                        .await;
                }
                Statement::CreateSchema { .. } => return Box::pin(self.create_schema(*s)).await,
                Statement::CreateStage { .. } => {
                    // We support only CSV uploads for now
                    return Box::pin(self.create_stage_query(*s)).await;
                }
                Statement::CopyIntoSnowflake { .. } => {
                    return Box::pin(self.copy_into_snowflake_query(*s)).await;
                }
                Statement::AlterTable { .. }
                | Statement::StartTransaction { .. }
                | Statement::Commit { .. }
                | Statement::Rollback { .. }
                | Statement::Update { .. } => return self.status_response(),
                Statement::Insert { .. } => {
                    return Box::pin(self.execute_with_custom_plan(&self.query)).await;
                }
                Statement::ShowDatabases { .. }
                | Statement::ShowSchemas { .. }
                | Statement::ShowTables { .. }
                | Statement::ShowColumns { .. }
                | Statement::ShowViews { .. }
                | Statement::ShowFunctions { .. }
                | Statement::ShowObjects { .. }
                | Statement::ShowVariables { .. }
                | Statement::ShowVariable { .. } => return Box::pin(self.show_query(*s)).await,
                Statement::Truncate { table_names, .. } => {
                    return Box::pin(self.truncate_table(table_names)).await;
                }
                Statement::Query(mut subquery) => {
                    self.traverse_and_update_query(subquery.as_mut()).await;
                    return Box::pin(self.execute_with_custom_plan(&subquery.to_string())).await;
                }
                Statement::Drop { .. } => return Box::pin(self.drop_query(*s)).await,
                Statement::Merge { .. } => return Box::pin(self.merge_query(*s)).await,
                _ => {}
            }
        } else if let DFStatement::CreateExternalTable(cetable) = statement {
            return Box::pin(self.create_external_table_query(cetable)).await;
        }
        self.execute_sql(&self.query).await
    }

    #[instrument(name = "UserQuery::get_catalog", level = "trace", skip(self), err)]
    pub fn get_catalog(&self, name: &str) -> Result<Arc<dyn CatalogProvider>> {
        self.session
            .ctx
            .state()
            .catalog_list()
            .catalog(name)
            .ok_or_else(|| {
                ex_error::CatalogNotFoundSnafu {
                    catalog: name.to_string(),
                }
                .build()
            })
    }

    #[instrument(name = "UserQuery::get_iceberg_mirror", level = "trace")]
    fn get_iceberg_mirror(catalog: &Arc<dyn CatalogProvider>) -> Option<Arc<Mirror>> {
        let caching_catalog = catalog.as_any().downcast_ref::<CachingCatalog>()?;
        let iceberg_catalog = caching_catalog
            .catalog
            .as_any()
            .downcast_ref::<IcebergCatalog>()?;
        Some(iceberg_catalog.mirror())
    }

    /// The code below relies on [`Catalog`] trait for different iceberg catalog
    /// implementations (REST, S3 table buckets, or anything else).
    /// In case this is built-in datafusion's [`MemoryCatalogProvider`] we shortcut and rely on its implementation
    /// to actually execute logical plan.
    /// Otherwise, code tries to downcast catalog to [`Catalog`] and if successful,
    /// return the catalog
    #[instrument(
        name = "UserQuery::resolve_iceberg_catalog_or_execute",
        level = "trace",
        skip(self, catalog)
    )]
    pub async fn resolve_iceberg_catalog_or_execute(
        &self,
        catalog: Arc<dyn CatalogProvider>,
        catalog_name: String,
        plan: LogicalPlan,
    ) -> IcebergCatalogResult {
        // Try to downcast to CachingCatalog first since all catalogs are registered as CachingCatalog
        let catalog =
            if let Some(caching_catalog) = catalog.as_any().downcast_ref::<CachingCatalog>() {
                &caching_catalog.catalog
            } else {
                return IcebergCatalogResult::Result(
                    ex_error::CatalogDownCastSnafu {
                        catalog: catalog_name,
                    }
                    .fail(),
                );
            };

        // Try to resolve the actual underlying catalog type
        if let Some(iceberg_catalog) = catalog.as_any().downcast_ref::<IcebergCatalog>() {
            IcebergCatalogResult::Catalog(iceberg_catalog.catalog())
        } else if let Some(embucket_catalog) = catalog.as_any().downcast_ref::<EmbucketCatalog>() {
            IcebergCatalogResult::Catalog(embucket_catalog.catalog())
        } else if catalog
            .as_any()
            .downcast_ref::<MemoryCatalogProvider>()
            .is_some()
        {
            let result = self.execute_logical_plan(plan).await;
            IcebergCatalogResult::Result(result)
        } else {
            IcebergCatalogResult::Result(
                ex_error::CatalogDownCastSnafu {
                    catalog: catalog_name,
                }
                .fail(),
            )
        }
    }

    #[instrument(name = "UserQuery::drop_query", level = "trace", skip(self), err)]
    pub async fn drop_query(&self, statement: Statement) -> Result<QueryResult> {
        let Statement::Drop {
            object_type,
            names,
            cascade,
            ..
        } = statement.clone()
        else {
            return ex_error::OnlyDropStatementsSnafu.fail();
        };

        // DROP DATABASE is a special case, since it is not a part of iceberg catalog
        if object_type == ObjectType::Database {
            if let Some(database) = names.first() {
                self.drop_catalog(&object_name_to_string(database), cascade)
                    .await?;
                return self.status_response();
            }
            let database_name = names
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            return ex_error::InvalidDatabaseIdentifierSnafu {
                ident: database_name,
            }
            .fail();
        }

        let plan = self.sql_statement_to_plan(statement).await?;
        let table_ref = match plan {
            LogicalPlan::Ddl(ref ddl) => match ddl {
                DdlStatement::DropTable(t) => self.resolve_table_ref(t.name.clone()),
                DdlStatement::DropView(v) => self.resolve_table_ref(v.name.clone()),
                DdlStatement::DropCatalogSchema(s) => self.resolve_schema_ref(s.name.clone()),
                _ => return ex_error::OnlyDropStatementsSnafu.fail(),
            },
            _ => return ex_error::OnlyDropStatementsSnafu.fail(),
        };

        let catalog_name = table_ref.catalog.as_ref();
        let schema_name = table_ref.schema.to_string();
        let ident = Identifier::new(&[schema_name.clone()], table_ref.table.as_ref());

        let catalog = self.get_catalog(catalog_name)?;
        let iceberg_catalog = match self
            .resolve_iceberg_catalog_or_execute(catalog.clone(), catalog_name.to_string(), plan)
            .await
        {
            IcebergCatalogResult::Catalog(catalog) => catalog,
            IcebergCatalogResult::Result(result) => {
                return result.map(|_| self.status_response())?;
            }
        };

        match object_type {
            ObjectType::Table | ObjectType::View => {
                if iceberg_catalog.clone().load_tabular(&ident).await.is_ok() {
                    iceberg_catalog
                        .drop_table(&ident)
                        .await
                        .context(ex_error::IcebergSnafu)?;
                    self.refresh_catalog_partially(CachedEntity::Table(MetastoreTableIdent {
                        database: catalog_name.to_string(),
                        schema: schema_name,
                        table: ident.name().to_string(),
                    }))
                    .await?;
                }
                self.status_response()
            }
            ObjectType::Schema => {
                let namespace = ident.namespace();
                if iceberg_catalog
                    .clone()
                    .namespace_exists(namespace)
                    .await
                    .is_ok()
                {
                    iceberg_catalog
                        .drop_namespace(namespace)
                        .await
                        .context(ex_error::IcebergSnafu)?;
                    if Self::get_iceberg_mirror(&catalog).is_some() {
                        catalog
                            .deregister_schema(&schema_name.clone(), cascade)
                            .context(ex_error::DataFusionSnafu)?;
                    }
                    self.refresh_catalog_partially(CachedEntity::Schema(MetastoreSchemaIdent {
                        database: catalog_name.to_string(),
                        schema: schema_name,
                    }))
                    .await?;
                }
                self.status_response()
            }
            _ => ex_error::OnlyDropStatementsSnafu.fail(),
        }
    }

    #[allow(clippy::redundant_else, clippy::too_many_lines)]
    #[instrument(
        name = "UserQuery::create_table_query",
        level = "trace",
        skip(self),
        err
    )]
    pub async fn create_table_query(&self, statement: Statement) -> Result<QueryResult> {
        let Statement::CreateTable(mut create_table_statement) = statement.clone() else {
            return ex_error::OnlyCreateTableStatementsSnafu.fail();
        };
        let table_location = create_table_statement
            .location
            .clone()
            .or_else(|| create_table_statement.base_location.clone());

        let new_table_ident =
            self.resolve_table_object_name(create_table_statement.name.0.clone())?;
        create_table_statement.name = new_table_ident.clone().into();
        // We don't support transient tables for now
        create_table_statement.transient = false;
        // Remove all unsupported iceberg params (we already take them into account)
        create_table_statement.iceberg = false;
        create_table_statement.base_location = None;
        create_table_statement.external_volume = None;
        create_table_statement.catalog = None;
        create_table_statement.catalog_sync = None;
        create_table_statement.storage_serialization_policy = None;
        create_table_statement.cluster_by = None;

        let plan = self
            .get_custom_logical_plan(&create_table_statement.to_string())
            .await?;
        let ident: MetastoreTableIdent = new_table_ident.into();

        let catalog = self.get_catalog(ident.database.as_str())?;
        self.create_iceberg_table(
            catalog.clone(),
            ident.database.clone(),
            table_location,
            ident.clone(),
            create_table_statement,
            plan.clone(),
        )
        .await?;

        // Now we have created table in the metastore, we need to register it in the catalog
        self.refresh_catalog_partially(CachedEntity::Table(ident))
            .await?;

        // Insert data to new table
        // Since we don't execute logical plan, and we don't transform it to physical plan and
        // also don't execute it as well, we need somehow to support CTAS
        if let LogicalPlan::Ddl(DdlStatement::CreateMemoryTable(CreateMemoryTable {
            name,
            input,
            ..
        })) = plan
        {
            if is_logical_plan_effectively_empty(&input) {
                return self.created_entity_response();
            }
            let schema_name = name.schema().ok_or_else(|| {
                ex_error::InvalidSchemaIdentifierSnafu {
                    ident: name.to_string(),
                }
                .build()
            })?;

            let target_table = catalog
                .schema(schema_name)
                .ok_or_else(|| {
                    ex_error::SchemaNotFoundSnafu {
                        schema: schema_name.to_string(),
                    }
                    .build()
                })?
                .table(name.table())
                .await
                .context(ex_error::DataFusionSnafu)?
                .ok_or_else(|| {
                    ex_error::TableProviderNotFoundSnafu {
                        table_name: name.table().to_string(),
                    }
                    .build()
                })?;
            let schema = target_table.schema();
            let insert_plan = LogicalPlan::Dml(DmlStatement::new(
                name,
                provider_as_source(target_table),
                WriteOp::Insert(InsertOp::Append),
                cast_input_to_target_schema(input, &schema)?,
            ));
            return self.execute_logical_plan(insert_plan).await;
        }
        self.created_entity_response()
    }

    #[allow(unused_variables)]
    #[instrument(
        name = "UserQuery::create_iceberg_table",
        level = "trace",
        skip(self),
        err
    )]
    pub async fn create_iceberg_table(
        &self,
        catalog: Arc<dyn CatalogProvider>,
        catalog_name: String,
        table_location: Option<String>,
        ident: MetastoreTableIdent,
        statement: CreateTableStatement,
        plan: LogicalPlan,
    ) -> Result<QueryResult> {
        let iceberg_catalog = match self
            .resolve_iceberg_catalog_or_execute(catalog, catalog_name, plan.clone())
            .await
        {
            IcebergCatalogResult::Catalog(catalog) => catalog,
            IcebergCatalogResult::Result(result) => {
                return result.map(|_| self.created_entity_response())?;
            }
        };

        let iceberg_ident = ident.to_iceberg_ident();

        // Check if it already exists, if exists and CREATE OR REPLACE - drop it
        let table_exists = iceberg_catalog
            .clone()
            .load_tabular(&iceberg_ident)
            .await
            .is_ok();

        if table_exists {
            if statement.if_not_exists {
                return self.created_entity_response();
            }

            if statement.or_replace {
                iceberg_catalog
                    .drop_table(&iceberg_ident)
                    .await
                    .context(ex_error::IcebergSnafu)?;
            } else {
                return ex_error::ObjectAlreadyExistsSnafu {
                    r#type: ExistingObjectType::Table,
                    name: ident.to_string(),
                }
                .fail();
            }
        }

        let fields_with_ids = StructType::try_from(&new_fields_with_ids(
            plan.schema().as_arrow().fields(),
            &mut 0,
        ))
        .map_err(|err| DataFusionError::External(Box::new(err)))
        .context(ex_error::DataFusionSnafu)?;

        // Create builder and configure it
        let mut builder = Schema::builder();
        builder.with_schema_id(0);
        builder.with_identifier_field_ids(vec![]);

        // Add each struct field individually
        for field in fields_with_ids.iter() {
            builder.with_struct_field(field.clone());
        }

        let schema = builder
            .build()
            .map_err(|err| DataFusionError::External(Box::new(err)))
            .context(ex_error::DataFusionSnafu)?;

        let mut create_table = CreateTableBuilder::default();
        create_table
            .with_name(ident.table)
            .with_schema(schema)
            // .with_location(location.clone())
            .build(&[ident.schema], iceberg_catalog)
            .await
            .context(ex_error::IcebergSnafu)?;
        self.created_entity_response()
    }

    #[instrument(
        name = "UserQuery::create_external_table_query",
        level = "trace",
        skip(self),
        err
    )]
    pub async fn create_external_table_query(
        &self,
        statement: CreateExternalTable,
    ) -> Result<QueryResult> {
        let table_location = statement.location.clone();
        let table_format = MetastoreTableFormat::from(statement.file_type);
        let session_context = HashMap::new();
        let session_context_planner = SessionContextProvider {
            state: &self.session.ctx.state(),
            tables: session_context,
        };
        let planner = ExtendedSqlToRel::new(
            &session_context_planner,
            self.session.ctx.state().get_parser_options(),
        );
        let table_schema = planner
            .build_schema(statement.columns)
            .context(ex_error::DataFusionSnafu)?;
        let fields_with_ids =
            StructType::try_from(&new_fields_with_ids(table_schema.fields(), &mut 0))
                .map_err(|err| DataFusionError::External(Box::new(err)))
                .context(ex_error::DataFusionSnafu)?;

        // TODO: Use the options with the table format in the future
        let _table_options = statement.options.clone();
        let table_ident: MetastoreTableIdent =
            self.resolve_table_object_name(statement.name.0)?.into();

        // Create builder and configure it
        let mut builder = Schema::builder();
        builder.with_schema_id(0);
        builder.with_identifier_field_ids(vec![]);

        // Add each struct field individually
        for field in fields_with_ids.iter() {
            builder.with_struct_field(field.clone());
        }

        let table_schema = builder
            .build()
            .map_err(|err| DataFusionError::External(Box::new(err)))
            .context(ex_error::DataFusionSnafu)?;

        let table_create_request = MetastoreTableCreateRequest {
            ident: table_ident.clone(),
            schema: table_schema,
            location: Some(table_location),
            partition_spec: None,
            sort_order: None,
            stage_create: None,
            volume_ident: None,
            is_temporary: Some(false),
            format: Some(table_format),
            properties: None,
        };

        self.metastore
            .create_table(&table_ident, table_create_request)
            .await
            .context(ex_error::MetastoreSnafu)?;

        self.created_entity_response()
    }

    /// This is experimental CREATE STAGE support
    /// Current limitations
    /// TODO
    /// - Prepare object storage depending on the URL. Currently we support only s3 public buckets    ///   with public access with default eu-central-1 region
    /// - Parse credentials from specified config
    /// - We don't need to create table in case we have common shared session context.
    ///   CSV is registered as a table which can referenced from SQL statements executed against this context
    /// - Revisit this with the new metastore approach
    #[instrument(
        name = "UserQuery::create_stage_query",
        level = "trace",
        skip(self),
        err
    )]
    pub async fn create_stage_query(&self, statement: Statement) -> Result<QueryResult> {
        let Statement::CreateStage {
            name,
            stage_params,
            file_format,
            ..
        } = statement
        else {
            return ex_error::OnlyCreateStageStatementsSnafu.fail();
        };

        let table_name = match name.0.last() {
            Some(ObjectNamePart::Identifier(ident)) => ident.value.clone(),
            _ => {
                return ex_error::InvalidTableIdentifierSnafu {
                    ident: name.to_string(),
                }
                .fail();
            }
        };

        let skip_header = file_format.options.iter().any(|option| {
            option.option_name.eq_ignore_ascii_case("skip_header")
                && option.value.eq_ignore_ascii_case("1")
        });

        let field_optionally_enclosed_by = file_format
            .options
            .iter()
            .find_map(|option| {
                if option
                    .option_name
                    .eq_ignore_ascii_case("field_optionally_enclosed_by")
                {
                    Some(option.value.as_bytes()[0])
                } else {
                    None
                }
            })
            .unwrap_or(b'"');

        let file_path = stage_params.url.unwrap_or_default();
        let url = Url::parse(file_path.as_str()).map_err(|_| {
            ex_error::InvalidFilePathSnafu {
                path: file_path.clone(),
            }
            .build()
        })?;
        let bucket = url.host_str().unwrap_or_default();
        // TODO Replace this with the new metastore volume approach
        let s3 = AmazonS3Builder::from_env()
            // TODO Get region automatically from the Volume
            .with_region("eu-central-1")
            .with_bucket_name(bucket)
            .build()
            .map_err(|_| {
                ex_error::InvalidBucketIdentifierSnafu {
                    ident: bucket.to_string(),
                }
                .build()
            })?;

        self.session.ctx.register_object_store(&url, Arc::new(s3));

        // Read CSV file to get default schema
        let csv_data = self
            .session
            .ctx
            .read_csv(
                file_path.clone(),
                CsvReadOptions::new()
                    .has_header(skip_header)
                    .quote(field_optionally_enclosed_by),
            )
            .await
            .context(ex_error::DataFusionSnafu)?;

        let fields = csv_data
            .schema()
            .iter()
            .map(|(_, field)| {
                let data_type = if matches!(field.data_type(), DataType::Null) {
                    DataType::Utf8
                } else {
                    field.data_type().clone()
                };
                Field::new(field.name(), data_type, field.is_nullable())
            })
            .collect::<Vec<_>>();

        // Register CSV file with filled missing datatype with default Utf8
        self.session
            .ctx
            .register_csv(
                table_name,
                file_path,
                CsvReadOptions::new()
                    .has_header(skip_header)
                    .quote(field_optionally_enclosed_by)
                    .schema(&ArrowSchema::new(fields)),
            )
            .await
            .context(ex_error::DataFusionSnafu)?;
        self.status_response()
    }

    #[instrument(
        name = "UserQuery::copy_into_snowflake_query",
        level = "trace",
        skip(self),
        err
    )]
    pub async fn copy_into_snowflake_query(&self, statement: Statement) -> Result<QueryResult> {
        let Statement::CopyIntoSnowflake { into, from_obj, .. } = statement else {
            return ex_error::OnlyCopyIntoStatementsSnafu.fail();
        };
        if let Some(from_obj) = from_obj {
            let from_stage: Vec<ObjectNamePart> = from_obj
                .0
                .iter()
                .map(|fs| ObjectNamePart::Identifier(Ident::new(fs.to_string().replace('@', ""))))
                .collect();
            let insert_into = self.resolve_table_object_name(into.0)?;
            let insert_from = self.resolve_table_object_name(from_stage)?;
            // Insert data to table
            let insert_query = format!("INSERT INTO {insert_into} SELECT * FROM {insert_from}");
            self.execute_with_custom_plan(&insert_query).await
        } else {
            ex_error::FromObjectRequiredForCopyIntoStatementsSnafu.fail()
        }
    }

    #[allow(clippy::too_many_lines)]
    #[instrument(name = "UserQuery::merge_query", level = "trace", skip(self), err)]
    pub async fn merge_query(&self, statement: Statement) -> Result<QueryResult> {
        let Statement::Merge {
            table: target,
            source,
            on,
            clauses,
            ..
        } = statement
        else {
            return ex_error::OnlyMergeStatementsSnafu.fail();
        };
        let df_session_state = self.session.ctx.state();

        let mut session_context_provider = SessionContextProvider {
            state: &df_session_state,
            tables: HashMap::new(),
        };
        let mut planner_context = datafusion::sql::planner::PlannerContext::new();

        // Create a LogicalPlan for the source table

        let (source_plan, target_filter) = match source {
            TableFactor::Table {
                name: source_ident,
                alias: source_alias,
                ..
            } => {
                let source_ident = self.resolve_table_object_name(source_ident.0)?;

                let source_provider = self.get_iceberg_table_provider(&source_ident, None).await?;

                let target_filter = target_filter_expression(&source_provider).await?;

                let source_table_source: Arc<dyn TableSource> =
                    Arc::new(DefaultTableSource::new(Arc::new(source_provider)));

                session_context_provider.tables.insert(
                    self.resolve_table_ref(&source_ident),
                    source_table_source.clone(),
                );

                let plan = LogicalPlanBuilder::scan(&source_ident, source_table_source, None)
                    .context(ex_error::DataFusionLogicalPlanMergeSourceSnafu)?;
                let plan = if let Some(source_alias) = source_alias {
                    plan.alias(source_alias.name.to_string())
                        .context(ex_error::DataFusionLogicalPlanMergeSourceSnafu)?
                } else {
                    plan
                };
                let source_plan = DataFrame::new(
                    df_session_state.clone(),
                    plan.build()
                        .context(ex_error::DataFusionLogicalPlanMergeSourceSnafu)?,
                )
                .with_column(SOURCE_EXISTS_COLUMN, lit(true))
                .context(ex_error::DataFusionLogicalPlanMergeSourceSnafu)?
                .into_unoptimized_plan();
                Ok((source_plan, target_filter))
            }
            TableFactor::Derived {
                lateral: _,
                subquery,
                alias,
            } => {
                let query = Statement::Query(subquery.clone());

                let tables = self
                    .table_references_for_statement(
                        &DFStatement::Statement(Box::new(query.clone())),
                        &df_session_state,
                    )
                    .await?;

                session_context_provider.tables.extend(tables);

                let sql_planner =
                    ExtendedSqlToRel::new(&session_context_provider, ParserOptions::default());

                let source_plan = sql_planner
                    .sql_statement_to_plan(query)
                    .context(ex_error::DataFusionLogicalPlanMergeSourceSnafu)?;

                let source_plan = if let Some(alias) = alias {
                    LogicalPlan::SubqueryAlias(
                        SubqueryAlias::try_new(
                            Arc::new(source_plan),
                            TableReference::parse_str(&alias.to_string()),
                        )
                        .context(ex_error::DataFusionLogicalPlanMergeSourceSnafu)?,
                    )
                } else {
                    source_plan
                };

                let source_plan = DataFrame::new(df_session_state.clone(), source_plan)
                    .with_column(SOURCE_EXISTS_COLUMN, lit(true))
                    .context(ex_error::DataFusionLogicalPlanMergeSourceSnafu)?
                    .into_unoptimized_plan();

                Ok((source_plan, None))
            }
            _ => MergeSourceNotSupportedSnafu.fail(),
        }?;

        let source_schema = source_plan.schema().clone();

        // Create a LogicalPlan for the target table

        let TableFactor::Table {
            name: target_ident,
            alias: target_alias,
            ..
        } = target
        else {
            return ex_error::MergeTargetMustBeTableSnafu.fail();
        };

        let target_ident = self.resolve_table_object_name(target_ident.0)?;

        let target_table = self
            .get_iceberg_table_provider(
                &target_ident,
                Some(
                    //TODO Return proper Error
                    #[allow(clippy::unwrap_used)]
                    DataFusionTableConfigBuilder::default()
                        .enable_data_file_path_column(true)
                        .enable_manifest_file_path_column(true)
                        .build()
                        .unwrap(),
                ),
            )
            .await?;

        let target_table_source: Arc<dyn TableSource> =
            Arc::new(DefaultTableSource::new(Arc::new(target_table.clone())));

        session_context_provider.tables.insert(
            self.resolve_table_ref(&target_ident),
            target_table_source.clone(),
        );

        let plan = if let Some(target_filter) = target_filter {
            LogicalPlanBuilder::scan_with_filters(
                &target_ident,
                target_table_source,
                None,
                vec![target_filter],
            )
            .context(ex_error::DataFusionLogicalPlanMergeTargetSnafu)?
        } else {
            LogicalPlanBuilder::scan(&target_ident, target_table_source, None)
                .context(ex_error::DataFusionLogicalPlanMergeTargetSnafu)?
        };
        let plan = if let Some(target_alias) = target_alias {
            plan.alias(target_alias.name.to_string())
                .context(ex_error::DataFusionLogicalPlanMergeTargetSnafu)?
        } else {
            plan
        };
        let target_plan = DataFrame::new(
            df_session_state.clone(),
            plan.build()
                .context(ex_error::DataFusionLogicalPlanMergeTargetSnafu)?,
        )
        .with_column(TARGET_EXISTS_COLUMN, lit(true))
        .context(ex_error::DataFusionLogicalPlanMergeTargetSnafu)?
        .into_unoptimized_plan();

        let target_schema = target_plan.schema().clone();

        // Create the LogicalPlan for the join

        let sql_planner =
            ExtendedSqlToRel::new(&session_context_provider, ParserOptions::default());

        let schema = build_join_schema(&target_schema, &source_schema, &JoinType::Full)
            .context(ex_error::DataFusionLogicalPlanMergeJoinSnafu)?;

        let on_expr = sql_planner
            .as_ref()
            .sql_to_expr((*on).clone(), &schema, &mut planner_context)
            .context(ex_error::DataFusionLogicalPlanMergeJoinSnafu)?;

        let merge_clause_projection =
            merge_clause_projection(&sql_planner, &schema, &target_schema, clauses)?;

        let join_plan = LogicalPlanBuilder::new(target_plan)
            .join_on(source_plan, JoinType::Full, [on_expr; 1])
            .context(ex_error::DataFusionLogicalPlanMergeJoinSnafu)?
            .project(merge_clause_projection)
            .context(ex_error::DataFusionLogicalPlanMergeJoinSnafu)?
            .build()
            .context(ex_error::DataFusionLogicalPlanMergeJoinSnafu)?;

        let merge_into_plan = MergeIntoCOWSink::new(Arc::new(join_plan), target_table)
            .context(ex_error::DataFusionSnafu)?;

        self.execute_logical_plan(LogicalPlan::Extension(Extension {
            node: Arc::new(merge_into_plan),
        }))
        .await
    }

    #[instrument(name = "UserQuery::create_database", level = "trace", skip(self), err)]
    pub async fn create_database(
        &self,
        db_name: ObjectName,
        if_not_exists: bool,
        external_volume: Option<String>,
    ) -> Result<QueryResult> {
        let catalog_name = object_name_to_string(&db_name);
        if external_volume.is_none() {
            return ex_error::ExternalVolumeRequiredForCreateDatabaseSnafu { name: catalog_name }
                .fail();
        }
        let catalog_exist = self.get_catalog(&catalog_name).is_ok();
        if catalog_exist {
            if if_not_exists {
                return self.created_entity_response();
            }
            return ex_error::ObjectAlreadyExistsSnafu {
                r#type: ExistingObjectType::Database,
                name: catalog_name,
            }
            .fail();
        }
        self.create_catalog(&catalog_name, &external_volume.unwrap_or_default())
            .await?;
        self.created_entity_response()
    }

    #[instrument(name = "UserQuery::create_schema", level = "trace", skip(self), err)]
    pub async fn create_schema(&self, statement: Statement) -> Result<QueryResult> {
        let Statement::CreateSchema {
            schema_name,
            if_not_exists,
        } = statement.clone()
        else {
            return ex_error::OnlyCreateSchemaStatementsSnafu.fail();
        };

        let SchemaName::Simple(schema_name) = schema_name else {
            return ex_error::OnlySimpleSchemaNamesSnafu.fail();
        };

        let ident: MetastoreSchemaIdent = self.resolve_schema_object_name(schema_name.0)?.into();
        let plan = self.sql_statement_to_plan(statement).await?;
        let catalog = self.get_catalog(&ident.database)?;

        let downcast_result = self
            .resolve_iceberg_catalog_or_execute(catalog.clone(), ident.database.clone(), plan)
            .await;
        let iceberg_catalog = match downcast_result {
            IcebergCatalogResult::Catalog(catalog) => catalog,
            IcebergCatalogResult::Result(result) => {
                return result.map(|_| self.created_entity_response())?;
            }
        };

        let schema_exists = iceberg_catalog
            .list_namespaces(None)
            .await
            .context(ex_error::IcebergSnafu)?
            .iter()
            .any(|namespace| namespace.join(".") == ident.schema);

        if schema_exists {
            if if_not_exists {
                return self.created_entity_response();
            }
            return ex_error::ObjectAlreadyExistsSnafu {
                r#type: ExistingObjectType::Schema,
                name: ident.schema,
            }
            .fail();
        }
        let namespace = Namespace::try_new(&[ident.schema.clone()])
            .map_err(|err| DataFusionError::External(Box::new(err)))
            .context(ex_error::DataFusionSnafu)?;
        iceberg_catalog
            .create_namespace(&namespace, None)
            .await
            .context(ex_error::IcebergSnafu)?;
        if let Some(mirror) = Self::get_iceberg_mirror(&catalog) {
            catalog
                .register_schema(
                    &namespace.to_string(),
                    Arc::new(IcebergSchema::new(namespace.clone(), mirror)),
                )
                .context(ex_error::DataFusionSnafu)?;
        }

        self.refresh_catalog_partially(CachedEntity::Schema(ident))
            .await?;

        self.created_entity_response()
    }

    #[allow(clippy::too_many_lines)]
    pub async fn show_query(&self, statement: Statement) -> Result<QueryResult> {
        let query = match statement {
            Statement::ShowDatabases { show_options, .. } => {
                let sql = format!(
                    "SELECT
                        NULL as created_on,
                        database_name as name,
                        'STANDARD' as kind,
                        NULL as database_name,
                        NULL as schema_name
                    FROM {}.information_schema.databases",
                    self.current_database()
                );
                let mut filters = Vec::new();
                if let Some(filter) =
                    build_starts_with_filter(show_options.starts_with, "database_name")
                {
                    filters.push(filter);
                }
                apply_show_filters(sql, &filters)
            }
            Statement::ShowSchemas { show_options, .. } => {
                let reference = self.resolve_show_in_name(show_options.show_in, false);
                let catalog: String = reference
                    .catalog()
                    .map_or_else(|| self.current_database(), ToString::to_string);
                let sql = format!(
                    "SELECT
                        NULL as created_on,
                        schema_name as name,
                        NULL as kind,
                        catalog_name as database_name,
                        NULL as schema_name
                    FROM {catalog}.information_schema.schemata",
                );
                let mut filters = Vec::new();
                if let Some(filter) =
                    build_starts_with_filter(show_options.starts_with, "schema_name")
                {
                    filters.push(filter);
                }
                apply_show_filters(sql, &filters)
            }
            Statement::ShowTables { show_options, .. } => {
                let reference = self.resolve_show_in_name(show_options.show_in, false);
                let catalog: String = reference
                    .catalog()
                    .map_or_else(|| self.current_database(), ToString::to_string);
                let sql = format!(
                    "SELECT
                        NULL as created_on,
                        table_name as name,
                        table_type as kind,
                        table_catalog as database_name,
                        table_schema as schema_name
                    FROM {catalog}.information_schema.tables"
                );
                let mut filters = vec!["table_type = 'TABLE'".to_string()];
                if let Some(filter) =
                    build_starts_with_filter(show_options.starts_with, "table_name")
                {
                    filters.push(filter);
                }
                if let Some(schema) = reference.schema().filter(|s| !s.is_empty()) {
                    filters.push(format!("table_schema = '{schema}'"));
                }
                apply_show_filters(sql, &filters)
            }
            Statement::ShowViews { show_options, .. } => {
                let reference = self.resolve_show_in_name(show_options.show_in, false);
                let catalog: String = reference
                    .catalog()
                    .map_or_else(|| self.current_database(), ToString::to_string);
                let sql = format!(
                    "SELECT
                        NULL as created_on,
                        view_name as name,
                        view_type as kind,
                        view_catalog as database_name,
                        view_schema as schema_name
                    FROM {catalog}.information_schema.views"
                );
                let mut filters = Vec::new();
                if let Some(filter) =
                    build_starts_with_filter(show_options.starts_with, "view_name")
                {
                    filters.push(filter);
                }
                if let Some(schema) = reference.schema().filter(|s| !s.is_empty()) {
                    filters.push(format!("view_schema = '{schema}'"));
                }
                apply_show_filters(sql, &filters)
            }
            Statement::ShowObjects(ShowObjects { show_options, .. }) => {
                let reference = self.resolve_show_in_name(show_options.show_in, false);
                let catalog: String = reference
                    .catalog()
                    .map_or_else(|| self.current_database(), ToString::to_string);
                let sql = format!(
                    "SELECT
                        NULL as created_on,
                        upper(table_name) as name,
                        table_type as kind,
                        upper(table_catalog) as database_name,
                        upper(table_schema) as schema_name,
                        is_iceberg,
                        'N' as is_dynamic
                    FROM {catalog}.information_schema.tables"
                );
                let mut filters = Vec::new();
                if let Some(filter) =
                    build_starts_with_filter(show_options.starts_with, "table_name")
                {
                    filters.push(filter);
                }
                if let Some(schema) = reference.schema().filter(|s| !s.is_empty()) {
                    filters.push(format!("table_schema = '{schema}'"));
                }
                apply_show_filters(sql, &filters)
            }
            Statement::ShowColumns { show_options, .. } => {
                let reference = self.resolve_show_in_name(show_options.show_in.clone(), true);
                let catalog: String = reference
                    .catalog()
                    .map_or_else(|| self.current_database(), ToString::to_string);
                let sql = format!(
                    "SELECT
                        table_name as table_name,
                        table_schema as schema_name,
                        column_name as column_name,
                        data_type as data_type,
                        column_default as default,
                        is_nullable as 'null?',
                        column_type as kind,
                        NULL as expression,
                        table_catalog as database_name,
                        NULL as autoincrement
                    FROM {catalog}.information_schema.columns"
                );
                let mut filters = Vec::new();
                if let Some(filter) =
                    build_starts_with_filter(show_options.starts_with, "column_name")
                {
                    filters.push(filter);
                }
                if let Some(schema) = reference.schema().filter(|s| !s.is_empty()) {
                    filters.push(format!("table_schema = '{schema}'"));
                }
                if show_options.show_in.is_some() {
                    filters.push(format!("table_name = '{}'", reference.table()));
                }
                apply_show_filters(sql, &filters)
            }
            Statement::ShowVariable { variable } => {
                let variable = object_name_to_string(&ObjectName::from(variable));
                format!(
                    "SELECT
                        NULL as created_on,
                        NULL as updated_on,
                        name,
                        value,
                        NULL as type,
                        description as comment
                    FROM {}.information_schema.df_settings
                    WHERE name = '{}'",
                    self.current_database(),
                    variable
                )
            }
            Statement::ShowVariables { filter, .. } => {
                let sql = format!(
                    "SELECT
                        session_id,
                        name AS name,
                        value,
                        type,
                        description as comment,
                        created_on,
                        updated_on,
                    FROM {}.information_schema.df_settings",
                    self.current_database()
                );
                let mut filters = vec!["session_id is NOT NULL".to_string()];
                if let Some(ShowStatementFilter::Like(pattern)) = filter {
                    filters.push(format!("name LIKE '{pattern}'"));
                }
                apply_show_filters(sql, &filters)
            }
            _ => {
                return ex_error::UnsupportedShowStatementSnafu {
                    statement: statement.to_string(),
                }
                .fail();
            }
        };
        Box::pin(self.execute_with_custom_plan(&query)).await
    }

    #[instrument(
        name = "UserQuery::truncate_table",
        level = "trace",
        skip(self),
        err,
        ret
    )]
    pub async fn truncate_table(
        &self,
        table_names: Vec<TruncateTableTarget>,
    ) -> Result<QueryResult> {
        let Some(first_table) = table_names.into_iter().next() else {
            return ex_error::NoTableNamesForTruncateTableSnafu {}.fail();
        };

        let object_name = self.resolve_table_object_name(first_table.name.0)?;
        let mut query = self.session.query(
            format!(
                "CREATE OR REPLACE TABLE {object_name} as (SELECT * FROM {object_name} WHERE FALSE)",
            ),
            QueryContext::default(),
        );
        let res = query.execute().await;

        let table_ident: MetastoreTableIdent = object_name.into();

        self.refresh_catalog_partially(CachedEntity::Table(table_ident))
            .await?;

        res
    }

    #[must_use]
    pub fn resolve_show_in_name(
        &self,
        show_in: Option<ShowStatementIn>,
        table: bool,
    ) -> TableReference {
        let parts: Vec<String> = show_in
            .and_then(|in_clause| in_clause.parent_name)
            .map(|obj| obj.0.into_iter().map(|ident| ident.to_string()).collect())
            .unwrap_or_default();

        let (catalog, schema, table_name) = match parts.as_slice() {
            [one] if table => (self.current_database(), self.current_schema(), one.clone()),
            [one] => (self.current_database(), one.clone(), String::new()),
            [s, t] if table => (self.current_database(), s.clone(), t.clone()),
            [d, s] => (d.clone(), s.clone(), String::new()),
            [d, s, t] => (d.clone(), s.clone(), t.clone()),
            _ => (self.current_database(), String::new(), String::new()),
        };
        TableReference::full(catalog, schema, table_name)
    }

    #[instrument(
        name = "UserQuery::get_custom_logical_plan",
        level = "trace",
        skip(self),
        err,
        ret
    )]
    pub async fn get_custom_logical_plan(&self, query: &str) -> Result<LogicalPlan> {
        let state = self.session.ctx.state();
        let dialect = state.config().options().sql_parser.dialect.as_str();

        // We turn a query to SQL only to turn it back into a statement
        // TODO: revisit this pattern
        let mut statement = state
            .sql_to_statement(query, dialect)
            .context(ex_error::DataFusionSnafu)?;
        self.update_statement_references(&mut statement)?;

        if let DFStatement::Statement(s) = statement.clone() {
            self.sql_statement_to_plan(*s).await
        } else {
            ex_error::OnlySQLStatementsSnafu.fail()
        }
    }

    /// Fully qualifies all table references in the provided SQL statement.
    ///
    /// This function traverses the SQL statement and updates table references to their fully qualified
    /// names (including catalog and schema), based on the current session context and catalog state.
    ///
    /// - Table references that are part of Common Table Expressions (CTEs) are skipped and left as-is.
    /// - Table functions (recognized by the session context) are also skipped and left unresolved.
    /// - All other table references are resolved using `resolve_table_object_name` and updated in-place.
    ///
    /// # Arguments
    /// * `statement` - The SQL statement (`DFStatement`) to update.
    ///
    /// # Errors
    /// Returns an error if table resolution fails for any non-CTE, non-table-function reference.
    pub fn update_statement_references(&self, statement: &mut DFStatement) -> Result<()> {
        let (_tables, ctes) = resolve_table_references(
            statement,
            self.session
                .ctx
                .state()
                .config()
                .options()
                .sql_parser
                .enable_ident_normalization,
        )
        .context(ex_error::DataFusionSnafu)?;
        match statement {
            DFStatement::Statement(stmt) => {
                let cte_names: HashSet<String> = ctes
                    .into_iter()
                    .map(|cte| cte.table().to_string())
                    .collect();

                let _ = visit_relations_mut(stmt, |table_name: &mut ObjectName| {
                    let is_table_func = self
                        .session
                        .ctx
                        .table_function(table_name.to_string().to_ascii_lowercase().as_str())
                        .is_ok();
                    if !cte_names.contains(&table_name.to_string()) && !is_table_func {
                        match self.resolve_table_object_name(table_name.0.clone()) {
                            Ok(resolved_name) => {
                                *table_name = ObjectName::from(resolved_name.0);
                            }
                            Err(e) => return ControlFlow::Break(e),
                        }
                    }
                    ControlFlow::Continue(())
                });
                Ok(())
            }
            // If needed, handle other DFStatement variants like CreateExternalTable similarly
            _ => Ok(()),
        }
    }

    pub async fn sql_statement_to_plan(&self, statement: Statement) -> Result<LogicalPlan> {
        let tables = self
            .table_references_for_statement(
                &DFStatement::Statement(Box::new(statement.clone())),
                &self.session.ctx.state(),
            )
            .await?;
        let ctx_provider = SessionContextProvider {
            state: &self.session.ctx.state(),
            tables,
        };
        let planner =
            ExtendedSqlToRel::new(&ctx_provider, self.session.ctx.state().get_parser_options());
        planner
            .sql_statement_to_plan(statement)
            .context(ex_error::DataFusionSnafu)
    }

    async fn execute_sql(&self, query: &str) -> Result<QueryResult> {
        let session = self.session.clone();
        let query_id = self.query_context.query_id;
        let query = query.to_string();
        let stream = self
            .session
            .executor
            .spawn(async move {
                let df = session
                    .ctx
                    .sql(&query)
                    .await
                    .context(ex_error::DataFusionSnafu)?;
                let mut schema = df.schema().as_arrow().clone();
                let records = df.collect().await.context(ex_error::DataFusionSnafu)?;
                if !records.is_empty() {
                    schema = records[0].schema().as_ref().clone();
                }
                Ok::<QueryResult, Error>(QueryResult::new(records, Arc::new(schema), query_id))
            })
            .await
            .context(ex_error::JobSnafu)??;
        Ok(stream)
    }

    async fn execute_logical_plan(&self, plan: LogicalPlan) -> Result<QueryResult> {
        let session = self.session.clone();
        let query_id = self.query_context.query_id;
        let stream = self
            .session
            .executor
            .spawn(async move {
                let mut schema = plan.schema().as_arrow().clone();
                let records = session
                    .ctx
                    .execute_logical_plan(plan)
                    .await
                    .context(ex_error::DataFusionSnafu)?
                    .collect()
                    .await
                    .context(ex_error::DataFusionSnafu)?;
                if !records.is_empty() {
                    schema = records[0].schema().as_ref().clone();
                }
                Ok::<QueryResult, Error>(QueryResult::new(records, Arc::new(schema), query_id))
            })
            .await
            .context(ex_error::JobSnafu)??;
        Ok(stream)
    }

    #[instrument(
        name = "UserQuery::execute_with_custom_plan",
        level = "trace",
        skip(self),
        err
    )]
    pub async fn execute_with_custom_plan(&self, query: &str) -> Result<QueryResult> {
        let mut plan = self.get_custom_logical_plan(query).await?;
        plan = self
            .session_context_expr_rewriter()
            .rewrite_plan(&plan)
            .context(ex_error::DataFusionSnafu)?;
        self.execute_logical_plan(plan).await
    }

    #[allow(clippy::unwrap_used)]
    async fn table_references_for_statement(
        &self,
        statement: &DFStatement,
        state: &SessionState,
    ) -> Result<HashMap<ResolvedTableReference, Arc<dyn TableSource>>> {
        let mut tables = HashMap::new();

        let references = state
            .resolve_table_references(statement)
            .context(ex_error::DataFusionSnafu)?;
        for reference in references {
            let resolved = self.resolve_table_ref(reference);
            if let Entry::Vacant(v) = tables.entry(resolved.clone()) {
                if let Ok(schema) = self.schema_for_ref(resolved.clone()) {
                    if let Some(table) = schema
                        .table(&resolved.table)
                        .await
                        .context(ex_error::DataFusionSnafu)?
                    {
                        v.insert(provider_as_source(table));
                    }
                }
            }
        }
        Ok(tables)
    }

    /// Resolves a [`TableReference`] to a [`ResolvedTableReference`]
    /// using the default catalog and schema.
    pub fn resolve_table_ref(
        &self,
        table_ref: impl Into<TableReference>,
    ) -> ResolvedTableReference {
        table_ref
            .into()
            .resolve(&self.current_database(), &self.current_schema())
    }

    #[must_use]
    pub fn resolve_schema_ref(&self, schema: SchemaReference) -> ResolvedTableReference {
        match schema {
            SchemaReference::Bare { schema } => ResolvedTableReference {
                catalog: Arc::from(self.current_database()),
                schema,
                table: Arc::from(""),
            },
            SchemaReference::Full { catalog, schema } => ResolvedTableReference {
                catalog,
                schema,
                table: Arc::from(""),
            },
        }
    }

    pub fn schema_for_ref(
        &self,
        table_ref: impl Into<TableReference>,
    ) -> datafusion_common::Result<Arc<dyn SchemaProvider>> {
        let resolved_ref = self.session.ctx.state().resolve_table_ref(table_ref);
        if self.session.ctx.state().config().information_schema()
            && *resolved_ref.schema == *INFORMATION_SCHEMA
        {
            return Ok(Arc::new(InformationSchemaProvider::new(
                Arc::clone(self.session.ctx.state().catalog_list()),
                resolved_ref.catalog,
            )));
        }
        self.session
            .ctx
            .state()
            .catalog_list()
            .catalog(&resolved_ref.catalog)
            .ok_or_else(|| {
                plan_datafusion_err!("failed to resolve catalog: {}", resolved_ref.catalog)
            })?
            .schema(&resolved_ref.schema)
            .ok_or_else(|| {
                plan_datafusion_err!("failed to resolve schema: {}", resolved_ref.schema)
            })
    }

    fn convert_batches_to_exprs(batches: Vec<RecordBatch>) -> Vec<sqlparser::ast::ExprWithAlias> {
        let mut exprs = Vec::new();
        for batch in batches {
            if batch.num_columns() > 0 {
                let column = batch.column(0);
                for row_idx in 0..batch.num_rows() {
                    if !column.is_null(row_idx) {
                        if let Ok(scalar_value) = ScalarValue::try_from_array(column, row_idx) {
                            let expr = if batch.schema().fields()[0].data_type().is_numeric() {
                                Expr::Value(
                                    Value::Number(scalar_value.to_string(), false)
                                        .with_empty_span(),
                                )
                            } else {
                                Expr::Value(
                                    Value::SingleQuotedString(scalar_value.to_string())
                                        .with_empty_span(),
                                )
                            };

                            exprs.push(sqlparser::ast::ExprWithAlias { expr, alias: None });
                        }
                    }
                }
            }
        }
        exprs
    }

    // This function is used to update the table references in the query
    // It executes the subquery and convert the result to a list of string expressions
    // It then replaces the pivot value source with the list of string expressions
    async fn replace_pivot_subquery_with_list(
        &self,
        table: &TableFactor,
        value_column: &[Ident],
        value_source: &mut PivotValueSource,
    ) {
        match value_source {
            PivotValueSource::Any(order_by_expr) => {
                let col = value_column
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(".");
                let mut query = format!("SELECT DISTINCT {col} FROM {table} ");

                if !order_by_expr.is_empty() {
                    let order_by_clause = order_by_expr
                        .iter()
                        .map(|expr| {
                            let direction = match expr.options.asc {
                                Some(true) => " ASC",
                                Some(false) => " DESC",
                                None => "",
                            };

                            let nulls = match expr.options.nulls_first {
                                Some(true) => " NULLS FIRST",
                                Some(false) => " NULLS LAST",
                                None => "",
                            };

                            format!("{}{}{}", expr.expr, direction, nulls)
                        })
                        .collect::<Vec<_>>()
                        .join(", ");

                    let _ = write!(query, "ORDER BY {order_by_clause}");
                }

                let result = self.execute_with_custom_plan(&query).await;
                if let Ok(batches) = result {
                    *value_source =
                        PivotValueSource::List(Self::convert_batches_to_exprs(batches.records));
                }
            }
            PivotValueSource::Subquery(subquery) => {
                let subquery_sql = subquery.to_string();

                let result = self.execute_with_custom_plan(&subquery_sql).await;

                if let Ok(batches) = result {
                    *value_source =
                        PivotValueSource::List(Self::convert_batches_to_exprs(batches.records));
                }
            }
            PivotValueSource::List(_) => {
                // Do nothing
            }
        }
    }

    /// This method traverses query including set expressions and updates it when needed.
    fn traverse_and_update_query<'a>(
        &'a self,
        query: &'a mut Query,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            fn process_set_expr<'a>(
                this: &'a UserQuery,
                set_expr: &'a mut sqlparser::ast::SetExpr,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
                Box::pin(async move {
                    match set_expr {
                        sqlparser::ast::SetExpr::Select(select) => {
                            for table_with_joins in &mut select.from {
                                if let TableFactor::Pivot {
                                    table,
                                    value_column,
                                    value_source,
                                    ..
                                } = &mut table_with_joins.relation
                                {
                                    this.replace_pivot_subquery_with_list(
                                        table,
                                        value_column,
                                        value_source,
                                    )
                                    .await;
                                }
                            }
                        }

                        sqlparser::ast::SetExpr::Query(inner_query) => {
                            this.traverse_and_update_query(inner_query).await;
                        }
                        sqlparser::ast::SetExpr::SetOperation { left, right, .. } => {
                            process_set_expr(this, left).await;
                            process_set_expr(this, right).await;
                        }
                        _ => {}
                    }
                })
            }

            process_set_expr(self, &mut query.body).await;

            if let Some(with) = &mut query.with {
                for cte in &mut with.cte_tables {
                    self.traverse_and_update_query(&mut cte.query).await;
                }
            }
        })
    }

    #[must_use]
    pub fn get_table_path(&self, statement: &DFStatement) -> Option<TableReference> {
        let empty = String::new;
        let references = self
            .session
            .ctx
            .state()
            .resolve_table_references(statement)
            .ok()?;

        match statement.clone() {
            DFStatement::Statement(s) => match *s {
                Statement::CopyIntoSnowflake { into, .. } => {
                    Some(TableReference::parse_str(&into.to_string()))
                }
                Statement::Drop { names, .. } => {
                    Some(TableReference::parse_str(&names[0].to_string()))
                }
                Statement::CreateSchema {
                    schema_name: SchemaName::Simple(name),
                    ..
                } => {
                    if name.0.len() == 2 {
                        // Extract the database and schema names
                        let db_name = match &name.0[0] {
                            ObjectNamePart::Identifier(ident) => ident.value.clone(),
                        };

                        let schema_name = match &name.0[1] {
                            ObjectNamePart::Identifier(ident) => ident.value.clone(),
                        };

                        Some(TableReference::full(db_name, schema_name, empty()))
                    } else {
                        // Extract just the schema name
                        let schema_name = match &name.0[0] {
                            ObjectNamePart::Identifier(ident) => ident.value.clone(),
                        };

                        Some(TableReference::full(empty(), schema_name, empty()))
                    }
                }
                _ => references.first().cloned(),
            },
            _ => references.first().cloned(),
        }
    }

    // Fill in the database and schema if they are missing
    // and normalize the identifiers for ObjectNamePart
    #[instrument(
        name = "UserQuery::resolve_table_object_name",
        level = "trace",
        skip(self),
        err
    )]
    pub fn resolve_table_object_name(
        &self,
        mut table_ident: Vec<ObjectNamePart>,
    ) -> Result<NormalizedIdent> {
        match table_ident.len() {
            1 => {
                table_ident.insert(
                    0,
                    ObjectNamePart::Identifier(Ident::new(self.current_database())),
                );
                table_ident.insert(
                    1,
                    ObjectNamePart::Identifier(Ident::new(self.current_schema())),
                );
            }
            2 => {
                table_ident.insert(
                    0,
                    ObjectNamePart::Identifier(Ident::new(self.current_database())),
                );
            }
            3 => {}
            _ => {
                return ex_error::InvalidTableIdentifierSnafu {
                    ident: table_ident
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join("."),
                }
                .fail();
            }
        }

        let normalized_idents = table_ident
            .into_iter()
            .map(|part| match part {
                ObjectNamePart::Identifier(ident) => self.normalize_ident(ident),
            })
            .collect();
        Ok(NormalizedIdent(normalized_idents))
    }

    // Fill in the database if missing and normalize the identifiers for ObjectNamePart
    pub fn resolve_schema_object_name(
        &self,
        mut schema_ident: Vec<ObjectNamePart>,
    ) -> Result<NormalizedIdent> {
        match schema_ident.len() {
            1 => {
                schema_ident.insert(
                    0,
                    ObjectNamePart::Identifier(Ident::new(self.current_database())),
                );
            }
            2 => {}
            _ => {
                return ex_error::InvalidSchemaIdentifierSnafu {
                    ident: schema_ident
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join("."),
                }
                .fail();
            }
        }
        let normalized_idents = schema_ident
            .into_iter()
            .map(|part| match part {
                ObjectNamePart::Identifier(ident) => self.normalize_ident(ident),
            })
            .collect();
        Ok(NormalizedIdent(normalized_idents))
    }

    fn normalize_ident(&self, ident: Ident) -> Ident {
        match ident.quote_style {
            Some(qs) => Ident::with_quote(qs, ident.value),
            None => Ident::new(self.session.ident_normalizer.normalize(ident)),
        }
    }

    pub fn created_entity_response(&self) -> Result<QueryResult> {
        let schema = Arc::new(ArrowSchema::new(vec![Field::new(
            "count",
            DataType::Int64,
            false,
        )]));
        Ok(QueryResult::new(
            vec![
                RecordBatch::try_new(schema.clone(), vec![Arc::new(Int64Array::from(vec![0]))])
                    .context(ex_error::ArrowSnafu)?,
            ],
            schema,
            self.query_context.query_id,
        ))
    }

    pub fn status_response(&self) -> Result<QueryResult> {
        let schema = Arc::new(ArrowSchema::new(vec![Field::new(
            "status",
            DataType::Utf8,
            false,
        )]));
        Ok(QueryResult::new(
            vec![RecordBatch::new_empty(schema.clone())],
            schema,
            self.query_context.query_id,
        ))
    }

    /// Retrieves and configures an Iceberg table provider with enhanced schema.
    ///
    /// This function resolves a table identifier to a `DataFusionTable` with an enhanced schema
    /// that includes metadata columns for tracking data and manifest file paths. It performs
    /// the necessary downcasting from the cached table provider to ensure the table is an
    /// Iceberg table.
    ///
    /// # Arguments
    ///
    /// * `target_ident` - The normalized table identifier to resolve
    /// * `config` - Optional `DataFusion` table configuration. If provided, overrides the table's
    ///   default configuration. Commonly used to enable metadata columns.
    ///
    /// # Returns
    ///
    /// Returns a configured `DataFusionTable` with an enhanced schema that includes data file
    /// path and manifest file path columns.
    ///
    /// # Errors
    ///
    /// * `DataFusionSnafu` - If the table provider cannot be retrieved from the session context
    /// * `CatalogDownCastSnafu` - If the cached table cannot be downcast to `CachingTable`
    /// * `MergeTargetMustBeIcebergTableSnafu` - If the table provider is not an Iceberg table
    async fn get_iceberg_table_provider(
        &self,
        target_ident: &NormalizedIdent,
        config: Option<datafusion_iceberg::table::DataFusionTableConfig>,
    ) -> Result<DataFusionTable> {
        let target_provider = {
            let target_cache = self
                .session
                .ctx
                .table_provider(target_ident)
                .await
                .context(ex_error::DataFusionSnafu)?;

            target_cache
                .as_any()
                .downcast_ref::<CachingTable>()
                .ok_or_else(|| {
                    ex_error::CatalogDownCastSnafu {
                        catalog: "DataFusionTable".to_string(),
                    }
                    .build()
                })?
                .table
                .clone()
        };

        let target_ref = target_provider
            .as_any()
            .downcast_ref::<DataFusionTable>()
            .ok_or_else(|| ex_error::MergeTargetMustBeIcebergTableSnafu.build())?;

        //TODO check if enabl options are set
        let schema = if config.is_some() {
            Arc::new(build_target_schema(target_ref.schema.as_ref()))
        } else {
            target_ref.schema.clone()
        };
        Ok(DataFusionTable {
            config,
            schema,
            ..target_ref.clone()
        })
    }
}

/// Builds a target schema with metadata columns added.
///
/// This function takes a base schema and adds data file path and manifest file path columns
/// to create a schema suitable for merge operations that require metadata tracking.
fn build_target_schema(base_schema: &ArrowSchema) -> ArrowSchema {
    let mut builder = SchemaBuilder::from(base_schema);
    builder.push(Field::new(DATA_FILE_PATH_COLUMN, DataType::Utf8, true));
    builder.push(Field::new(MANIFEST_FILE_PATH_COLUMN, DataType::Utf8, true));
    builder.finish()
}

/// Converts merge clauses into projection expressions for copy-on-write operations.
///
/// This function processes MERGE statement clauses (UPDATE/INSERT) and generates `DataFusion`
/// projection expressions that compute the new table state. Each column gets an expression
/// that determines its new value based on the merge operation type:
/// - Operation code 3: Matched rows (UPDATE)
/// - Operation code 2: Not matched rows (INSERT)
///
/// # Arguments
/// * `sql_planner` - SQL to logical expression planner
/// * `schema` - Target table schema
/// * `merge_clause` - Vector of merge clauses to process
///
/// # Returns
/// Vector of expressions for each column in the target schema
pub fn merge_clause_projection<S: ContextProvider>(
    sql_planner: &ExtendedSqlToRel<'_, S>,
    schema: &DFSchema,
    target_schema: &DFSchema,
    merge_clause: Vec<MergeClause>,
) -> Result<Vec<logical_expr::Expr>> {
    let mut updates: HashMap<String, Vec<(logical_expr::Expr, logical_expr::Expr)>> =
        HashMap::new();
    let mut inserts: HashMap<String, Vec<(logical_expr::Expr, logical_expr::Expr)>> =
        HashMap::new();

    let mut planner_context = datafusion::sql::planner::PlannerContext::new();

    for merge_clause in merge_clause {
        let op = merge_clause_expression(&merge_clause)?;
        let op = if let Some(predicate) = merge_clause.predicate {
            let predicate = sql_planner
                .as_ref()
                .sql_to_expr(predicate, schema, &mut planner_context)
                .context(ex_error::DataFusionSnafu)?;
            and(op, predicate)
        } else {
            op
        };
        match merge_clause.action {
            MergeAction::Update { assignments } => {
                for assignment in assignments {
                    match assignment.target {
                        AssignmentTarget::ColumnName(mut column) => {
                            let column_name = column
                                .0
                                .pop()
                                .ok_or_else(|| {
                                    InvalidColumnIdentifierSnafu {
                                        ident: column.to_string(),
                                    }
                                    .build()
                                })?
                                .to_string();
                            let expr = sql_planner
                                .as_ref()
                                .sql_to_expr(assignment.value, schema, &mut planner_context)
                                .context(ex_error::DataFusionSnafu)?;
                            updates
                                .entry(column_name)
                                .and_modify(|x| x.push((op.clone(), expr.clone())))
                                .or_insert_with(|| vec![(op.clone(), expr)]);
                        }
                        AssignmentTarget::Tuple(_) => todo!(),
                    }
                }
            }
            MergeAction::Insert(insert) => {
                let MergeInsertKind::Values(values) = insert.kind else {
                    return Err(ex_error::OnlyMergeStatementsSnafu.build());
                };
                if values.rows.len() != 1 {
                    return Err(ex_error::MergeInsertOnlyOneRowSnafu.build());
                }
                let mut all_columns: HashSet<String> = target_schema
                    .iter()
                    .map(|x| x.1.name().to_string())
                    .collect();
                for (column, value) in insert.columns.iter().zip(
                    values
                        .rows
                        .into_iter()
                        .next()
                        .ok_or_else(|| ex_error::MergeInsertOnlyOneRowSnafu.build())?
                        .into_iter(),
                ) {
                    let column_name = column.value.clone();
                    let expr = sql_planner
                        .as_ref()
                        .sql_to_expr(value, schema, &mut planner_context)
                        .context(ex_error::DataFusionSnafu)?;
                    all_columns.remove(&column_name);
                    inserts
                        .entry(column_name)
                        .and_modify(|x| x.push((op.clone(), expr.clone())))
                        .or_insert_with(|| vec![(op.clone(), expr)]);
                }
                for column in all_columns {
                    inserts.insert(column, vec![(op.clone(), lit(ScalarValue::Null))]);
                }
            }
            MergeAction::Delete => (),
        }
    }
    let exprs = collect_merge_clause_expressions(target_schema, updates, inserts)?;
    Ok(exprs)
}

/// Builds projection expressions for MERGE statement by combining UPDATE and INSERT operations.
///
/// This function creates a CASE expression for each column in the target schema that handles
/// both UPDATE and INSERT operations from MERGE clauses. For each column, it builds a conditional
/// expression that applies the appropriate transformation based on the merge operation type.
///
/// # Arguments
/// * `target_schema` - Schema of the target table
/// * `updates` - Map of column names to their UPDATE expressions with conditions
/// * `inserts` - Map of column names to their INSERT expressions with conditions
///
/// # Returns
/// Vector of expressions for each column, plus the `SOURCE_EXISTS_COLUMN` for tracking
fn collect_merge_clause_expressions(
    target_schema: &DFSchema,
    mut updates: HashMap<String, Vec<(DFExpr, DFExpr)>>,
    mut inserts: HashMap<String, Vec<(DFExpr, DFExpr)>>,
) -> Result<Vec<DFExpr>> {
    let mut exprs: Vec<datafusion_expr::Expr> = target_schema
        .iter()
        .map(|(table_reference, field)| {
            let name = table_reference
                .map(|x| x.to_string() + ".")
                .unwrap_or_default()
                + field.name();
            let updates = updates.remove(field.name());
            let insert = inserts.remove(field.name());

            // If there is no update or insert, do nothing
            if updates.is_none() && insert.is_none() {
                return Ok(col(name));
            }

            let case_expr = match (updates, insert) {
                (Some(updates), Some(inserts)) => {
                    let builder_opt = updates.into_iter().chain(inserts.into_iter()).fold(
                        None::<CaseBuilder>,
                        |acc, (w, t)| {
                            if let Some(mut acc) = acc {
                                Some(acc.when(w, t))
                            } else {
                                Some(when(w, t))
                            }
                        },
                    );
                    if let Some(mut builder) = builder_opt {
                        builder.otherwise(col(name))?
                    } else {
                        col(name)
                    }
                }
                (Some(x), None) | (None, Some(x)) => {
                    let builder_opt = x.into_iter().fold(None::<CaseBuilder>, |acc, (w, t)| {
                        if let Some(mut acc) = acc {
                            Some(acc.when(w, t))
                        } else {
                            Some(when(w, t))
                        }
                    });
                    if let Some(mut builder) = builder_opt {
                        builder.otherwise(col(name))?
                    } else {
                        col(name)
                    }
                }
                (None, None) => col(name),
            }
            .alias(field.name().clone());

            Ok::<_, DataFusionError>(case_expr)
        })
        .collect::<StdResult<_, DataFusionError>>()
        .context(ex_error::DataFusionSnafu)?;
    exprs.push(col(SOURCE_EXISTS_COLUMN));
    Ok(exprs)
}

/// Builds a `DataFusion` expression for filtering rows based on MERGE clause conditions.
///
/// This function translates MERGE clause semantics into boolean expressions that determine
/// which rows should be processed by each clause type:
///
/// - `Matched`: Rows that exist in both source and target tables
/// - `NotMatched`: Rows that don't exist in either source or target
///
/// The expressions use special columns (`TARGET_EXISTS_COLUMN` and `SOURCE_EXISTS_COLUMN`)
/// that indicate row presence in respective tables during the merge operation.
fn merge_clause_expression(merge_clause: &MergeClause) -> Result<DFExpr> {
    let expr = match merge_clause.clause_kind {
        MergeClauseKind::Matched => Ok(and(col(TARGET_EXISTS_COLUMN), col(SOURCE_EXISTS_COLUMN))),
        MergeClauseKind::NotMatched => Ok(or(
            is_null(col(TARGET_EXISTS_COLUMN)),
            is_null(col(TARGET_EXISTS_COLUMN)),
        )),
        MergeClauseKind::NotMatchedByTarget => Ok(is_null(col(TARGET_EXISTS_COLUMN))),
        MergeClauseKind::NotMatchedBySource => {
            return Err(ex_error::NotMatchedBySourceNotSupportedSnafu.build());
        }
    }?;
    Ok(expr)
}

/// Constructs a filter expression for the target table based on the partition column bounds
/// of the source table.
///
/// This function analyzes the partition structure of the source table and creates
/// a `DataFusion` expression that filters data based on the minimum and maximum
/// values of the partition columns. This is used for query optimization to prune
/// partitions that don't contain relevant data.
///
/// # Arguments
///
/// * `table` - The `DataFusion` table wrapper for the Iceberg table
///
/// # Returns
///
/// * `Ok(Some(expr))` - A filter expression combining bounds for all partition columns
/// * `Ok(None)` - If no snapshot exists, no partition bounds available, or no partition fields
/// * `Err` - If there's an error accessing table metadata or building the expression
///
/// # Behavior
///
/// The function creates range conditions (column >= min AND column <= max) for each
/// partition column and combines them with AND operators. Only works with Iceberg tables;
/// returns an error for other table types.
async fn target_filter_expression(
    table: &DataFusionTable,
) -> Result<Option<datafusion_expr::Expr>> {
    let lock = table.tabular.read().await;
    let table = match &*lock {
        Tabular::Table(table) => Ok(table),
        _ => MergeSourceNotSupportedSnafu.fail(),
    }?;
    let object_store = table.object_store();
    let table_metadata = table.metadata();
    let Some(current_snapshot) = table_metadata
        .current_snapshot(None)
        .map_err(IcebergError::from)
        .context(ex_error::IcebergSnafu)?
    else {
        return Ok(None);
    };
    let Some(partition_column_bounds) =
        partition_column_bounds(current_snapshot, table_metadata, object_store).await?
    else {
        return Ok(None);
    };
    let partition_fields = table
        .metadata()
        .partition_fields(*current_snapshot.snapshot_id())
        .map_err(IcebergError::from)
        .context(ex_error::IcebergSnafu)?;
    let expr = partition_fields
        .iter()
        .zip(partition_column_bounds.into_iter())
        .fold(None, |acc, (column, [min, max])| {
            let column_expr = col(column.source_name());
            let expr = and(
                datafusion_expr::Expr::BinaryExpr(BinaryExpr::new(
                    Box::new(column_expr.clone()),
                    Operator::GtEq,
                    Box::new(min),
                )),
                datafusion_expr::Expr::BinaryExpr(BinaryExpr::new(
                    Box::new(column_expr),
                    Operator::LtEq,
                    Box::new(max),
                )),
            );
            if let Some(acc) = acc {
                Some(and(acc, expr))
            } else {
                Some(expr)
            }
        });
    Ok(expr)
}

/// Retrieves partition column bounds from an Iceberg table snapshot.
///
/// This function extracts the minimum and maximum bounds for partition columns
/// from the given snapshot and converts them into `DataFusion` expressions for
/// query optimization.
///
/// # Arguments
///
/// * `current_snapshot` - The Iceberg table snapshot to extract bounds from
/// * `table_metadata` - Metadata about the table structure and partitioning
/// * `object_store` - Object store for accessing partition metadata
///
/// # Returns
///
/// * `Ok(Some(bounds))` - Vector of min/max expression pairs for each partition column
/// * `Ok(None)` - If no partition bounds are available or bounds are empty
/// * `Err` - If there's an error accessing snapshot or converting values
async fn partition_column_bounds(
    current_snapshot: &Snapshot,
    table_metadata: &TableMetadata,
    object_store: Arc<dyn ObjectStore>,
) -> Result<Option<Vec<[datafusion_expr::Expr; 2]>>> {
    let Some(bounds) = snapshot_partition_bounds(current_snapshot, table_metadata, object_store)
        .await
        .context(ex_error::IcebergSnafu)?
    else {
        return Ok(None);
    };
    let bounds = bounds
        .min
        .iter()
        .zip(bounds.max.iter())
        .map(|(min, max)| Ok([value_to_literal(min)?, value_to_literal(max)?]))
        .collect::<Result<Vec<_>>>()?;
    if bounds.is_empty() {
        Ok(None)
    } else {
        Ok(Some(bounds))
    }
}

/// Converts an `IcebergValue` to `DataFusion`on literal expression.
///
/// # Arguments
/// * `value` - The `IcebergValue` to convert
///
/// # Returns
/// A `DataFusion` expression representing the literal value, or an error if the value type is not supported.
///
/// # Supported Types
/// * All primitive types: Boolean, Int, `LongInt`, Float, Double, Date, Time, Timestamp, `TimestampTZ`, String, UUID, Fixed, Binary, Decimal
/// * Complex types (Struct, List, Map) are not currently supported
fn value_to_literal(value: &IcebergValue) -> Result<datafusion_expr::Expr> {
    match value {
        IcebergValue::Boolean(b) => Ok(lit(*b)),
        IcebergValue::Int(i) => Ok(lit(*i)),
        IcebergValue::LongInt(l) => Ok(lit(*l)),
        IcebergValue::Float(f) => Ok(lit(f.0)),
        IcebergValue::Double(d) => Ok(lit(d.0)),
        IcebergValue::Date(d) => Ok(lit(*d)),
        IcebergValue::Time(t) => Ok(lit(*t)),
        IcebergValue::Timestamp(ts) | IcebergValue::TimestampTZ(ts) => Ok(lit(*ts)),
        IcebergValue::String(s) => Ok(lit(s.as_str())),
        IcebergValue::UUID(u) => Ok(lit(u.to_string())),
        IcebergValue::Fixed(_, data) | IcebergValue::Binary(data) => Ok(lit(data.clone())),
        IcebergValue::Decimal(d) => Ok(lit(d.to_string())),
        IcebergValue::Struct(_) | IcebergValue::List(_) | IcebergValue::Map(_) => {
            ex_error::UnsupportedIcebergValueTypeSnafu {
                value_type: format!("{value:?}"),
            }
            .fail()
        }
    }
}

fn build_starts_with_filter(starts_with: Option<Value>, column_name: &str) -> Option<String> {
    if let Some(Value::SingleQuotedString(prefix)) = starts_with {
        let escaped = prefix.replace('\'', "''");
        Some(format!("{column_name} LIKE '{escaped}%'"))
    } else {
        None
    }
}

fn apply_show_filters(sql: String, filters: &[String]) -> String {
    if filters.is_empty() {
        sql
    } else {
        format!("{} WHERE {}", sql, filters.join(" AND "))
    }
}

pub fn cast_input_to_target_schema(
    input: Arc<LogicalPlan>,
    target_schema: &SchemaRef,
) -> Result<Arc<LogicalPlan>> {
    let input_schema = input.schema().as_arrow();
    let mut projections: Vec<DFExpr> = Vec::with_capacity(target_schema.fields().len());

    for field in target_schema.fields() {
        let name = field.name();
        let data_type = field.data_type();
        let input_field = input_schema
            .field_with_name(name)
            .context(ex_error::ArrowSnafu)?;
        if input_field.data_type() == data_type {
            projections.push(col(name));
        } else {
            projections.push(DFExpr::TryCast(TryCast::new(
                Box::new(col(name)),
                data_type.clone(),
            )));
        }
    }
    let projection = Projection::try_new(projections, input).context(ex_error::DataFusionSnafu)?;
    Ok(Arc::new(LogicalPlan::Projection(projection)))
}
