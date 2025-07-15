use crate::test_query;

test_query!(
    to_decimal_basic,
    "SELECT column1,
       TO_NUMBER(column1),
       TO_NUMBER(column1, 10, 1),
       TO_NUMBER(column1, 10, 8)
    FROM VALUES ('12.3456'), ('98.76546')",
    snapshot_path = "to_decimal"
);

test_query!(
    to_decimal_null,
    "SELECT column1, TRY_TO_NUMBER(column1, 10, 9)
    FROM VALUES ('12.3456'), ('98.76546')",
    snapshot_path = "to_decimal"
);

test_query!(
    to_decimal_format,
    "SELECT column1,
       TO_DECIMAL(column1, '99.9') as D0,
       TO_DECIMAL(column1, '99.9', 9, 5) as D5,
       TO_DECIMAL(column1, 'TM9', 9, 5) as TD5
    FROM VALUES ('1.0'), ('-12.3'), ('0.0'), ('- 0.1');",
    snapshot_path = "to_decimal"
);

test_query!(
    to_decimal_format_comma,
    "SELECT column1,
       TO_DECIMAL(column1, '9,999.99', 6, 2) as convert_number
    FROM VALUES ('3,741.72')",
    snapshot_path = "to_decimal"
);

test_query!(
    to_decimal_format_dollar_sign,
    "SELECT column1,
       TO_DECIMAL(column1, '$9,999.99', 6, 2) as convert_currency
    FROM VALUES ('$3,741.72')",
    snapshot_path = "to_decimal"
);
