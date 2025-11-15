use super::diesel_schema::sql_types::QueryStatusType;
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
#[diesel(sql_type = QueryStatusType)]
pub enum QueryStatus {
    Created,
    LimitExceeded,
    Queued,
    Running,
    Successful,
    Failed,
    Cancelled,
    TimedOut,
}

impl FromSql<QueryStatusType, Pg> for QueryStatus {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"created" => Ok(QueryStatus::Created),
            b"limit_exceeded" => Ok(QueryStatus::LimitExceeded),
            b"queued" => Ok(QueryStatus::Queued),
            b"running" => Ok(QueryStatus::Running),
            b"successful" => Ok(QueryStatus::Successful),
            b"failed" => Ok(QueryStatus::Failed),
            b"canceled" => Ok(QueryStatus::Cancelled),
            b"timed_out" => Ok(QueryStatus::TimedOut),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl ToSql<QueryStatusType, Pg> for QueryStatus {
    fn to_sql<'b>(&self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            QueryStatus::Created => out.write_all(b"created")?,
            QueryStatus::LimitExceeded => out.write_all(b"limit_exceeded")?,
            QueryStatus::Queued => out.write_all(b"queued")?,
            QueryStatus::Running => out.write_all(b"running")?,
            QueryStatus::Successful => out.write_all(b"successful")?,
            QueryStatus::Failed => out.write_all(b"failed")?,
            QueryStatus::Cancelled => out.write_all(b"canceled")?,
            QueryStatus::TimedOut => out.write_all(b"timed_out")?,
        }
        Ok(IsNull::No)
    }
}
