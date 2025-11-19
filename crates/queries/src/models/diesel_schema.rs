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
    use diesel::sql_types::{Uuid, Text, Int2, Nullable, Jsonb, Timestamptz, Int8};
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
        queued_at -> Nullable<Timestamptz>,
        running_at -> Nullable<Timestamptz>,
        finished_at -> Nullable<Timestamptz>,
        duration_ms -> Int8,
        rows_count -> Int8,
        error -> Nullable<Text>,
    }
}
