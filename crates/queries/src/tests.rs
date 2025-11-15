use std::sync::Arc;
use uuid::Uuid;

use crate::Connection;
use crate::{Queries, QueriesDb, Query, QuerySource, ResultFormat};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::{AsyncPgConnection, SimpleAsyncConnection};

fn create_pool(uri: &str) -> Pool<AsyncPgConnection> {
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(uri);
    Pool::builder(config)
        .build()
        .expect("Failed to create pool")
}

fn create_test_pool() -> Pool<AsyncPgConnection> {
    create_pool("postgres://dev:dev@localhost:5432/dev")
}

async fn create_test_schema(conn: &mut Connection, test_name: &str) {
    conn.batch_execute(
        format!(
            "
        DROP SCHEMA IF EXISTS {test_name};
        CREATE SCHEMA {test_name};
        SET search_path TO {test_name};
    "
        )
        .as_str(),
    )
    .await
    .expect("Failed to create test schema");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_queries_db() {
    let pool = create_test_pool();
    let mut conn = pool.get().await.expect("Failed to get connection");

    create_test_schema(&mut conn, "test_queries_db").await;

    let queries: Arc<dyn Queries> = Arc::new(
        QueriesDb::new(pool)
            .await
            .expect("Failed to create queries db"),
    );

    let request_metadata = serde_json::json!({"key": "value"});

    let mut item = Query::new(
        "SELECT 1".to_string(),
        QuerySource::SnowflakeRestApi,
        ResultFormat::Json,
    )
    .with_request_id(Uuid::new_v4())
    .with_request_metadata(request_metadata.clone());
    let _ = queries
        .add(item.clone())
        .await
        .expect("Failed to add query");

    item.set_successful(1);   

    let updated_query = queries
        .update(item.clone())
        .await
        .expect("Failed to update query");
    // timestamps not equal after loading from db, as postgres preserves just microseconds
    assert_ne!(updated_query.successful_at, item.successful_at);
    assert_eq!(updated_query.successful_at.map(|t|t.timestamp_micros()), item.successful_at.map(|t|t.timestamp_micros()));
    // check rest of the fields
    assert_eq!(updated_query.sql, item.sql);
    assert_eq!(updated_query.source, item.source);
    assert_eq!(updated_query.result_format, item.result_format);
    assert_eq!(updated_query.request_id, item.request_id);
    assert_eq!(updated_query.request_metadata, request_metadata);
    assert_ne!(updated_query.created_at, updated_query.successful_at.unwrap());
    assert_eq!(updated_query.rows_count, 1);
    assert!(updated_query.failed_at.is_none());
    assert!(updated_query.duration_ms > 0);

    let deleted = queries.delete(item.id).await.expect("Failed to delete query");
    assert_eq!(deleted, 1);
}
