use crate::error::Result;
use crate::models::QueryContext;
use crate::service::{CoreExecutionService, ExecutionService};
use crate::session::UserSession;
use crate::utils::Config;
use crate::{QueryResult, SessionMetadata};
use catalog_metastore::InMemoryMetastore;
use catalog_metastore::metastore_bootstrap_config::MetastoreBootstrapConfig;
use insta::assert_json_snapshot;
use state_store::{MockStateStore, Query, SessionRecord};
use std::sync::Arc;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

// Run unittests:
// cargo test --workspace --lib --features=state-store-query-test tests::statestore_queries_unittest

const TEST_SESSION_ID: &str = "test_session_id";
const TEST_DATABASE: &str = "test_database";
const TEST_SCHEMA: &str = "test_schema";
const TEST_TIMESTAMP: u64 = 1_764_161_275_445;

const MOCK_RELATED_TIMEOUT_DURATION: Duration = Duration::from_millis(100);

// Note: Run mocked async function with timeout.
// In case if mocked_function.withf() not returning true then entire test stucks.

pub struct TestStateStore;

#[must_use]
fn insta_settings(name: &str) -> insta::Settings {
    let mut settings = insta::Settings::new();
    settings.set_sort_maps(true);
    settings.set_description(name);
    settings.add_redaction(".execution_time", "1");
    settings.add_redaction(".query_metrics", "[query_metrics]");
    settings.add_filter(
        r"[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}",
        "00000000-0000-0000-0000-000000000000",
    );
    settings.add_filter(
        r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{9}Z",
        "2026-01-01T01:01:01.000000001Z",
    );
    settings.add_filter(
        r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{6}Z",
        "2026-01-01T01:01:01.000001Z",
    );
    settings
}

pub struct Mocker;

impl Mocker {
    pub fn apply_bypass_queries_mock(state_store_mock: &mut MockStateStore, count: usize) {
        state_store_mock
            .expect_put_query()
            .times(count)
            .returning(|_| Ok(()));
        state_store_mock
            .expect_update_query()
            .times(count)
            .returning(|_| Ok(()));
    }

    pub fn apply_bypass_put_queries_only_mock(state_store_mock: &mut MockStateStore, count: usize) {
        state_store_mock
            .expect_put_query()
            .times(count)
            .returning(|_| Ok(()));
    }

    pub fn apply_create_session_mock(
        state_store_mock: &mut MockStateStore,
        f: fn(&str) -> state_store::Result<SessionRecord>,
    ) {
        state_store_mock
            .expect_put_new_session()
            .returning(|_| Ok(()));
        state_store_mock.expect_put_session().returning(|_| Ok(()));
        state_store_mock.expect_get_session().returning(f);
    }

    pub async fn create_session(
        executor: Arc<dyn ExecutionService>,
        session_id: &str,
    ) -> Result<Arc<UserSession>> {
        timeout(
            MOCK_RELATED_TIMEOUT_DURATION,
            executor.create_session(session_id),
        )
        .await
        .expect("Create session timed out")
    }

