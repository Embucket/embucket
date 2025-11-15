use diesel::AsExpression;
use diesel::FromSqlRow;
use diesel::backend::Backend;
use diesel::deserialize;
use diesel::deserialize::FromSql;
use diesel::serialize;
use diesel::serialize::Output;
use diesel::serialize::ToSql;
use diesel::sql_types::SmallInt;

#[repr(i16)]
#[derive(AsExpression, Debug, Clone, Copy, FromSqlRow, Eq, PartialEq)]
#[diesel(sql_type = SmallInt)]
pub enum QuerySource {
    SnowflakeRestApi = 1,
    UiRestApi = 2,
}

impl<DB> FromSql<SmallInt, DB> for QuerySource
where
    DB: Backend,
    i16: FromSql<SmallInt, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        match i16::from_sql(bytes)? {
            1 => Ok(QuerySource::SnowflakeRestApi),
            2 => Ok(QuerySource::UiRestApi),
            x => Err(format!("Unrecognized variant {}", x).into()),
        }
    }
}

impl<DB> ToSql<SmallInt, DB> for QuerySource
where
    DB: Backend,
    i16: ToSql<SmallInt, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        match self {
            QuerySource::SnowflakeRestApi => 1.to_sql(out),
            QuerySource::UiRestApi => 2.to_sql(out),
        }
    }
}
