use super::TEST_JWT_SECRET;
use super::{run_test_rest_api_server, server_default_cfg};
use crate::sql_test;
use crate::tests::sql_macro::JSON;

// Tests executed sequentially, as of:
//  - no queues of queries yet, so parallel execution would exceeded concurrency limit;

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
        ],
        [|s| {
            insta::assert_yaml_snapshot!(s, @r#"
                data:
                  rowtype: []
                  rowsetBase64: ~
                  rowset: ~
                  queryResultFormat: ~
                  errorCode: "001003"
                  sqlState: "02000"
                  queryId: UUID
                success: false
                message: "SQL compilation error: syntax error unexpected end of input"
                code: "001003"
                "#)
        }]
    );

    sql_test!(
        server_default_cfg(JSON),
        create_table_missing_schema,
        [
            // "Snowflake:
            // 002003 (02000): SQL compilation error:
            // Schema 'TESTS.MISSING_SCHEMA' does not exist or not authorized."
            "create table missing_schema.foo(a int)",
        ],
        [|s| {
            insta::assert_yaml_snapshot!(s, @r#"
                data:
                  rowtype: []
                  rowsetBase64: ~
                  rowset: ~
                  queryResultFormat: ~
                  errorCode: "002003"
                  sqlState: "02000"
                  queryId: UUID
                success: false
                message: "SQL compilation error: Schema 'embucket.missing_schema' does not exist or not authorized"
                code: "002003"
                "#)
        }]
    );

    sql_test!(
        server_default_cfg(JSON),
        create_table_missing_db,
        [
            // "Snowflake:
            // 002003 (02000): SQL compilation error:
            // Database 'MISSING_DB' does not exist or not authorized."
            "create table missing_db.public.foo(a int)",
        ],
        [|s| {
            insta::assert_yaml_snapshot!(s, @r#"
                data:
                  rowtype: []
                  rowsetBase64: ~
                  rowset: ~
                  queryResultFormat: ~
                  errorCode: "002003"
                  sqlState: "02000"
                  queryId: UUID
                success: false
                message: "SQL compilation error: Database 'missing_db' does not exist or not authorized"
                code: "002003"
                "#)
        }]
    );

    sql_test!(
        server_default_cfg(JSON),
        show_schemas_in_missing_db,
        [
            // "Snowflake:
            // 002043 (02000): UUID: SQL compilation error:
            // Object does not exist, or operation cannot be performed."
            "show schemas in database missing_db",
        ],
        [|s| {
            insta::assert_yaml_snapshot!(s, @r#"
                data:
                  rowtype: []
                  rowsetBase64: ~
                  rowset: ~
                  queryResultFormat: ~
                  errorCode: "002043"
                  sqlState: "02000"
                  queryId: UUID
                success: false
                message: "SQL compilation error: Database 'missing_db' does not exist or not authorized"
                code: "002043"
                "#)
        }]
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
        ],
        [|s| {
            insta::assert_yaml_snapshot!(s, @r#"
                data:
                  rowtype:
                    - name: Int64(1)
                      database: ""
                      schema: ""
                      table: ""
                      nullable: false
                      type: fixed
                      byteLength: ~
                      length: ~
                      scale: 0
                      precision: 38
                      collation: ~
                  rowsetBase64: ~
                  rowset:
                    "$serde_json::private::RawValue": "[[1]]"
                  total: 1
                  returned: 1
                  queryResultFormat: json
                  sqlState: "02000"
                  queryId: UUID
                success: true
                message: successfully executed
                code: ~
                "#)
        }]
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
        ],
        [|s| {
            insta::assert_yaml_snapshot!(s, @r#"
                data:
                  rowtype:
                    - name: col
                      database: ""
                      schema: ""
                      table: ""
                      nullable: false
                      type: fixed
                      byteLength: ~
                      length: ~
                      scale: 0
                      precision: 38
                      collation: ~
                    - name: col
                      database: ""
                      schema: ""
                      table: ""
                      nullable: false
                      type: fixed
                      byteLength: ~
                      length: ~
                      scale: 0
                      precision: 38
                      collation: ~
                  rowsetBase64: ~
                  rowset:
                    "$serde_json::private::RawValue": "[[1,2]]"
                  total: 1
                  returned: 1
                  queryResultFormat: json
                  sqlState: "02000"
                  queryId: UUID
                success: true
                message: successfully executed
                code: ~
                "#)
        }]
    );

    sql_test!(
        server_default_cfg(JSON),
        alter_missing_table,
        [
            // 002003 (42S02): SQL compilation error:
            // Table 'EMBUCKET.PUBLIC.TEST2' does not exist or not authorized.
            "ALTER TABLE embucket.public.test ADD COLUMN new_col INT",
        ],
        [|s| {
            insta::assert_yaml_snapshot!(s, @r#"
                data:
                  rowtype: []
                  rowsetBase64: ~
                  rowset: ~
                  queryResultFormat: ~
                  errorCode: "002003"
                  sqlState: 42S02
                  queryId: UUID
                success: false
                message: "SQL compilation error: Table 'embucket.public.test' does not exist or not authorized"
                code: "002003"
                "#)
        }]
    );

    sql_test!(
        server_default_cfg(JSON),
        alter_table_schema_missing,
        [
            // 002003 (02000): SQL compilation error:
            // Schema 'EMBUCKET.MISSING_SCHEMA' does not exist or not authorized.
            "ALTER TABLE embucket.missing_schema.test ADD COLUMN new_col INT",
        ],
        [|s| {
            insta::assert_yaml_snapshot!(s, @r#"
                data:
                  rowtype: []
                  rowsetBase64: ~
                  rowset: ~
                  queryResultFormat: ~
                  errorCode: "002003"
                  sqlState: "02000"
                  queryId: UUID
                success: false
                message: "SQL compilation error: Schema 'embucket.missing_schema' does not exist or not authorized"
                code: "002003"
                "#)
        }]
    );

    sql_test!(
        server_default_cfg(JSON),
        alter_table_db_missing,
        [
            // 002003 (02000): SQL compilation error:
            // Database 'MISSING_DB' does not exist or not authorized.
            "ALTER TABLE missing_db.public.test2 ADD COLUMN new_col INT",
        ],
        [|s| {
            insta::assert_yaml_snapshot!(s, @r#"
                data:
                  rowtype: []
                  rowsetBase64: ~
                  rowset: ~
                  queryResultFormat: ~
                  errorCode: "002003"
                  sqlState: "02000"
                  queryId: UUID
                success: false
                message: "SQL compilation error: Database 'missing_db' does not exist or not authorized"
                code: "002003"
                "#)
        }]
    );

    sql_test!(
        server_default_cfg(JSON),
        regression_bug_591_date_timestamps,
        ["SELECT TO_DATE('2022-08-19', 'YYYY-MM-DD'), CAST('2022-08-19-00:00' AS TIMESTAMP)",],
        [|s| {
            insta::assert_yaml_snapshot!(s, @r#"
                data:
                  rowtype:
                    - name: "to_date(Utf8(\"2022-08-19\"),Utf8(\"YYYY-MM-DD\"))"
                      database: ""
                      schema: ""
                      table: ""
                      nullable: false
                      type: date
                      byteLength: ~
                      length: ~
                      scale: ~
                      precision: ~
                      collation: ~
                    - name: "to_timestamp(Utf8(\"2022-08-19-00:00\"))"
                      database: ""
                      schema: ""
                      table: ""
                      nullable: false
                      type: timestamp_ntz
                      byteLength: ~
                      length: ~
                      scale: 9
                      precision: 0
                      collation: ~
                  rowsetBase64: ~
                  rowset:
                    "$serde_json::private::RawValue": "[[19223,\"1660867200.0\"]]"
                  total: 1
                  returned: 1
                  queryResultFormat: json
                  sqlState: "02000"
                  queryId: UUID
                success: true
                message: successfully executed
                code: ~
                "#)
        }]
    );

    sql_test!(
        server_default_cfg(JSON),
        chained_sqls,
        [
            "create schema if not exists embucket.test_schema",
            "create table if not exists embucket.test_schema.test_table (id int)",
            "use embucket.test_schema",
            "select count(*) from test_table"
        ],
        [
            |s| {
                insta::assert_yaml_snapshot!(s, @r#""#)
            },
            |s| {
                insta::assert_yaml_snapshot!(s, @r#""#)
            },
            |s| {
                insta::assert_yaml_snapshot!(s, @r#""#)
            },
            |s| {
                insta::assert_yaml_snapshot!(s, @r#""#)
            },
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
        ],
        [|s| {
            insta::assert_yaml_snapshot!(s, @r#"
                data:
                  rowtype: []
                  rowsetBase64: ~
                  rowset: ~
                  queryResultFormat: ~
                  errorCode: "002003"
                  sqlState: "02000"
                  queryId: UUID
                success: false
                message: "SQL compilation error: table 'embucket.public.missing_table' not found"
                code: "002003"
                "#);
        }]
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
        ],
        [|s| {
            insta::assert_yaml_snapshot!(s, @r#"
                data:
                  rowtype: []
                  rowsetBase64: ~
                  rowset: ~
                  queryResultFormat: ~
                  errorCode: "002003"
                  sqlState: "02000"
                  queryId: UUID
                success: false
                message: "SQL compilation error: table 'embucket.missing_schema.foo' not found"
                code: "002003"
                "#)
        }]
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
        ],
        [|s| {
            insta::assert_yaml_snapshot!(s, @r#"
                data:
                  rowtype: []
                  rowsetBase64: ~
                  rowset: ~
                  queryResultFormat: ~
                  errorCode: "002003"
                  sqlState: "02000"
                  queryId: UUID
                success: false
                message: "SQL compilation error: table 'missing_db.foo.foo' not found"
                code: "002003"
                "#)
        }]
    );
}

mod custom_server {
    use super::*;
    use crate::server::server_models::RestApiConfig;
    use crate::tests::sql_macro::ARROW;
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
        ["SELECT TO_DATE('2022-08-19', 'YYYY-MM-DD'), CAST('2022-08-19-00:00' AS TIMESTAMP)"],
        [|s| {
            insta::assert_yaml_snapshot!(s, @r#"
                data:
                  rowtype:
                    - name: "to_date(Utf8(\"2022-08-19\"),Utf8(\"YYYY-MM-DD\"))"
                      database: ""
                      schema: ""
                      table: ""
                      nullable: false
                      type: date
                      byteLength: ~
                      length: ~
                      scale: ~
                      precision: ~
                      collation: ~
                    - name: "to_timestamp(Utf8(\"2022-08-19-00:00\"))"
                      database: ""
                      schema: ""
                      table: ""
                      nullable: false
                      type: timestamp_ntz
                      byteLength: ~
                      length: ~
                      scale: 9
                      precision: 0
                      collation: ~
                  rowsetBase64: /////2ACAAAQAAAAAAAKAAwACgAJAAQACgAAABAAAAAAAQQACAAIAAAABAAIAAAABAAAAAIAAAAkAQAABAAAAPb+//9YAAAAGAAAACAAAAAAAAACHAAAAAgADAAEAAsACAAAAEAAAAAAAAABAAAAACYAAAB0b190aW1lc3RhbXAoVXRmOCgiMjAyMi0wOC0xOS0wMDowMCIpKQAABAAAAIQAAABQAAAAKAAAAAQAAAB0/v//CAAAAAwAAAABAAAAOQAAAAUAAABzY2FsZQAAAJT+//8IAAAADAAAAAEAAAAwAAAACQAAAHByZWNpc2lvbgAAALj+//8IAAAAGAAAAA0AAABUSU1FU1RBTVBfTlRaAAAACwAAAGxvZ2ljYWxUeXBlAOj+//8IAAAADAAAAAEAAAAwAAAACgAAAGNoYXJMZW5ndGgAAAAAEgAYABQAAAATAAgAAAAMAAQAEgAAAFwAAAAcAAAADAAAAAAAAAgYAAAAAAAAAAAABgAIAAYABgAAAAAAAAAuAAAAdG9fZGF0ZShVdGY4KCIyMDIyLTA4LTE5IiksVXRmOCgiWVlZWS1NTS1ERCIpKQAABAAAAIQAAABQAAAAKAAAAAQAAACU////CAAAAAwAAAABAAAAMAAAAAUAAABzY2FsZQAAALT///8IAAAADAAAAAIAAAAzOAAACQAAAHByZWNpc2lvbgAAANj///8IAAAAEAAAAAQAAABEQVRFAAAAAAsAAABsb2dpY2FsVHlwZQAIAAwACAAEAAgAAAAIAAAADAAAAAEAAAAwAAAACgAAAGNoYXJMZW5ndGgAAP////+4AAAAEAAAAAwAGgAYABcABAAIAAwAAAAgAAAAIAAAAAAAAAAAAAAAAAAAAwQACgAYAAwACAAEAAoAAAA8AAAAEAAAAAEAAAAAAAAAAAAAAAIAAAABAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAEAAAAAAAAAAAAAAABAAAAAAAAAAgAAAAAAAAABAAAAAAAAAAQAAAAAAAAAAEAAAAAAAAAGAAAAAAAAAAIAAAAAAAAAP8AAAAAAAAAF0sAAAAAAAD/AAAAAAAAAAAAGTPrlQwX/////wAAAAA=
                  rowset: ~
                  total: 1
                  returned: 1
                  queryResultFormat: arrow
                  sqlState: "02000"
                  queryId: UUID
                success: true
                message: successfully executed
                code: ~
                "#)
        }]
    );
}
