use crate::test_query;

// https:://docs.snowflake.com/en/sql-reference/sql/explain
// https://datafusion.apache.org/user-guide/sql/explain.html
// Datafusion has different output format.
// Check session config ExplainOptions for the full list of options
// logical_only_plan flag is used to only print logical plans
// since physical plan contains dynamic files names
test_query!(
    explain_select,
    "EXPLAIN SELECT * FROM embucket.public.employee_table",
    setup_queries = ["SET datafusion.explain.logical_plan_only = true"],
    snapshot_path = "explain"
);
test_query!(
    explain_select_limit,
    "EXPLAIN SELECT * FROM embucket.public.employee_table limit 1",
    setup_queries = ["SET datafusion.explain.logical_plan_only = true"],
    snapshot_path = "explain"
);
test_query!(
    explain_select_column,
    "EXPLAIN SELECT last_name FROM embucket.public.employee_table limit 1",
    setup_queries = ["SET datafusion.explain.logical_plan_only = true"],
    snapshot_path = "explain"
);
test_query!(
    explain_select_missing_column,
    "EXPLAIN SELECT missing FROM embucket.public.employee_table limit 1",
    setup_queries = ["SET datafusion.explain.logical_plan_only = true"],
    snapshot_path = "explain"
);
