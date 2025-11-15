// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "query_status_type"))]
    pub struct QueryStatusType;

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "result_format_type"))]
    pub struct ResultFormatType;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::QueryStatusType;
    use super::sql_types::ResultFormatType;

    queries (id) {
        id -> Uuid,
        sql -> Text,
        status -> QueryStatusType,
        source -> Int2,
        result_format -> ResultFormatType,
        request_id -> Nullable<Uuid>,
        request_metadata -> Jsonb,
        created_at -> Timestamptz,
        limit_exceeded_at -> Nullable<Timestamptz>,
        queued_at -> Nullable<Timestamptz>,
        running_at -> Nullable<Timestamptz>,
        successful_at -> Nullable<Timestamptz>,
        failed_at -> Nullable<Timestamptz>,
        cancelled_at -> Nullable<Timestamptz>,
        timedout_at -> Nullable<Timestamptz>,
        duration_ms -> Int8,
        rows_count -> Int8,
        error -> Nullable<Text>,
    }
}
