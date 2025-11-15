use diesel::SelectableHelper;
use diesel_async::RunQueryDsl;
use crate::models::query::Query;
use crate::models::diesel_schema::queries;
use crate::error::{Result, DieselSnafu};
use snafu::ResultExt;

pub async fn create_query(
    conn: &mut crate::Connection,
    query: Query,
) -> Result<Query> {
    diesel::insert_into(queries::table)
        .values(query)
        .returning(Query::as_returning())
        .get_result(conn)
        .await
        .context(DieselSnafu)
}
