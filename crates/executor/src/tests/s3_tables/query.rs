use crate::models::QueryContext;
use crate::running_queries::RunningQueriesRegistry;
use crate::service::CoreExecutionService;
use crate::session::UserSession;
use crate::utils::Config;
#[cfg(feature = "state-store")]
use aws_sdk_dynamodb::{Client, Config as AwsConfig};
use catalog_metastore::InMemoryMetastore;
use catalog_metastore::metastore_bootstrap_config::MetastoreBootstrapConfig;
use datafusion::prelude::SessionContext;
#[cfg(feature = "state-store")]
use state_store::DynamoDbStateStore;
use std::sync::Arc;

#[allow(clippy::expect_used, clippy::unwrap_used)]
pub async fn create_s3_tables_df_session() -> Arc<UserSession> {
    let metastore = Arc::new(InMemoryMetastore::new());
    let metastore_config =
        MetastoreBootstrapConfig::load_from_env().expect("Failed to load volume config");
    assert!(
        metastore_config.contains_s3_tables_volume(),
        "Failed to load volume config"
    );
    metastore_config
        .apply(metastore.clone())
        .await
        .expect("Failed to apply config");
    let config = Arc::new(Config::default());
    let catalog_list = CoreExecutionService::catalog_list(metastore.clone(), &config)
        .await
        .expect("Failed to create catalog list");
    let runtime_env = CoreExecutionService::runtime_env(&config, catalog_list.clone())
        .expect("Failed to create runtime env");

    #[cfg(feature = "state-store")]
    let client = Client::from_conf(AwsConfig::builder().build());
    #[cfg(feature = "state-store")]
    let state_store = Arc::new(DynamoDbStateStore::new(client, ""));

    Arc::new(
        UserSession::new(
            metastore,
            Arc::new(RunningQueriesRegistry::new()),
            Arc::new(Config::default()),
            catalog_list,
            runtime_env,
            "",
            #[cfg(feature = "state-store")]
            state_store,
        )
        .await
        .expect("Failed to create user session"),
    )
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "s3 tables integration"]
#[allow(clippy::unwrap_used)]
async fn run_queries() {
    let queries = [
        ("SHOW DATABASES", "show_databases"),
        (
            "CREATE SCHEMA IF NOT EXISTS embucket.tests",
            "create_schema_if_not_exists",
        ),
        ("CREATE SCHEMA embucket.tests_second", "create_schema"),
        ("SHOW SCHEMAS IN embucket", "show_schemas_initial"),
        (
            "CREATE TABLE IF NOT EXISTS embucket.tests.first (id INT, name VARCHAR, category STRING)",
            "create_table_if_not_exists",
        ),
        (
            "CREATE OR REPLACE TABLE embucket.tests.second (id INT, name VARCHAR, category STRING)",
            "create_table_or_replace",
        ),
        (
            "CREATE TABLE embucket.tests.third (id INT, name VARCHAR)",
            "create_table",
        ),
        ("SHOW TABLES IN embucket.tests", "show_tables_initial"),
        (
            "INSERT INTO embucket.tests.first VALUES (1, 'old', 'A'), (2, 'keep', 'B')",
            "insert_first",
        ),
        (
            "SELECT count(*) FROM embucket.tests.first",
            "select_with_insert",
        ),
        ("SHOW COLUMNS IN embucket.tests.first", "show_columns"),
        (
            "INSERT INTO embucket.tests.second VALUES (1, 'updated', 'A'), (3, 'inserted', 'C')",
            "insert_second",
        ),
        ("MERGE INTO embucket.tests.first AS tgt USING embucket.tests.second AS src
            ON tgt.id = src.id
            WHEN MATCHED THEN UPDATE SET name = src.name, category = src.category
            WHEN NOT MATCHED THEN INSERT (id, name, category) VALUES (src.id, src.name, src.category)",
            "merge_into"
        ),
        ("SELECT * FROM embucket.tests.first", "merge_into_result"),
        ("DROP TABLE embucket.tests.second", "drop_table"),
        ("SHOW TABLES IN embucket.tests", "show_tables_after_drop"),
        ("DROP SCHEMA embucket.tests_second", "drop_schema"),
        ("SHOW SCHEMAS IN embucket", "show_schemas_after_drop"),
    ];

    let session = create_s3_tables_df_session().await;
    // cleanup BEFORE running test
    CleanupGuard {
        ctx: session.ctx.clone(),
    }
    .cleanup();

    // cleanup AFTER running test (guard dropped at end of scope)
    let _guard = CleanupGuard {
        ctx: session.ctx.clone(),
    };

    for (query, ident) in queries {
        let mut q = session.query(query, QueryContext::default());
        let res = q.execute().await;

        if ident.is_empty() {
            res.unwrap();
        } else {
            let mut settings = insta::Settings::new();
            settings.set_description(query);
            settings.set_omit_expression(true);
            settings.set_prepend_module_to_snapshot(false);
            settings.bind(|| {
                let df = match res {
                    Ok(query_res) => Ok(datafusion::arrow::util::pretty::pretty_format_batches(
                        &query_res.records,
                    )
                    .unwrap()
                    .to_string()),
                    Err(e) => Err(format!("Error: {e}")),
                };
                let df = df.map(|df| {
                    df.split('\n')
                        .map(ToString::to_string)
                        .collect::<Vec<String>>()
                });
                insta::assert_debug_snapshot!(ident, df);
            });
        }
    }
}

struct CleanupGuard {
    ctx: SessionContext,
}

impl CleanupGuard {
    fn cleanup(&self) {
        if let Some(catalog) = self.ctx.state().catalog_list().catalog("embucket") {
            for schema_name in catalog.schema_names() {
                if let Some(schema) = catalog.schema(&schema_name) {
                    for table in schema.table_names() {
                        let _ = schema.deregister_table(&table);
                    }
                }
                let _ = catalog.deregister_schema(&schema_name, true);
            }
        }
    }
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        self.cleanup();
    }
}
