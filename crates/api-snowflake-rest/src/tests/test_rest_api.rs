use super::TEST_JWT_SECRET;
use super::{run_test_rest_api_server, server_default_cfg};
use crate::sql_test;
use crate::tests::sql_test_macro::JSON;

mod compatible {
    use super::*;

    sql_test!(
        server_default_cfg(JSON),
        create_table_bad_syntax,
        [
            // "Snowflake:
            // 001003 (42000): UUID: SQL compilation error:
            // syntax error line 1 at position 16 unexpected '<EOF>'."
            "create table foo",
        ]
    );

    sql_test!(
        server_default_cfg(JSON),
        create_table_missing_schema,
        [
            // "Snowflake:
            // 002003 (02000): SQL compilation error:
            // Schema 'TESTS.MISSING_SCHEMA' does not exist or not authorized."
            "create table missing_schema.foo(a int)",
        ]
    );

    sql_test!(
        server_default_cfg(JSON),
        create_table_missing_db,
        [
            // "Snowflake:
            // 002003 (02000): SQL compilation error:
            // Database 'MISSING_DB' does not exist or not authorized."
            "create table missing_db.public.foo(a int)",
        ]
    );

    sql_test!(
        server_default_cfg(JSON),
        show_schemas_in_missing_db,
        [
            // "Snowflake:
            // 002043 (02000): UUID: SQL compilation error:
            // Object does not exist, or operation cannot be performed."
            "show schemas in database missing_db",
        ]
    );

    sql_test!(
        server_default_cfg(JSON),
        select_1,
        [
            // "Snowflake:
            // +---+
            // | 1 |
            // |---|
            // | 1 |
            // +---+"
            "select 1",
        ]
    );

    sql_test!(
        server_default_cfg(JSON),
        regression_bug_1662_ambiguous_schema,
        [
            // +-----+-----+
            // | COL | COL |
            // |-----+-----|
            // |   1 |   2 |
            // +-----+-----+
            "select * from 
                ( select 1 as col ) schema1,
                ( select 2 as col ) schema2",
        ]
    );

    sql_test!(
        server_default_cfg(JSON),
        alter_missing_table,
        [
            // 002003 (42S02): SQL compilation error:
            // Table 'EMBUCKET.PUBLIC.TEST2' does not exist or not authorized.
            "ALTER TABLE embucket.public.test ADD COLUMN new_col INT",
        ]
    );

    sql_test!(
        server_default_cfg(JSON),
        alter_table_schema_missing,
        [
            // 002003 (02000): SQL compilation error:
            // Schema 'EMBUCKET.MISSING_SCHEMA' does not exist or not authorized.
            "ALTER TABLE embucket.missing_schema.test ADD COLUMN new_col INT",
        ]
    );

    sql_test!(
        server_default_cfg(JSON),
        alter_table_db_missing,
        [
            // 002003 (02000): SQL compilation error:
            // Database 'MISSING_DB' does not exist or not authorized.
            "ALTER TABLE missing_db.public.test2 ADD COLUMN new_col INT",
        ]
    );

    sql_test!(
        server_default_cfg(JSON),
        regression_bug_591_date_timestamps,
        ["SELECT TO_DATE('2022-08-19', 'YYYY-MM-DD'), CAST('2022-08-19-00:00' AS TIMESTAMP)",]
    );

    sql_test!(
        server_default_cfg(JSON),
        chained_sqls,
        [
            "create schema if not exists embucket.test_schema",
            "create table if not exists embucket.test_schema.test_table (id int)",
            "use schema test_schema",
            "select count(*) from test_table"
        ]
    );
}

mod known_issues {
    use super::*;

    sql_test!(
        server_default_cfg(JSON),
        select_from_missing_table,
        [
            // "Snowflake:
            // 002003 (42S02): SQL compilation error
            // "Embucket:
            // 002003 (02000): SQL compilation error
            "select * from missing_table",
        ]
    );

    sql_test!(
        server_default_cfg(JSON),
        select_from_missing_schema,
        [
            // "Snowflake:
            // 002003 (02000): SQL compilation error:
            // Schema 'TESTS.MISSING_SCHEMA' does not exist or not authorized.
            // "Embucket:
            // 002003 (02000): SQL compilation error:
            // table 'embucket.missing_schema.foo' not found
            "select * from missing_schema.foo",
        ]
    );

    sql_test!(
        server_default_cfg(JSON),
        select_from_missing_db,
        [
            // "Snowflake:
            // 002003 (02000): SQL compilation error:
            // Schema 'TESTS.MISSING_SCHEMA' does not exist or not authorized.
            // "Embucket:
            // 002003 (02000): SQL compilation error:
            // table 'embucket.missing_schema.foo' not found
            "select * from missing_db.foo.foo",
        ]
    );
}

mod custom_server {
    use super::*;
    use crate::server::server_models::RestApiConfig;
    use crate::tests::sql_test_macro::ARROW;
    use executor::utils::Config as UtilsConfig;

    #[allow(clippy::unnecessary_wraps)]
    fn server_custom_cfg(data_format: &str) -> Option<(RestApiConfig, UtilsConfig)> {
        Some((
            RestApiConfig::new(data_format, TEST_JWT_SECRET.to_string())
                .expect("Failed to create server config")
                .with_demo_credentials("embucket".to_string(), "embucket".to_string()),
            UtilsConfig::default()
                .with_max_concurrency_level(2)
                .with_query_timeout(2),
        ))
    }

    sql_test!(
        server_custom_cfg(ARROW),
        select_date_timestamp_in_arrow_format,
        ["SELECT TO_DATE('2022-08-19', 'YYYY-MM-DD'), CAST('2022-08-19-00:00' AS TIMESTAMP)"]
    );
}
