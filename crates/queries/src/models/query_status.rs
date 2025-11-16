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
    Queued,
    Running,
    LimitExceeded,
    Successful,
    Failed,
    Cancelled,
    TimedOut,
}

impl FromSql<QueryStatusType, Pg> for QueryStatus {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"created" => Ok(Self::Created),
            b"limit_exceeded" => Ok(Self::LimitExceeded),
            b"queued" => Ok(Self::Queued),
            b"running" => Ok(Self::Running),
            b"successful" => Ok(Self::Successful),
            b"failed" => Ok(Self::Failed),
            b"cancelled" => Ok(Self::Cancelled),
            b"timed_out" => Ok(Self::TimedOut),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl ToSql<QueryStatusType, Pg> for QueryStatus {
    fn to_sql(&self, out: &mut Output<'_, '_, Pg>) -> serialize::Result {
        match *self {
            Self::Created => out.write_all(b"created")?,
            Self::LimitExceeded => out.write_all(b"limit_exceeded")?,
            Self::Queued => out.write_all(b"queued")?,
            Self::Running => out.write_all(b"running")?,
            Self::Successful => out.write_all(b"successful")?,
            Self::Failed => out.write_all(b"failed")?,
            Self::Cancelled => out.write_all(b"cancelled")?,
            Self::TimedOut => out.write_all(b"timed_out")?,
        }
        Ok(IsNull::No)
    }
}
