use crate::session::UserSession;
use std::collections::HashMap;

use crate::models::QueryContext;
use crate::running_queries::RunningQueriesRegistry;
use crate::service::CoreExecutionService;
use crate::utils::Config;
use catalog_metastore::InMemoryMetastore;
use catalog_metastore::{
    Database as MetastoreDatabase, Schema as MetastoreSchema, SchemaIdent as MetastoreSchemaIdent,
    Volume as MetastoreVolume,
};
use catalog_metastore::{Metastore, VolumeType};
use datafusion::sql::parser::DFParser;
use functions::session_params::SessionProperty;
use std::sync::Arc;

#[allow(clippy::unwrap_used)]
#[tokio::test]
async fn test_update_all_table_names_visitor() {
    let args = vec![
        ("select * from foo", "SELECT * FROM embucket.new_schema.foo"),
        (
            "insert into foo (id) values (5)",
            "INSERT INTO embucket.new_schema.foo (id) VALUES (5)",
        ),
        (
            "insert into foo select * from bar",
            "INSERT INTO embucket.new_schema.foo SELECT * FROM embucket.new_schema.bar",
        ),
        (
            "insert into foo select * from bar where id = 1",
            "INSERT INTO embucket.new_schema.foo SELECT * FROM embucket.new_schema.bar WHERE id = 1",
        ),
        (
            "select * from foo join bar on foo.id = bar.id",
            "SELECT * FROM embucket.new_schema.foo JOIN embucket.new_schema.bar ON foo.id = bar.id",
        ),
        (
            "select * from foo where id = 1",
            "SELECT * FROM embucket.new_schema.foo WHERE id = 1",
        ),
        (
            "select count(*) from foo",
            "SELECT count(*) FROM embucket.new_schema.foo",
        ),
        (
            "WITH sales_data AS (SELECT * FROM foo) SELECT * FROM sales_data",
            "WITH sales_data AS (SELECT * FROM embucket.new_schema.foo) SELECT * FROM sales_data",
        ),
        // Skip table functions
        // (
        //     "select * from result_scan('1')",
        //     "SELECT * FROM result_scan('1')",
        // ),
        (
            "SELECT * from flatten('[1,77]','',false,false,'both')",
            "SELECT * FROM flatten('[1,77]', '', false, false, 'both')",
        ),
    ];

    let session = create_df_session().await;
    let mut params = HashMap::new();
    params.insert(
        "schema".to_string(),
        SessionProperty::from_str_value("schema".to_string(), "new_schema".to_string(), None),
    );
    session.set_session_variable(true, params).unwrap();
    let query = session.query("", QueryContext::default());
    for (init, exp) in args {
        let statement = DFParser::parse_sql(init).unwrap().pop_front();
        if let Some(mut s) = statement {
            query.update_statement_references(&mut s).unwrap();
            assert_eq!(s.to_string(), exp);
        }
    }
}

const TABLE_SETUP: &str = include_str!(r"./table_setup.sql");

#[allow(clippy::unwrap_used, clippy::expect_used)]
pub async fn create_df_session() -> Arc<UserSession> {
    let metastore = Arc::new(InMemoryMetastore::new());
    let running_queries = Arc::new(RunningQueriesRegistry::new());
    metastore
        .create_volume(
            &"test_volume".to_string(),
            MetastoreVolume::new("test_volume".to_string(), VolumeType::Memory),
        )
        .await
        .expect("Failed to create volume");
    metastore
        .create_database(
            &"embucket".to_string(),
            MetastoreDatabase {
                ident: "embucket".to_string(),
                properties: None,
                volume: "test_volume".to_string(),
            },
        )
        .await
        .expect("Failed to create database");
    let schema_ident = MetastoreSchemaIdent {
        database: "embucket".to_string(),
        schema: "public".to_string(),
    };

    metastore
        .create_schema(
            &schema_ident.clone(),
            MetastoreSchema {
                ident: schema_ident,
                properties: None,
            },
        )
        .await
        .expect("Failed to create schema");
    let config = Arc::new(Config::default());
    let catalog_list = CoreExecutionService::catalog_list(metastore.clone())
        .await
        .expect("Failed to create catalog list");
    let runtime_env = CoreExecutionService::runtime_env(&config, catalog_list.clone())
        .expect("Failed to create runtime env");

    let user_session = Arc::new(
        UserSession::new(
            metastore,
            running_queries, // queries aborting will not work, unless its properly used (as in ExecutionService)
            Arc::new(Config::default()),
            catalog_list,
            runtime_env,
        )
        .expect("Failed to create user session"),
    );

    for q in TABLE_SETUP.split(';') {
        if !q.is_empty() {
            let mut exec = user_session.query(q, QueryContext::default());
            exec.execute().await.unwrap();
        }
    }
    user_session
}

