use crate::test_query;

test_query!(
    basic,
    "select json_get_float('[1]', 0)",
    snapshot_path = "json_get_float"
);
