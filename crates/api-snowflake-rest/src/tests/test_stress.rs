use crate::server::core_state::MetastoreConfig;
use crate::tests::sql_test_macro::{SqlTest, sql_test_wrapper};
use catalog_metastore::metastore_settings_config::MetastoreSettingsConfig;
use tokio::task::JoinError;

fn check_if_test_failed(results: Vec<std::result::Result<bool, JoinError>>) -> bool {
    results.into_iter().all(|r| {
        r.unwrap_or_else(|e| {
            eprintln!("Task join error: {e:?}");
            false
        })
    })
}

mod stress {
    use super::*;

    // This test is for reference to be sure no errors happens when working with memory database
    #[tokio::test(flavor = "multi_thread")]
    async fn concurrency_test_memory_database() {
        let expected_patterns = vec![
            "successfully executed",
            "Too many open files", // OS related error
        ];
        let handles = (0..50)
            .map(|idx| {
                let expected_patterns = expected_patterns.clone();
                tokio::spawn(async move {
                    sql_test_wrapper(
                        SqlTest::new(&[
                            "create table if not exists embucket.public.test_table (id int)",
                            "drop table if exists embucket.public.test_table",
                        ])
                        .with_metastore_bootstrap_config(MetastoreConfig::DefaultConfig)
                        .with_skip_login(),
                        move |(sql, _), response| {
                            let err_msg = response.message.clone().unwrap_or_default();
                            let err_code = response.code.clone().unwrap_or_default();
                            let result = format!("{err_msg} {err_code}");
                            eprintln!("{idx}: {sql} = {result}");
                            expected_patterns
                                .iter()
                                .any(|pattern| result.trim().contains(pattern))
                        },
                    )
                    .await
                })
            })
            .collect::<Vec<_>>();

        let results = futures::future::join_all(handles).await;
        assert!(check_if_test_failed(results));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrency_test_s3tables_database() {
        // Returned results match any of these patterns
        let expected_patterns = vec![
            "successfully executed",
            "Too many open files", // OS related error
            "SQL compilation error: Schema 's3_table_db.schema1' does not exist or not authorized 002003",
            "Generic S3 error: Error performing GET",
            "Iceberg Object store: The operation lacked the necessary privileges to complete for path metadata",
            "Table test_table not found",
            // broken s3 tables error
            "External error: Iceberg error: service error",
            // create table if not exists
            "External error: The operation lacked the necessary privileges to complete for path metadata",
            // create table if not exists
            "S3Tables create table failed with service error ConflictException (HTTP 409)",
            // create table if not exists
            "External error: ForbiddenException: Unauthorized",
            // create table if not exists
            "S3Tables get table failed with service error NotFoundException (HTTP 404)",
        ];
        if let Ok(metastore_config_json) = std::env::var("METASTORE_CONFIG_JSON") {
            let metastore_config = MetastoreConfig::ConfigJson(metastore_config_json);

            let handles = (0..50).map(|idx| {
                let metastore_config = metastore_config.clone();
                let expected_patterns = expected_patterns.clone();
                tokio::spawn(async move {
                    sql_test_wrapper(
                    SqlTest::new(&[
                        "create schema if not exists s3_table_db.schema1",
                        "create table if not exists s3_table_db.schema1.test_table (id int)",
                        "drop table if exists s3_table_db.schema1.test_table",
                    ])
                    .with_global_settings(MetastoreSettingsConfig::default())
                    .with_metastore_bootstrap_config(metastore_config)
                    .with_skip_login(),
                    move |sql_info, response| {
                        let sql = sql_info.0;
                        let msg = response.message.clone().unwrap_or_default();
                        let err_code = response.code.clone().unwrap_or_default();
                        let bool_result = expected_patterns
                            .iter()
                            .any(|pattern| msg.contains(pattern.trim()));
                        eprintln!("{idx}: {} {sql} = {msg} {err_code}", if bool_result { "" } else { "NEW_ERROR" });
                        bool_result
                    }).await
                })
            }).collect::<Vec<_>>();

            let results = futures::future::join_all(handles).await;
            assert!(check_if_test_failed(results));
        }
    }
}
