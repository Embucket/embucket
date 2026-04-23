use crate::test_query;

// SELECT
test_query!(
    select_star,
    "SELECT * FROM employee_table",
    snapshot_path = "select"
);

// FIXME: ILIKE is not supported yet
// test_query!(select_ilike, "SELECT * ILIKE '%id%' FROM employee_table;");
test_query!(
    select_exclude,
    "SELECT * EXCLUDE department_id FROM employee_table;",
    snapshot_path = "select"
);

test_query!(
    select_exclude_multiple,
    "SELECT * EXCLUDE (department_id, employee_id) FROM employee_table;",
    snapshot_path = "select"
);

test_query!(
    qualify,
    "SELECT product_id, retail_price, quantity, city
    FROM sales
    QUALIFY ROW_NUMBER() OVER (PARTITION BY city ORDER BY retail_price) = 1;",
    snapshot_path = "select"
);

// Regression test for issue #131: when a SELECT-list alias (`start_tstamp`)
// shadows an actual column of the FROM-clause CTE, references to that name
// inside other projection expressions must still resolve to the FROM-clause
// column (per ANSI SQL / Snowflake), not the alias. If the alias is inlined
// instead, the CASE predicate degenerates to `user_start_tstamp =
// user_start_tstamp` (always true) and the aggregate returns `S2` instead of
// the correct `S1`.
test_query!(
    alias_shadows_column_in_aggregate_case,
    "WITH s AS (
        SELECT 'S1' AS sid,
               TIMESTAMP '2020-01-01 00:00:00' AS start_tstamp,
               TIMESTAMP '2020-01-01 00:00:00' AS user_start_tstamp
        UNION ALL
        SELECT 'S2' AS sid,
               TIMESTAMP '2020-01-01 05:00:00' AS start_tstamp,
               TIMESTAMP '2020-01-01 00:00:00' AS user_start_tstamp
    )
    SELECT user_start_tstamp AS start_tstamp,
           MAX(CASE WHEN start_tstamp = user_start_tstamp THEN sid END) AS first_sid
    FROM s
    GROUP BY user_start_tstamp;",
    snapshot_path = "select"
);
