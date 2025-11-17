use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel::SelectableHelper;
use diesel::TextExpressionMethods;
use diesel_async::RunQueryDsl;

use crate::Connection;
use crate::error::{DieselSnafu, Result};
use crate::models::diesel_schema::queries;
use crate::models::{Query, QueryStatus};
use crate::{FilterBy, ListParams, OrderBy, OrderDirection};
use snafu::ResultExt;
use uuid::Uuid;

macro_rules! order_by {
    ($query:expr, $direction:ident, $field:expr) => {
        match $direction {
            OrderDirection::Desc => $query.order($field.desc()),
            OrderDirection::Asc => $query.order($field.asc()),
        }
    };
}

pub async fn create_query(conn: &mut Connection, query: Query) -> Result<Query> {
    diesel::insert_into(queries::table)
        .values(query)
        .returning(Query::as_returning())
        .get_result(conn)
        .await
        .context(DieselSnafu)
}

pub async fn update_query(conn: &mut Connection, query: Query) -> Result<Query> {
    diesel::update(queries::table.filter(queries::dsl::id.eq(query.id)))
        .set((
            // change only those fields that make sense
            // created_at was set on create
            queries::dsl::queued_at.eq(query.queued_at),
            queries::dsl::running_at.eq(query.running_at),
            queries::dsl::finished_at.eq(query.finished_at),
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

pub async fn delete_query(conn: &mut Connection, id: Uuid) -> Result<usize> {
    diesel::delete(queries::table.filter(queries::dsl::id.eq(id)))
        .execute(conn)
        .await
        .context(DieselSnafu)
}

pub async fn list_queries(conn: &mut Connection, params: ListParams) -> Result<Vec<Query>> {
    // map params to orm request in other way
    let mut query = queries::table.select(Query::as_select()).into_boxed();

    for filter_by in params.filter_by {
        query = match filter_by {
            FilterBy::Status(status) => query.filter(queries::status.eq(status)),
            FilterBy::Source(source) => query.filter(queries::source.eq(source)),
            FilterBy::Format(format) => query.filter(queries::result_format.eq(format)),
            FilterBy::Sql(sql) => query.filter(queries::sql.like(format!("%{sql}%"))),
            FilterBy::Error(error) => query.filter(queries::error.like(format!("%{error}%"))),
        };
    }

    for order_by in params.order_by {
        query = match order_by {
            OrderBy::Status(direction) => {
                order_by!(query, direction, queries::status)
            }
            OrderBy::Source(direction) => {
                order_by!(query, direction, queries::source)
            }
            OrderBy::Format(direction) => {
                order_by!(query, direction, queries::result_format)
            }
            OrderBy::Timestamp(direction, status) => match status {
                QueryStatus::Created => order_by!(query, direction, queries::created_at),
                QueryStatus::Queued => order_by!(query, direction, queries::queued_at),
                QueryStatus::Running => order_by!(query, direction, queries::running_at),
                QueryStatus::LimitExceeded
                | QueryStatus::Successful
                | QueryStatus::Failed
                | QueryStatus::Cancelled
                | QueryStatus::TimedOut => order_by!(query, direction, queries::finished_at),
            },
            OrderBy::Duration(direction) => {
                order_by!(query, direction, queries::duration_ms)
            }
            OrderBy::RowsCount(direction) => {
                order_by!(query, direction, queries::rows_count)
            }
            OrderBy::Error(direction) => {
                order_by!(query, direction, queries::error)
            }
        };
    }

    if let Some(offset) = params.offset {
        query = query.offset(offset);
    }

    if let Some(limit) = params.limit {
        query = query.limit(limit);
    }

    query.load::<Query>(conn).await.context(DieselSnafu)
}
