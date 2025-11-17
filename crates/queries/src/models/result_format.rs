use super::diesel_schema::sql_types::ResultFormatType;
use diesel::AsExpression;
use diesel::FromSqlRow;
use diesel::deserialize;
use diesel::deserialize::FromSql;
use diesel::pg::{Pg, PgValue};
use diesel::serialize;
use diesel::serialize::{IsNull, Output, ToSql};
use std::io::Write;

// Used reference following implementation:
// https://github.com/diesel-rs/diesel/blob/main/diesel_tests/tests/custom_types.rs

#[derive(AsExpression, Debug, Clone, Copy, FromSqlRow, Eq, PartialEq)]
#[diesel(sql_type = ResultFormatType)]
pub enum ResultFormat {
    Json,
    Arrow,
}

impl FromSql<ResultFormatType, Pg> for ResultFormat {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"json" => Ok(Self::Json),
            b"arrow" => Ok(Self::Arrow),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl ToSql<ResultFormatType, Pg> for ResultFormat {
    fn to_sql(&self, out: &mut Output<'_, '_, Pg>) -> serialize::Result {
        match *self {
            Self::Json => out.write_all(b"json")?,
            Self::Arrow => out.write_all(b"arrow")?,
        }
        Ok(IsNull::No)
    }
}