#[macro_export]
macro_rules! test_query {
    (
        $test_fn_name:ident,
        $query:expr
        $(, setup_queries =[$($setup_queries:expr),* $(,)?])?
        $(, sort_all = $sort_all:expr)?
        $(, exclude_columns = [$($excluded:expr),* $(,)?])?
        $(, snapshot_path = $user_snapshot_path:expr)?
        $(, snowflake_error = $snowflake_error:expr)?
    ) => {
        paste::paste! {
            #[tokio::test]
            async fn [< query_ $test_fn_name >]() {
                let ctx = $crate::tests::query::create_df_session().await;

                // Execute all setup queries (if provided) to set up the session context
                $(
                    $(
                        {
                            let mut q = ctx.query($setup_queries, $crate::models::QueryContext::default());
                            q.execute().await.unwrap();
                        }
                    )*
                )?

                let mut query = ctx.query($query, $crate::models::QueryContext::default().with_ip_address("test_ip".to_string()));
                let res = query.execute().await;
                let sort_all = false $(|| $sort_all)?;
                let excluded_columns: std::collections::HashSet<&str> = std::collections::HashSet::from([
                    $($($excluded),*)?
                ]);
                let snowflake_error = false $(|| $snowflake_error)?;
                let mut settings = insta::Settings::new();
                settings.set_description(stringify!($query));
                settings.set_omit_expression(true);
                settings.set_prepend_module_to_snapshot(false);
                settings.set_snapshot_path(concat!("snapshots", "/") $(.to_owned() + $user_snapshot_path)?);
                settings.add_filter(r"/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\.parquet", "/[UUID].parquet");
                settings.add_filter(r"data/[0-9a-fA-F]{4,8}/", "data/[HEX]/");
                settings.add_filter(r"/testing/data/[0-9a-fA-F]{4,8}/", "/testing/data/[HEX]/");
                settings.add_filter(r"(?i)\b(metadata_load_time|time_elapsed_opening|time_elapsed_processing|time_elapsed_scanning_total|time_elapsed_scanning_until_data|elapsed_compute|bloom_filter_eval_time|page_index_eval_time|row_pushdown_eval_time|statistics_eval_time)\s*=\s*[0-9]+(?:\.[0-9]+)?\s*(?:ns|µs|us|ms|s)", "$1=[TIME]");
                settings.add_filter(r"(-{140})(-{1,})", "$1");
                settings.add_filter(r"( {110})( {1,})", "$1");

                let setup: Vec<&str> = vec![$($($setup_queries),*)?];
                if !setup.is_empty() {
                    settings.set_info(
                        &format!(
                            "{}Setup queries: {}",
                            if snowflake_error { "Tests Snowflake Error; " } else { "" },
                            setup.join("; "),
                        ),
                    );
                } else if snowflake_error {
                    settings.set_info(&format!("Tests Snowflake Error"));
                }
                settings.bind(|| {
                    let df = match res {
                        Ok(record_batches) => {
                            let mut batches: Vec<datafusion::arrow::array::RecordBatch> = record_batches.records;
                            if !excluded_columns.is_empty() {
                                batches = catalog::test_utils::remove_columns_from_batches(batches, &excluded_columns);
                            }

                            if sort_all {
                                for batch in &mut batches {
                                    *batch = catalog::test_utils::sort_record_batch_by_sortable_columns(batch);
                                }
                            }
                            Ok(datafusion::arrow::util::pretty::pretty_format_batches(&batches).unwrap().to_string())
                        },
                        Err(e) => {
                            if snowflake_error {
                                // Do not convert to QueryExecution error before turning to snowflake error
                                // since we don't need query_id here
                                let e = e.to_snowflake_error();

                                // location is only available for debug purposes for not handled errors.
                                // it should not be saved to the snapshot, if location bothers you then
                                // remove snowflake_error macros arg or set to false.
                                let mut location = e.unhandled_location();
                                if !location.is_empty() {
                                    location = format!("; location: {}", location);
                                }

                                Err(format!("Snowflake Error: {e}{location}"))
                            } else {
                                Err(format!("Error: {e}"))
                            }
                        }
                    };

                    let df = df.map(|df| df.split('\n').map(|s| s.to_string()).collect::<Vec<String>>());
                    insta::assert_debug_snapshot!((df));
                });
            }
        }
    };
}

test_query!(describe_table, "DESCRIBE TABLE employee_table");

test_query!(
    copy_into_without_volume,
    "SELECT SUM(L_QUANTITY) FROM embucket.public.lineitem;",
    setup_queries = [
        "CREATE TABLE embucket.public.lineitem (
    L_ORDERKEY BIGINT NOT NULL,
    L_PARTKEY BIGINT NOT NULL,
    L_SUPPKEY BIGINT NOT NULL,
    L_LINENUMBER INT NOT NULL,
    L_QUANTITY DOUBLE NOT NULL,
    L_EXTENDED_PRICE DOUBLE NOT NULL,
    L_DISCOUNT DOUBLE NOT NULL,
    L_TAX DOUBLE NOT NULL,
    L_RETURNFLAG CHAR NOT NULL,
    L_LINESTATUS CHAR NOT NULL,
    L_SHIPDATE DATE NOT NULL,
    L_COMMITDATE DATE NOT NULL,
    L_RECEIPTDATE DATE NOT NULL,
    L_SHIPINSTRUCT VARCHAR NOT NULL,
    L_SHIPMODE VARCHAR NOT NULL,
    L_COMMENT VARCHAR NOT NULL );",
        "COPY INTO embucket.public.lineitem FROM 's3://embucket-testdata/tpch/lineitem.csv' FILE_FORMAT = ( TYPE = CSV );"
    ]
);
