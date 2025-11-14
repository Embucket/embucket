use crate::test_query;

// SESSION RELATED https://docs.snowflake.com/en/sql-reference/commands-session
test_query!(
    alter_session_set,
    "SHOW VARIABLES",
    setup_queries = ["ALTER SESSION SET v1 = 'test'"],
    exclude_columns = ["created_on", "updated_on", "session_id"],
    snapshot_path = "alter"
);
test_query!(
    alter_session_unset,
    "SHOW VARIABLES",
    setup_queries = [
        "ALTER SESSION SET v1 = 'test' v2 = 1",
        "ALTER SESSION UNSET v1"
    ],
    exclude_columns = ["created_on", "updated_on", "session_id"],
    snapshot_path = "alter"
);