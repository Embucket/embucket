use crate::test_query;

test_query!(
    set_variable_with_binary_op_placeholder,
    "SELECT $max",
    setup_queries = [
        "SET (min, max) = (40, 70);",
        "SET (min, max) = (50, 2 * $min)",
    ],
    exclude_columns = ["created_on", "updated_on", "session_id"],
    snapshot_path = "set"
);
test_query!(
    set_variable_and_access_by_placeholder,
    "SELECT $v1",
    setup_queries = ["SET v1 = 'test';"],
    exclude_columns = ["created_on", "updated_on", "session_id"],
    snapshot_path = "set"
);
test_query!(
    set_variable_system,
    "SELECT name, value FROM snowplow.information_schema.df_settings
     WHERE name = 'datafusion.execution.time_zone'",
    setup_queries = ["SET datafusion.execution.time_zone = 'TEST_TIMEZONE'"],
    snapshot_path = "set"
);

// TODO Currently UNSET is not supported
test_query!(
    unset_variable,
    "UNSET v3",
    setup_queries = ["SET v1 = 'test'", "SET v2 = 1", "SET v3 = true"],
    snapshot_path = "set"
);
test_query!(
    session_last_query_id,
    "SELECT
        length(LAST_QUERY_ID()) > 0 as last,
        length(LAST_QUERY_ID(-1)) > 0 as last_index,
        length(LAST_QUERY_ID(2)) > 0 as second,
        length(LAST_QUERY_ID(100)) = 0 as empty",
    setup_queries = ["SET v1 = 'test'", "SET v2 = 1", "SET v3 = true"],
    snapshot_path = "set"
);
