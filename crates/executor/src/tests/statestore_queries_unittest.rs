use crate::models::QueryContext;
use crate::service::{CoreExecutionService, ExecutionService};
use crate::utils::Config;
use catalog_metastore::InMemoryMetastore;
use insta::assert_json_snapshot;
use state_store::{MockStateStore, Query, SessionRecord, StateStore};
use std::sync::Arc;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

// Run unittests:
// cargo test --workspace --lib --features=state-store-query-test tests::statestore_queries_unittest

const TEST_SESSION_ID: &str = "test_session_id";
const TEST_DATABASE: &str = "test_database";
const TEST_SCHEMA: &str = "test_schema";
const OK_QUERY_TEXT: &str = "SELECT 1 AS a, 2.0 AS b, '3' AS c WHERE False";

const MOCK_RELATED_TIMEOUT_DURATION: Duration = Duration::from_millis(100);

// Note: Run mocked async function with timeout.
// In case if mocked_function.withf() not returning true then entire test stucks.

pub struct TestStateStore;

#[must_use]
fn insta_settings(name: &str) -> insta::Settings {
    let mut settings = insta::Settings::new();
    settings.set_sort_maps(true);
    settings.set_description(name);
    settings.set_info(&format!("{name}"));
    settings.add_redaction(".execution_time", "1");
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

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_ok_query() {
    let query_context = QueryContext::default().with_request_id(Uuid::default());

    let mut state_store_mock = MockStateStore::new();
    state_store_mock
        .expect_put_new_session()
        .returning(|_| Ok(()));
    state_store_mock
        .expect_get_session()
        .returning(|_| Ok(SessionRecord::new(TEST_SESSION_ID)));
    state_store_mock
        .expect_put_query()
        .returning(|_| Ok(()))
        // check created query attributes only here (it is expected to be the same for any invocation)
        .withf(move |query: &Query| {
            insta_settings("ok_query_put").bind(|| {
                assert_json_snapshot!(query, @r#"
                {
                  "query_id": "00000000-0000-0000-0000-000000000000",
                  "request_id": "00000000-0000-0000-0000-000000000000",
                  "query_text": "SELECT 1 AS a, 2.0 AS b, '3' AS c WHERE False",
                  "session_id": "test_session_id",
                  "warehouse_type": "DEFAULT",
                  "execution_status": "Running",
                  "start_time": "2026-01-01T01:01:01.000000001Z",
                  "query_hash": "12320374230549905548",
                  "query_hash_version": 1
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
                  "query_text": "SELECT 1 AS a, 2.0 AS b, '3' AS c WHERE False",
                  "session_id": "test_session_id",
                  "database_name": "embucket",
                  "schema_name": "public",
                  "warehouse_type": "DEFAULT",
                  "execution_status": "Success",
                  "start_time": "2026-01-01T01:01:01.000000001Z",
                  "end_time": "2026-01-01T01:01:01.000000001Z",
                  "execution_time": "1",
                  "query_hash": "12320374230549905548",
                  "query_hash_version": 1,
                  "query_metrics": [
                    {
                      "node_id": 0,
                      "parent_node_id": null,
                      "operator": "EmptyExec",
                      "metrics": []
                    }
                  ]
                }
                "#);
            });
            true
        });

    let state_store: Arc<dyn StateStore> = Arc::new(state_store_mock);

    let metastore = Arc::new(InMemoryMetastore::new());
    let execution_svc = CoreExecutionService::new_test_executor(
        metastore,
        state_store,
        Arc::new(Config::default()),
    )
    .await
    .expect("Failed to create execution service");

    execution_svc
        .create_session(TEST_SESSION_ID)
        .await
        .expect("Failed to create session");

    // See note about timeout above
    let _ = timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.query(TEST_SESSION_ID, OK_QUERY_TEXT, query_context),
    )
    .await
    .expect("Query timed out")
    .expect("Query execution stopped by timeout");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_query_status_incident_limit_exceeded() {
    let query_context = QueryContext::new(
        Some(TEST_DATABASE.to_string()),
        Some(TEST_SCHEMA.to_string()),
        None,
    )
    .with_request_id(Uuid::default());

    let mut state_store_mock = MockStateStore::new();
    state_store_mock
        .expect_put_new_session()
        .returning(|_| Ok(()));
    state_store_mock
        .expect_get_session()
        .returning(|_| Ok(SessionRecord::new(TEST_SESSION_ID)));
    state_store_mock.expect_put_query()
        .returning(|_| Ok(()) )
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
                  "query_hash": "8436521302113462945",
                  "query_hash_version": 1
                }
                "#);
            });
            true
        });

    let state_store: Arc<dyn StateStore> = Arc::new(state_store_mock);

    let metastore = Arc::new(InMemoryMetastore::new());
    let execution_svc = CoreExecutionService::new_test_executor(
        metastore,
        state_store,
        Arc::new(Config::default().with_max_concurrency_level(0)),
    )
    .await
    .expect("Failed to create execution service");

    execution_svc
        .create_session(TEST_SESSION_ID)
        .await
        .expect("Failed to create session");

    // See note about timeout above
    let _ = timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.query(TEST_SESSION_ID, "SELECT 1", query_context),
    )
    .await
    .expect("Query timed out")
    .expect_err("Query execution should fail");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_query_status_fail() {
    let mut state_store_mock = MockStateStore::new();
    state_store_mock
        .expect_put_new_session()
        .returning(|_| Ok(()));
    state_store_mock
        .expect_get_session()
        .returning(|_| Ok(SessionRecord::new(TEST_SESSION_ID)));
    state_store_mock
        .expect_put_query()
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
                  "warehouse_type": "DEFAULT",
                  "execution_status": "Fail",
                  "error_code": "002003",
                  "error_message": "00000000-0000-0000-0000-000000000000: Query execution error: DataFusion error: Schema error: No field named should.",
                  "start_time": "2026-01-01T01:01:01.000000001Z",
                  "end_time": "2026-01-01T01:01:01.000000001Z",
                  "execution_time": "1",
                  "query_hash": "17999132521915915058",
                  "query_hash_version": 1
                }
                "#);
            });
            true
        });

    let state_store: Arc<dyn StateStore> = Arc::new(state_store_mock);

    let metastore = Arc::new(InMemoryMetastore::new());
    let execution_svc = CoreExecutionService::new_test_executor(
        metastore,
        state_store,
        Arc::new(Config::default()),
    )
    .await
    .expect("Failed to create execution service");

    execution_svc
        .create_session(TEST_SESSION_ID)
        .await
        .expect("Failed to create session");

    // See note about timeout above
    let _ = timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.query(
            TEST_SESSION_ID,
            "SELECT should fail",
            QueryContext::default().with_request_id(Uuid::new_v4()),
        ),
    )
    .await
    .expect("Query timed out")
    .expect_err("Query execution should fail");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_query_status_cancelled() {
    let mut state_store_mock = MockStateStore::new();
    state_store_mock
        .expect_put_new_session()
        .returning(|_| Ok(()));
    state_store_mock
        .expect_get_session()
        .returning(|_| Ok(SessionRecord::new(TEST_SESSION_ID)));
    state_store_mock
        .expect_put_query()
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
                  "query_hash": "8436521302113462945",
                  "query_hash_version": 1
                }
                "#);
            });
            true
        });

    let state_store: Arc<dyn StateStore> = Arc::new(state_store_mock);

    let metastore = Arc::new(InMemoryMetastore::new());
    let execution_svc = CoreExecutionService::new_test_executor(
        metastore,
        state_store,
        Arc::new(Config::default()),
    )
    .await
    .expect("Failed to create execution service");

    execution_svc
        .create_session(TEST_SESSION_ID)
        .await
        .expect("Failed to create session");

    // See note about timeout above
    let query_handle = timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.submit(
            TEST_SESSION_ID,
            "SELECT 1",
            QueryContext::default().with_request_id(Uuid::new_v4()),
        ),
    )
    .await
    .expect("Query timed out")
    .expect("Query submit error");

    let _ = timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.abort(query_handle),
    )
    .await
    .expect("Query timed out")
    .expect("Failed to cancel query");
}
