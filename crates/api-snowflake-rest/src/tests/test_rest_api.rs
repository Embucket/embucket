use super::TEST_JWT_SECRET;
use super::run_test_rest_api_server;
use crate::sql_test;
use crate::tests::snow_sql::*;
use crate::tests::sql_test_macro::{SqlTest, sql_test_wrapper};
use crate::server::core_state::MetastoreConfig;

mod compatible {
    use super::*;

    sql_test!(
        create_table_bad_syntax,
        SqlTest::new(&[
            // "Snowflake:
            // 001003 (42000): UUID: SQL compilation error:
            // syntax error line 1 at position 16 unexpected '<EOF>'."
            "create table foo",
        ])
    );

    sql_test!(
        create_table_missing_db,
        SqlTest::new(&[
            // "Snowflake:
            // 002003 (02000): SQL compilation error:
            // Database 'MISSING_DB' does not exist or not authorized."
            "create table missing_db.public.foo(a int)",
        ])
    );

    sql_test!(
        show_schemas_in_missing_db,
        SqlTest::new(&[
            // "Snowflake:
            // 002043 (02000): UUID: SQL compilation error:
            // Object does not exist, or operation cannot be performed."
            "show schemas in database missing_db",
        ])
    );

    sql_test!(
        select_1,
        SqlTest::new(&[
            // "Snowflake:
            // +---+
            // | 1 |
            // |---|
            // | 1 |
            // +---+"
            "select 1",
        ])
    );

    sql_test!(
        regression_bug_1662_ambiguous_schema,
        SqlTest::new(&[
            // +-----+-----+
            // | COL | COL |
            // |-----+-----|
            // |   1 |   2 |
            // +-----+-----+
            "select * from 
                ( select 1 as col ) schema1,
                ( select 2 as col ) schema2",
        ])
    );

    sql_test!(
        alter_table_db_missing,
        SqlTest::new(&[
            // 002003 (02000): SQL compilation error:
            // Database 'MISSING_DB' does not exist or not authorized.
            "ALTER TABLE missing_db.public.test2 ADD COLUMN new_col INT",
        ])
    );

    sql_test!(
        regression_bug_591_date_timestamps,
        SqlTest::new(&[
            // SELECT TO_DATE('2022-08-19', 'YYYY-MM-DD'), CAST('2022-08-19-00:00' AS TIMESTAMP)
            "SELECT TO_DATE('2022-08-19', 'YYYY-MM-DD'), CAST('2022-08-19-00:00' AS TIMESTAMP)",
        ])
    );

    sql_test!(
        use_command_show_variables,
        SqlTest::new(&["use schema test_schema", "SHOW VARIABLES"])
    );

    sql_test!(
        create_table_missing_schema,
        SqlTest::new(&[
            // "Snowflake:
            // 002003 (02000): SQL compilation error:
            // Schema 'TESTS.MISSING_SCHEMA' does not exist or not authorized."
            "create table missing_schema.foo(a int)",
        ])
        .with_metastore_config(MetastoreConfig::DefaultConfig)
    );

    sql_test!(
        alter_missing_table,
        SqlTest::new(&[
            // 002003 (42S02): SQL compilation error:
            // Table 'EMBUCKET.PUBLIC.TEST2' does not exist or not authorized.
            "ALTER TABLE embucket.public.test ADD COLUMN new_col INT",
        ])
        .with_metastore_config(MetastoreConfig::DefaultConfig)
    );

    sql_test!(
        alter_table_schema_missing,
        SqlTest::new(&[
            // 002003 (02000): SQL compilation error:
            // Schema 'EMBUCKET.MISSING_SCHEMA' does not exist or not authorized.
            "ALTER TABLE embucket.missing_schema.test ADD COLUMN new_col INT",
        ])
        .with_metastore_config(MetastoreConfig::DefaultConfig)
    );

    sql_test!(
        login_specified_params,
        SqlTest::new(&["select count(*) from test_table"])
            .with_setup_queries(&[
                "create schema if not exists embucket.test_schema",
                "create table if not exists embucket.test_schema.test_table (id int)",
            ])
            .with_params(vec![
                (DATABASE_QUERY_PARAM_KEY, "embucket".to_string()),
                (SCHEMA_QUERY_PARAM_KEY, "test_schema".to_string()),
            ])
            .with_metastore_config(MetastoreConfig::DefaultConfig)
    );
}

mod known_issues {
    use super::*;

    sql_test!(
        select_from_missing_table,
        SqlTest::new(&[
            // "Snowflake:
            // 002003 (42S02): SQL compilation error
            // "Embucket:
            // 002003 (02000): SQL compilation error
            "select * from missing_table",
        ])
    );

    sql_test!(
        select_from_missing_schema,
        SqlTest::new(&[
            // "Snowflake:
            // 002003 (02000): SQL compilation error:
            // Schema 'TESTS.MISSING_SCHEMA' does not exist or not authorized.
            // "Embucket:
            // 002003 (02000): SQL compilation error:
            // table 'embucket.missing_schema.foo' not found
            "select * from missing_schema.foo",
        ])
    );

    sql_test!(
        select_from_missing_db,
        SqlTest::new(&[
            // "Snowflake:
            // 002003 (02000): SQL compilation error:
            // Schema 'TESTS.MISSING_SCHEMA' does not exist or not authorized.
            // "Embucket:
            // 002003 (02000): SQL compilation error:
            // table 'embucket.missing_schema.foo' not found
            "select * from missing_db.foo.foo",
        ])
    );

    sql_test!(
        use_command_then_select,
        SqlTest::new(&["select count(*) from test_table"]).with_setup_queries(&[
            "create schema if not exists embucket.test_schema",
            "create table if not exists embucket.test_schema.test_table (id int)",
        ])
        .with_metastore_config(MetastoreConfig::DefaultConfig)
    );
}

mod custom_server {
    use super::*;
    use crate::server::server_models::RestApiConfig;
    use crate::tests::sql_test_macro::ARROW;

    sql_test!(
        select_date_timestamp_in_arrow_format,
        SqlTest::new(&[
            "SELECT TO_DATE('2022-08-19', 'YYYY-MM-DD'), CAST('2022-08-19-00:00' AS TIMESTAMP)",
        ])
        .with_server_config(
            RestApiConfig::new(ARROW, TEST_JWT_SECRET.to_string())
                .expect("Failed to create server config")
                .with_demo_credentials("embucket".to_string(), "embucket".to_string()),
        )
    );

    sql_test!(
        test_query_timeout,
        SqlTest::new(&[
            "SELECT SLEEP(2)",
        ])
        .with_server_config(
            RestApiConfig::new(ARROW, TEST_JWT_SECRET.to_string())
                .expect("Failed to create server config")
                .with_demo_credentials("embucket".to_string(), "embucket".to_string()),
        )
        .with_executor_config(
            executor::utils::Config::default().with_query_timeout(1)
        )
    );
}
