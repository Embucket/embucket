use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel::SelectableHelper;
use diesel_async::RunQueryDsl;

use crate::error::{DieselSnafu, Result};
use crate::models::diesel_schema::queries;
use crate::models::query::Query;
use snafu::ResultExt;
use uuid::Uuid;

pub async fn create_query(conn: &mut crate::Connection, query: Query) -> Result<Query> {
    diesel::insert_into(queries::table)
        .values(query)
        .returning(Query::as_returning())
        .get_result(conn)
        .await
        .context(DieselSnafu)
}

pub async fn update_query(conn: &mut crate::Connection, query: Query) -> Result<Query> {
    diesel::update(queries::table.filter(queries::dsl::id.eq(query.id)))
        .set((
            // change only those fields that make sense
            // created_at was set on create
            queries::dsl::queued_at.eq(query.queued_at),
            queries::dsl::running_at.eq(query.running_at),
            queries::dsl::successful_at.eq(query.successful_at),
            queries::dsl::failed_at.eq(query.failed_at),
            queries::dsl::cancelled_at.eq(query.cancelled_at),
            queries::dsl::timedout_at.eq(query.timedout_at),
            queries::dsl::duration_ms.eq(query.duration_ms),
            queries::dsl::rows_count.eq(query.rows_count),
            queries::dsl::result_format.eq(query.result_format),
            queries::dsl::status.eq(query.status),
            queries::dsl::error.eq(query.error),
        ))
        .returning(Query::as_returning())
        .get_result(conn)
        .await
        .context(DieselSnafu)
}

pub async fn delete_query(conn: &mut crate::Connection, id: Uuid) -> Result<usize> {
    diesel::delete(queries::table.filter(queries::dsl::id.eq(id)))
        .execute(conn)
        .await
        .context(DieselSnafu)
}
