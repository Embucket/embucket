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
