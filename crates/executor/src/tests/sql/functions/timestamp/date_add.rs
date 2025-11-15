use crate::test_query;

test_query!(
    select_date_add_diff,
    "SELECT dateadd(day, 5, '2025-06-01')",
    snapshot_path = "date_add"
);
test_query!(
    func_date_add,
    "SELECT date_add(day, 30, '2025-01-06')",
    snapshot_path = "date_add"
);
