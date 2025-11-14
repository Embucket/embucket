use crate::test_query;

test_query!(
    timestamp_str_format,
    "SELECT
       TO_TIMESTAMP('04/05/2024 01:02:03', 'mm/dd/yyyy hh24:mi:ss') as a,
       TO_TIMESTAMP('04/05/2024 01:02:03') as b",
    setup_queries = ["SET timestamp_input_format = 'mm/dd/yyyy hh24:mi:ss'"],
    snapshot_path = "to_timestamp"
);

test_query!(
    timestamp_timezone,
    "SELECT TO_TIMESTAMP(1000000000)",
    setup_queries = ["ALTER SESSION SET timestamp_input_mapping = 'timestamp_tz'"],
    snapshot_path = "to_timestamp"
);

test_query!(
    timestamp_with_timezone_to_timestamp,
    "SELECT TO_TIMESTAMP(CONVERT_TIMEZONE('UTC', '2024-12-31 10:00:00.000'::TIMESTAMP)) as model_tstamp;",
    snapshot_path = "to_timestamp"
);