    pub async fn query(
        executor: Arc<dyn ExecutionService>,
        session_id: &str,
        query_context: QueryContext,
        sql: &str,
    ) -> Result<QueryResult> {
        timeout(
            MOCK_RELATED_TIMEOUT_DURATION,
            executor.query(session_id, sql, query_context.clone()),
        )
        .await
        .expect("Query timed out")
    }
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_ok_query() {
    let mut state_store_mock = MockStateStore::new();
    Mocker::apply_create_session_mock(&mut state_store_mock, |_| {
        Ok(SessionRecord::new(TEST_SESSION_ID))
    });
    Mocker::apply_bypass_queries_mock(&mut state_store_mock, 2);

    state_store_mock
        .expect_put_query()
        .times(1)
        .returning(|_| Ok(()))
        // check created query attributes only here (it is expected to be the same for any invocation)
        .withf(move |query: &Query| {
            insta_settings("ok_query_put").bind(|| {
                assert_json_snapshot!(query, @r#"
                {
                  "query_id": "00000000-0000-0000-0000-000000000000",
                  "request_id": "00000000-0000-0000-0000-000000000000",
                  "query_text": "SELECT 1 AS a, 2.0 AS b, '3' AS 'c'",
                  "session_id": "test_session_id",
                  "warehouse_type": "DEFAULT",
                  "execution_status": "Running",
                  "start_time": "2026-01-01T01:01:01.000000001Z",
                  "release_version": "test-version",
                  "query_hash": "1717924485430328356",
                  "query_hash_version": 1,
                  "user_database_name": "test_database",
                  "user_schema_name": "test_schema",
                  "client_app_id": "client_app_id",
                  "client_app_version": "1.0.0"
                }
                "#);
            });
            true
        });

    state_store_mock
        .expect_update_query()
        .times(1)
        .returning(|_| Ok(()))
        .withf(move |query: &Query| {
            insta_settings("ok_query_update").bind(|| {
                assert_json_snapshot!(query, @r#"
                {
                  "query_id": "00000000-0000-0000-0000-000000000000",
                  "request_id": "00000000-0000-0000-0000-000000000000",
                  "query_text": "SELECT 1 AS a, 2.0 AS b, '3' AS 'c'",
                  "session_id": "test_session_id",
                  "database_name": "embucket",
                  "schema_name": "public",
                  "query_type": "SELECT",
                  "warehouse_type": "DEFAULT",
                  "execution_status": "Success",
                  "start_time": "2026-01-01T01:01:01.000000001Z",
                  "end_time": "2026-01-01T01:01:01.000000001Z",
                  "rows_produced": 1,
                  "execution_time": "1",
                  "release_version": "test-version",
                  "query_hash": "1717924485430328356",
                  "query_hash_version": 1,
                  "user_database_name": "test_database",
                  "user_schema_name": "test_schema",
                  "query_metrics": "[query_metrics]",
                  "client_app_id": "client_app_id",
                  "client_app_version": "1.0.0"
                }
                "#);
            });
            true
        });

    let mut session_metadata = SessionMetadata::default();
    session_metadata.set_attr(
        crate::SessionMetadataAttr::ClientAppId,
        "client_app_id".to_string(),
    );
    session_metadata.set_attr(
        crate::SessionMetadataAttr::ClientAppVersion,
        "1.0.0".to_string(),
    );
    let ctx = QueryContext::new(
        Some("test_database".to_string()),
        Some("test_schema".to_string()),
        None,
    )
    .with_request_id(Uuid::default())
    .with_session_metadata(Some(session_metadata));

    let metastore = Arc::new(InMemoryMetastore::new());
    MetastoreBootstrapConfig::bootstrap()
        .apply(metastore.clone())
        .await
        .expect("Failed to bootstrap metastore");

    let ex: Arc<dyn ExecutionService> = Arc::new(
        CoreExecutionService::new_test_executor(
            metastore,
            Arc::new(state_store_mock),
            Arc::new(Config::default()),
        )
        .await
        .expect("Failed to create execution service"),
    );

    Mocker::create_session(ex.clone(), TEST_SESSION_ID)
        .await
        .expect("Failed to create session");

    Mocker::query(
        ex.clone(),
        TEST_SESSION_ID,
        ctx.clone(),
        "SET DATABASE = 'embucket'",
    )
    .await
    .expect("Query execution failed");

    Mocker::query(
        ex.clone(),
        TEST_SESSION_ID,
        ctx.clone(),
        "SET SCHEMA = 'public'",
    )
    .await
    .expect("Query execution failed");

    Mocker::query(
        ex.clone(),
        TEST_SESSION_ID,
        ctx.clone(),
        "SELECT 1 AS a, 2.0 AS b, '3' AS 'c'",
    )
    .await
    .expect("Query execution failed");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_explain_query() {
    let mut state_store_mock = MockStateStore::new();
    Mocker::apply_create_session_mock(&mut state_store_mock, |_| {
        Ok(SessionRecord::new(TEST_SESSION_ID))
    });
    Mocker::apply_bypass_put_queries_only_mock(&mut state_store_mock, 1);

    state_store_mock
        .expect_update_query()
        .times(1)
        .returning(|_| Ok(()))
        .withf(move |query: &Query| {
            insta_settings("explain_query_update").bind(|| {
                assert_json_snapshot!(query, @r#"
                {
                  "query_id": "00000000-0000-0000-0000-000000000000",
                  "request_id": "00000000-0000-0000-0000-000000000000",
                  "query_text": "EXPLAIN SELECT 1 AS a, 2.0 AS b, '3' AS 'c'",
                  "session_id": "test_session_id",
                  "database_name": "test_database",
                  "schema_name": "test_schema",
                  "query_type": "EXPLAIN",
                  "warehouse_type": "DEFAULT",
                  "execution_status": "Success",
                  "start_time": "2026-01-01T01:01:01.000000001Z",
                  "end_time": "2026-01-01T01:01:01.000000001Z",
                  "execution_time": "1",
                  "release_version": "test-version",
                  "query_hash": "1265703338911562377",
                  "query_hash_version": 1,
                  "user_database_name": "test_database",
                  "user_schema_name": "test_schema",
                  "query_metrics": "[query_metrics]",
                  "client_app_id": "client_app_id",
                  "client_app_version": "1.0.0"
                }
                "#);
            });
            true
        });

    let mut session_metadata = SessionMetadata::default();
    session_metadata.set_attr(
        crate::SessionMetadataAttr::ClientAppId,
        "client_app_id".to_string(),
    );
    session_metadata.set_attr(
        crate::SessionMetadataAttr::ClientAppVersion,
        "1.0.0".to_string(),
    );
    let ctx = QueryContext::new(
        Some("test_database".to_string()),
        Some("test_schema".to_string()),
        None,
    )
    .with_request_id(Uuid::default())
    .with_session_metadata(Some(session_metadata));

    let metastore = Arc::new(InMemoryMetastore::new());
    MetastoreBootstrapConfig::bootstrap()
        .apply(metastore.clone())
        .await
        .expect("Failed to bootstrap metastore");

    let ex: Arc<dyn ExecutionService> = Arc::new(
        CoreExecutionService::new_test_executor(
            metastore,
            Arc::new(state_store_mock),
            Arc::new(Config::default()),
        )
        .await
        .expect("Failed to create execution service"),
    );

    Mocker::create_session(ex.clone(), TEST_SESSION_ID)
        .await
        .expect("Failed to create session");

    Mocker::query(
        ex.clone(),
        TEST_SESSION_ID,
        ctx.clone(),
        "EXPLAIN SELECT 1 AS a, 2.0 AS b, '3' AS 'c'",
    )
    .await
    .expect("Query execution failed");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_ok_insert() {
    let mut state_store_mock = MockStateStore::new();
    Mocker::apply_create_session_mock(&mut state_store_mock, |_| {
        Ok(SessionRecord::new(TEST_SESSION_ID))
    });
    Mocker::apply_bypass_queries_mock(&mut state_store_mock, 1);
    Mocker::apply_bypass_put_queries_only_mock(&mut state_store_mock, 1);

    state_store_mock
        .expect_update_query()
        .times(1)
        .returning(|_| Ok(()))
        .withf(move |query: &Query| {
            insta_settings("ok_insert_update").bind(|| {
                assert_json_snapshot!(query, @r#"
                {
                  "query_id": "00000000-0000-0000-0000-000000000000",
                  "request_id": "00000000-0000-0000-0000-000000000000",
                  "query_text": "INSERT INTO embucket.public.table VALUES (1)",
                  "session_id": "test_session_id",
                  "database_name": "test_database",
                  "schema_name": "test_schema",
                  "query_type": "INSERT",
                  "warehouse_type": "DEFAULT",
                  "execution_status": "Success",
                  "start_time": "2026-01-01T01:01:01.000000001Z",
                  "end_time": "2026-01-01T01:01:01.000000001Z",
                  "rows_inserted": 1,
                  "execution_time": "1",
                  "release_version": "test-version",
                  "query_hash": "17856184221539895914",
                  "query_hash_version": 1,
                  "user_database_name": "test_database",
                  "user_schema_name": "test_schema",
                  "query_metrics": "[query_metrics]",
                  "query_submission_time": 1764161275445
                }
                "#);
            });
            true
        });

    let metastore = Arc::new(InMemoryMetastore::new());
    MetastoreBootstrapConfig::bootstrap()
        .apply(metastore.clone())
        .await
        .expect("Failed to bootstrap metastore");

    let ex: Arc<dyn ExecutionService> = Arc::new(
        CoreExecutionService::new_test_executor(
            metastore,
            Arc::new(state_store_mock),
            Arc::new(Config::default()),
        )
        .await
        .expect("Failed to create execution service"),
    );

    let ctx = QueryContext::new(
        Some(TEST_DATABASE.to_string()),
        Some(TEST_SCHEMA.to_string()),
        None,
    )
    .with_query_submission_time(Some(TEST_TIMESTAMP))
    .with_request_id(Uuid::default());

    Mocker::create_session(ex.clone(), TEST_SESSION_ID)
        .await
        .expect("Failed to create session");

    Mocker::query(
        ex.clone(),
        TEST_SESSION_ID,
        ctx.clone(),
        "create table if not exists embucket.public.table (id int)",
    )
    .await
    .expect("Query execution failed");

    Mocker::query(
        ex.clone(),
        TEST_SESSION_ID,
        ctx.clone(),
        "INSERT INTO embucket.public.table VALUES (1)",
    )
    .await
    .expect("Query execution failed");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_ok_update() {
    let mut state_store_mock = MockStateStore::new();
    Mocker::apply_create_session_mock(&mut state_store_mock, |_| {
        Ok(SessionRecord::new(TEST_SESSION_ID))
    });
    Mocker::apply_bypass_queries_mock(&mut state_store_mock, 1);
    Mocker::apply_bypass_put_queries_only_mock(&mut state_store_mock, 1);

    // verify 2nd update
    state_store_mock
        .expect_update_query()
        .times(1)
        .returning(|_| Ok(()))
        .withf(move |query: &Query| {
            insta_settings("ok_update_update").bind(|| {
                assert_json_snapshot!(query, @r#"
                {
                  "query_id": "00000000-0000-0000-0000-000000000000",
                  "request_id": "00000000-0000-0000-0000-000000000000",
                  "query_text": "UPDATE embucket.public.table SET name = 'John'",
                  "session_id": "test_session_id",
                  "database_name": "embucket",
                  "schema_name": "public",
                  "query_type": "UPDATE",
                  "warehouse_type": "DEFAULT",
                  "execution_status": "Success",
                  "start_time": "2026-01-01T01:01:01.000000001Z",
                  "end_time": "2026-01-01T01:01:01.000000001Z",
                  "execution_time": "1",
                  "release_version": "test-version",
                  "query_hash": "16763742305627145642",
                  "query_hash_version": 1,
                  "query_metrics": "[query_metrics]"
                }
                "#);
            });
            true
        });

    let ctx = QueryContext::default().with_request_id(Uuid::default());

    let metastore = Arc::new(InMemoryMetastore::new());
    MetastoreBootstrapConfig::bootstrap()
        .apply(metastore.clone())
        .await
        .expect("Failed to bootstrap metastore");

    let ex: Arc<dyn ExecutionService> = Arc::new(
        CoreExecutionService::new_test_executor(
            metastore,
            Arc::new(state_store_mock),
            Arc::new(Config::default()),
        )
        .await
        .expect("Failed to create execution service"),
    );

    Mocker::create_session(ex.clone(), TEST_SESSION_ID)
        .await
        .expect("Failed to create session");

    Mocker::query(
        ex.clone(),
        TEST_SESSION_ID,
        ctx.clone(),
        "CREATE TABLE embucket.public.table AS SELECT 
            id, 
            name, 
            RANDOM() AS random_value, 
            CURRENT_TIMESTAMP AS current_time
        FROM (VALUES 
            (1, 'Alice'),
            (2, 'Bob'),
            (3, 'Charlie'),
            (4, 'David')
        ) AS t(id, name);",
    )
    .await
    .expect("Query execution failed");

    Mocker::query(
        ex.clone(),
        TEST_SESSION_ID,
        ctx.clone(),
        "UPDATE embucket.public.table SET name = 'John'",
    )
    .await
    .expect("Query execution failed");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_delete_failed() {
    let mut state_store_mock = MockStateStore::new();
    Mocker::apply_create_session_mock(&mut state_store_mock, |_| {
        Ok(SessionRecord::new(TEST_SESSION_ID))
    });
    Mocker::apply_bypass_queries_mock(&mut state_store_mock, 1);
    Mocker::apply_bypass_put_queries_only_mock(&mut state_store_mock, 1);

    state_store_mock
        .expect_update_query()
        .times(1)
        .returning(|_| Ok(()))
        .withf(move |query: &Query| {
            insta_settings("ok_truncate_update").bind(|| {
                assert_json_snapshot!(query, @r#"
                {
                  "query_id": "00000000-0000-0000-0000-000000000000",
                  "request_id": "00000000-0000-0000-0000-000000000000",
                  "query_text": "DELETE FROM embucket.public.table",
                  "session_id": "test_session_id",
                  "database_name": "embucket",
                  "schema_name": "public",
                  "query_type": "DELETE",
                  "warehouse_type": "DEFAULT",
                  "execution_status": "Fail",
                  "error_code": "010001",
                  "error_message": "00000000-0000-0000-0000-000000000000: Query execution error: DataFusion error: This feature is not implemented: Unsupported logical plan: Dml(Delete)",
                  "start_time": "2026-01-01T01:01:01.000000001Z",
                  "end_time": "2026-01-01T01:01:01.000000001Z",
                  "execution_time": "1",
                  "release_version": "test-version",
                  "query_hash": "13652442282618196356",
                  "query_hash_version": 1
                }
                "#);
            });
            true
        });

    let ctx = QueryContext::default().with_request_id(Uuid::default());

    let metastore = Arc::new(InMemoryMetastore::new());
    MetastoreBootstrapConfig::bootstrap()
        .apply(metastore.clone())
        .await
        .expect("Failed to bootstrap metastore");

    let ex: Arc<dyn ExecutionService> = Arc::new(
        CoreExecutionService::new_test_executor(
            metastore,
            Arc::new(state_store_mock),
            Arc::new(Config::default()),
        )
        .await
        .expect("Failed to create execution service"),
    );

    Mocker::create_session(ex.clone(), TEST_SESSION_ID)
        .await
        .expect("Failed to create session");

    Mocker::query(
        ex.clone(),
        TEST_SESSION_ID,
        ctx.clone(),
        "CREATE TABLE embucket.public.table AS SELECT 
            id, 
            name, 
            RANDOM() AS random_value, 
            CURRENT_TIMESTAMP AS current_time
        FROM (VALUES 
            (1, 'Alice'),
            (2, 'Bob'),
            (3, 'Charlie'),
            (4, 'David')
        ) AS t(id, name);",
    )
    .await
    .expect("Query execution failed");

    Mocker::query(
        ex.clone(),
        TEST_SESSION_ID,
        ctx.clone(),
        "DELETE FROM embucket.public.table",
    )
    .await
    .expect_err("Query expected to fail");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_ok_truncate() {
    let mut state_store_mock = MockStateStore::new();
    Mocker::apply_create_session_mock(&mut state_store_mock, |_| {
        Ok(SessionRecord::new(TEST_SESSION_ID))
    });
    Mocker::apply_bypass_queries_mock(&mut state_store_mock, 1);
    Mocker::apply_bypass_put_queries_only_mock(&mut state_store_mock, 1);

    state_store_mock
        .expect_update_query()
        .times(1)
        .returning(|_| Ok(()))
        .withf(move |query: &Query| {
            insta_settings("ok_truncate_update").bind(|| {
                assert_json_snapshot!(query, @r#"
                {
                  "query_id": "00000000-0000-0000-0000-000000000000",
                  "request_id": "00000000-0000-0000-0000-000000000000",
                  "query_text": "TRUNCATE TABLE embucket.public.table",
                  "session_id": "test_session_id",
                  "database_name": "embucket",
                  "schema_name": "public",
                  "query_type": "TRUNCATE",
                  "warehouse_type": "DEFAULT",
                  "execution_status": "Success",
                  "start_time": "2026-01-01T01:01:01.000000001Z",
                  "end_time": "2026-01-01T01:01:01.000000001Z",
                  "rows_deleted": 0,
                  "execution_time": "1",
                  "release_version": "test-version",
                  "query_hash": "16187825059241168947",
                  "query_hash_version": 1,
                  "query_metrics": "[query_metrics]"
                }
                "#);
            });
            true
        });

    let ctx = QueryContext::default().with_request_id(Uuid::default());

    let metastore = Arc::new(InMemoryMetastore::new());
    MetastoreBootstrapConfig::bootstrap()
        .apply(metastore.clone())
        .await
        .expect("Failed to bootstrap metastore");

    let ex: Arc<dyn ExecutionService> = Arc::new(
        CoreExecutionService::new_test_executor(
            metastore,
            Arc::new(state_store_mock),
            Arc::new(Config::default()),
        )
        .await
        .expect("Failed to create execution service"),
    );

    Mocker::create_session(ex.clone(), TEST_SESSION_ID)
        .await
        .expect("Failed to create session");

    Mocker::query(
        ex.clone(),
        TEST_SESSION_ID,
        ctx.clone(),
        "CREATE TABLE embucket.public.table AS SELECT 
            id, 
            name, 
            RANDOM() AS random_value, 
            CURRENT_TIMESTAMP AS current_time
        FROM (VALUES 
            (1, 'Alice'),
            (2, 'Bob'),
            (3, 'Charlie'),
            (4, 'David')
        ) AS t(id, name);",
    )
    .await
    .expect("Query execution failed");

    Mocker::query(
        ex.clone(),
        TEST_SESSION_ID,
        ctx.clone(),
        "TRUNCATE TABLE embucket.public.table",
    )
    .await
    .expect("Query execution failed");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_ok_merge() {
    let mut state_store_mock = MockStateStore::new();
    Mocker::apply_create_session_mock(&mut state_store_mock, |_| {
        Ok(SessionRecord::new(TEST_SESSION_ID))
    });
    Mocker::apply_bypass_queries_mock(&mut state_store_mock, 2);
    Mocker::apply_bypass_put_queries_only_mock(&mut state_store_mock, 1);

    // verify 3rd update
    state_store_mock
        .expect_update_query()
        .times(1)
        .returning(|_| Ok(()))
        .withf(move |query: &Query| {
            insta_settings("ok_merge_update").bind(|| {
                assert_json_snapshot!(query, @r#"
                {
                  "query_id": "00000000-0000-0000-0000-000000000000",
                  "request_id": "00000000-0000-0000-0000-000000000000",
                  "query_text": "MERGE INTO t1 USING \n        (SELECT * FROM t2) AS t2 \n        ON t1.a = t2.a \n        WHEN MATCHED THEN UPDATE SET t1.c = t2.c \n        WHEN NOT MATCHED THEN INSERT (a,c) VALUES(t2.a,t2.c)",
                  "session_id": "test_session_id",
                  "database_name": "embucket",
                  "schema_name": "public",
                  "query_type": "MERGE",
                  "warehouse_type": "DEFAULT",
                  "execution_status": "Success",
                  "start_time": "2026-01-01T01:01:01.000000001Z",
                  "end_time": "2026-01-01T01:01:01.000000001Z",
                  "rows_produced": 4,
                  "rows_inserted": 1,
                  "execution_time": "1",
                  "release_version": "test-version",
                  "query_hash": "10180476120311618623",
                  "query_hash_version": 1,
                  "query_metrics": "[query_metrics]"
                }
                "#);
            });
            true
        });

    let ctx = QueryContext::default().with_request_id(Uuid::default());

    let metastore = Arc::new(InMemoryMetastore::new());
    MetastoreBootstrapConfig::bootstrap()
        .apply(metastore.clone())
        .await
        .expect("Failed to bootstrap metastore");

    let ex: Arc<dyn ExecutionService> = Arc::new(
        CoreExecutionService::new_test_executor(
            metastore,
            Arc::new(state_store_mock),
            Arc::new(Config::default()),
        )
        .await
        .expect("Failed to create execution service"),
    );

    Mocker::create_session(ex.clone(), TEST_SESSION_ID)
        .await
        .expect("Failed to create session");

    Mocker::query(
        ex.clone(),
        TEST_SESSION_ID,
        ctx.clone(),
        "CREATE TABLE embucket.public.t1 AS SELECT 
        a,b,c
        FROM (VALUES 
            (1,'b1','c1'),
            (2,'b2','c2'),
            (2,'b3','c3'),
            (3,'b4','c4')
        ) AS t(a, b, c);",
    )
    .await
    .expect("Query execution failed");

    Mocker::query(
        ex.clone(),
        TEST_SESSION_ID,
        ctx.clone(),
        "CREATE TABLE embucket.public.t2 AS SELECT
        a,b,c
        FROM (VALUES 
            (1,'b_5','c_5'),
            (3,'b_6','c_6'),
            (2,'b_7','c_7'),
            (4,'b_8','c_8')
        ) AS t(a, b, c);",
    )
    .await
    .expect("Query execution failed");

    Mocker::query(
        ex.clone(),
        TEST_SESSION_ID,
        ctx.clone(),
        "MERGE INTO t1 USING 
        (SELECT * FROM t2) AS t2 
        ON t1.a = t2.a 
        WHEN MATCHED THEN UPDATE SET t1.c = t2.c 
        WHEN NOT MATCHED THEN INSERT (a,c) VALUES(t2.a,t2.c)",
    )
    .await
    .expect("Query execution failed");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_query_status_incident_limit_exceeded() {
    let mut state_store_mock = MockStateStore::new();
    Mocker::apply_create_session_mock(&mut state_store_mock, |_| {
        Ok(SessionRecord::new(TEST_SESSION_ID))
    });

    state_store_mock.expect_put_query()
        .returning(|_| Ok(()) )
        .times(1)
        // check created query attributes only here (it is expected to be the same for any invocation)
        .withf(move |query: &Query| {
            insta_settings("incident_query_put").bind(|| {
                assert_json_snapshot!(query, @r#"
                {
                  "query_id": "00000000-0000-0000-0000-000000000000",
                  "request_id": "00000000-0000-0000-0000-000000000000",
                  "query_text": "SELECT 1",
                  "session_id": "test_session_id",
                  "warehouse_type": "DEFAULT",
                  "execution_status": "Incident",
                  "error_code": "010001",
                  "error_message": "00000000-0000-0000-0000-000000000000: Query execution error: Concurrency limit reached — too many concurrent queries are running",
                  "start_time": "2026-01-01T01:01:01.000000001Z",
                  "end_time": "2026-01-01T01:01:01.000000001Z",
                  "execution_time": "1",
                  "release_version": "test-version",
                  "query_hash": "8436521302113462945",
                  "query_hash_version": 1,
                  "user_database_name": "test_database",
                  "user_schema_name": "test_schema"
                }
                "#);
            });
            true
        });

    let ctx = QueryContext::new(
        Some(TEST_DATABASE.to_string()),
        Some(TEST_SCHEMA.to_string()),
        None,
    )
    .with_request_id(Uuid::default());

    let metastore = Arc::new(InMemoryMetastore::new());
    MetastoreBootstrapConfig::bootstrap()
        .apply(metastore.clone())
        .await
        .expect("Failed to bootstrap metastore");

    let ex: Arc<dyn ExecutionService> = Arc::new(
        CoreExecutionService::new_test_executor(
            metastore,
            Arc::new(state_store_mock),
            Arc::new(Config::default().with_max_concurrency_level(0)),
        )
        .await
        .expect("Failed to create execution service"),
    );

    Mocker::create_session(ex.clone(), TEST_SESSION_ID)
        .await
        .expect("Failed to create session");

    Mocker::query(ex.clone(), TEST_SESSION_ID, ctx.clone(), "SELECT 1")
        .await
        .expect_err("Query execution should fail");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_query_status_fail() {
    let mut state_store_mock = MockStateStore::new();
    Mocker::apply_create_session_mock(&mut state_store_mock, |_| {
        Ok(SessionRecord::new(TEST_SESSION_ID))
    });

    state_store_mock
        .expect_put_query()
        .times(1)
        .returning(|_| Ok(()))
        .withf(|query: &Query| {
            insta_settings("fail_query_put").bind(|| {
                assert_json_snapshot!(query, @r#"
                {
                  "query_id": "00000000-0000-0000-0000-000000000000",
                  "request_id": "00000000-0000-0000-0000-000000000000",
                  "query_text": "SELECT should fail",
                  "session_id": "test_session_id",
                  "warehouse_type": "DEFAULT",
                  "execution_status": "Running",
                  "start_time": "2026-01-01T01:01:01.000000001Z",
                  "release_version": "test-version",
                  "query_hash": "17999132521915915058",
                  "query_hash_version": 1
                }
                "#);
            });
            true
        });
    state_store_mock.expect_update_query()
        .times(1)
        .returning(|_| Ok(()) )
        .withf(|query: &Query| {
            insta_settings("fail_query_update").bind(|| {
                assert_json_snapshot!(query, @r#"
                {
                  "query_id": "00000000-0000-0000-0000-000000000000",
                  "request_id": "00000000-0000-0000-0000-000000000000",
                  "query_text": "SELECT should fail",
                  "session_id": "test_session_id",
                  "database_name": "embucket",
                  "schema_name": "public",
                  "query_type": "SELECT",
                  "warehouse_type": "DEFAULT",
                  "execution_status": "Fail",
                  "error_code": "002003",
                  "error_message": "00000000-0000-0000-0000-000000000000: Query execution error: DataFusion error: Schema error: No field named should.",
                  "start_time": "2026-01-01T01:01:01.000000001Z",
                  "end_time": "2026-01-01T01:01:01.000000001Z",
                  "execution_time": "1",
                  "release_version": "test-version",
                  "query_hash": "17999132521915915058",
                  "query_hash_version": 1
                }
                "#);
            });
            true
        });

    let ctx = QueryContext::default().with_request_id(Uuid::new_v4());

    let metastore = Arc::new(InMemoryMetastore::new());
    MetastoreBootstrapConfig::bootstrap()
        .apply(metastore.clone())
        .await
        .expect("Failed to bootstrap metastore");

    let ex: Arc<dyn ExecutionService> = Arc::new(
        CoreExecutionService::new_test_executor(
            metastore,
            Arc::new(state_store_mock),
            Arc::new(Config::default()),
        )
        .await
        .expect("Failed to create execution service"),
    );

    Mocker::create_session(ex.clone(), TEST_SESSION_ID)
        .await
        .expect("Failed to create session");

    Mocker::query(
        ex.clone(),
        TEST_SESSION_ID,
        ctx.clone(),
        "SELECT should fail",
    )
    .await
    .expect_err("Query execution should fail");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_query_status_cancelled() {
    let mut state_store_mock = MockStateStore::new();
    Mocker::apply_create_session_mock(&mut state_store_mock, |_| {
        Ok(SessionRecord::new(TEST_SESSION_ID))
    });

    state_store_mock
        .expect_put_query()
        .times(1)
        .returning(|_| Ok(()))
        .withf(|query: &Query| {
            insta_settings("cancelled_query_put").bind(|| {
                assert_json_snapshot!(query, @r#"
                {
                  "query_id": "00000000-0000-0000-0000-000000000000",
                  "request_id": "00000000-0000-0000-0000-000000000000",
                  "query_text": "SELECT 1",
                  "session_id": "test_session_id",
                  "warehouse_type": "DEFAULT",
                  "execution_status": "Running",
                  "start_time": "2026-01-01T01:01:01.000000001Z",
                  "release_version": "test-version",
                  "query_hash": "8436521302113462945",
                  "query_hash_version": 1
                }
                "#);
            });
            true
        });
    state_store_mock.expect_update_query()
        .times(1)
        .returning(|_| Ok(()) )
        .withf(|query: &Query| {
            insta_settings("cancelled_query_update").bind(|| {
                assert_json_snapshot!(query, @r#"
                {
                  "query_id": "00000000-0000-0000-0000-000000000000",
                  "request_id": "00000000-0000-0000-0000-000000000000",
                  "query_text": "SELECT 1",
                  "session_id": "test_session_id",
                  "database_name": "embucket",
                  "schema_name": "public",
                  "warehouse_type": "DEFAULT",
                  "execution_status": "Fail",
                  "error_code": "000684",
                  "error_message": "00000000-0000-0000-0000-000000000000: Query execution error: Query 00000000-0000-0000-0000-000000000000 cancelled",
                  "start_time": "2026-01-01T01:01:01.000000001Z",
                  "end_time": "2026-01-01T01:01:01.000000001Z",
                  "execution_time": "1",
                  "release_version": "test-version",
                  "query_hash": "8436521302113462945",
                  "query_hash_version": 1
                }
                "#);
            });
            true
        });

    let ctx = QueryContext::default().with_request_id(Uuid::default());

    let metastore = Arc::new(InMemoryMetastore::new());
    MetastoreBootstrapConfig::bootstrap()
        .apply(metastore.clone())
        .await
        .expect("Failed to bootstrap metastore");

    let ex: Arc<dyn ExecutionService> = Arc::new(
        CoreExecutionService::new_test_executor(
            metastore,
            Arc::new(state_store_mock),
            Arc::new(Config::default()),
        )
        .await
        .expect("Failed to create execution service"),
    );

    Mocker::create_session(ex.clone(), TEST_SESSION_ID)
        .await
        .expect("Failed to create session");

    // See note about timeout above
    let query_handle = timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        ex.submit(TEST_SESSION_ID, "SELECT 1", ctx),
    )
    .await
    .expect("Query timed out")
    .expect("Query submit error");

    let _ = timeout(MOCK_RELATED_TIMEOUT_DURATION, ex.abort(query_handle))
        .await
        .expect("Query timed out")
        .expect("Failed to cancel query");
}
