use crate::models::QueryContext;
use crate::service::{CoreExecutionService, ExecutionService};
use crate::utils::Config;
use catalog_metastore::InMemoryMetastore;
use catalog_metastore::metastore_bootstrap_config::MetastoreBootstrapConfig;
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
                  "query_metrics": "[query_metrics]"
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

    timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.create_session(TEST_SESSION_ID),
    )
    .await
    .expect("Create session timed out")
    .expect("Failed to create session");

    // See note about timeout above
    let _ = timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.query(
            TEST_SESSION_ID,
            "SELECT 1 AS a, 2.0 AS b, '3' AS 'c'",
            query_context,
        ),
    )
    .await
    .expect("Query timed out")
    .expect("Query execution failed");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_ok_insert() {
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
        .times(2)
        .returning(|_| Ok(()));

    // bypass 1st update
    state_store_mock
        .expect_update_query()
        .times(1)
        .returning(|_| Ok(()));
    // verify 2nd update
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
                  "database_name": "embucket",
                  "schema_name": "public",
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
                  "query_metrics": "[query_metrics]"
                }
                "#);
            });
            true
        });

    let state_store: Arc<dyn StateStore> = Arc::new(state_store_mock);

    let metastore = Arc::new(InMemoryMetastore::new());
    MetastoreBootstrapConfig::bootstrap()
        .apply(metastore.clone())
        .await
        .expect("Failed to bootstrap metastore");

    let execution_svc = CoreExecutionService::new_test_executor(
        metastore,
        state_store,
        Arc::new(Config::default()),
    )
    .await
    .expect("Failed to create execution service");

    timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.create_session(TEST_SESSION_ID),
    )
    .await
    .expect("Create session timed out")
    .expect("Failed to create session");

    // prepare table
    let _ = timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.query(
            TEST_SESSION_ID,
            "create table if not exists embucket.public.table (id int)",
            query_context.clone(),
        ),
    )
    .await
    .expect("Query timed out")
    .expect("Query execution failed");

    // insert
    timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.query(
            TEST_SESSION_ID,
            "INSERT INTO embucket.public.table VALUES (1)",
            query_context,
        ),
    )
    .await
    .expect("Query timed out")
    .expect("Query execution failed");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_ok_update() {
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
        .times(2)
        .returning(|_| Ok(()));

    // bypass 1st update
    state_store_mock
        .expect_update_query()
        .times(1)
        .returning(|_| Ok(()));
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

    let state_store: Arc<dyn StateStore> = Arc::new(state_store_mock);

    let metastore = Arc::new(InMemoryMetastore::new());
    MetastoreBootstrapConfig::bootstrap()
        .apply(metastore.clone())
        .await
        .expect("Failed to bootstrap metastore");

    let execution_svc = CoreExecutionService::new_test_executor(
        metastore,
        state_store,
        Arc::new(Config::default()),
    )
    .await
    .expect("Failed to create execution service");

    timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.create_session(TEST_SESSION_ID),
    )
    .await
    .expect("Create session timed out")
    .expect("Failed to create session");

    // prepare table
    let _ = timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.query(
            TEST_SESSION_ID,
            "
        CREATE TABLE embucket.public.table AS SELECT 
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
            query_context.clone(),
        ),
    )
    .await
    .expect("Query timed out")
    .expect("Query execution failed");

    // update
    timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.query(
            TEST_SESSION_ID,
            "UPDATE embucket.public.table SET name = 'John'",
            query_context,
        ),
    )
    .await
    .expect("Query timed out")
    .expect("Query execution failed");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_delete_failed() {
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
        .times(2)
        .returning(|_| Ok(()));

    // bypass 1st update
    state_store_mock
        .expect_update_query()
        .times(1)
        .returning(|_| Ok(()));
    // verify 2nd update
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

    let state_store: Arc<dyn StateStore> = Arc::new(state_store_mock);

    let metastore = Arc::new(InMemoryMetastore::new());
    MetastoreBootstrapConfig::bootstrap()
        .apply(metastore.clone())
        .await
        .expect("Failed to bootstrap metastore");

    let execution_svc = CoreExecutionService::new_test_executor(
        metastore,
        state_store,
        Arc::new(Config::default()),
    )
    .await
    .expect("Failed to create execution service");

    timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.create_session(TEST_SESSION_ID),
    )
    .await
    .expect("Create session timed out")
    .expect("Failed to create session");

    // prepare table
    let _ = timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.query(
            TEST_SESSION_ID,
            "
        CREATE TABLE embucket.public.table AS SELECT 
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
            query_context.clone(),
        ),
    )
    .await
    .expect("Query timed out")
    .expect("Query execution failed");

    // update
    let _ = timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.query(
            TEST_SESSION_ID,
            "DELETE FROM embucket.public.table",
            query_context,
        ),
    )
    .await
    .expect("Query timed out")
    .expect_err("Query expected to fail");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_ok_truncate() {
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
        .times(2)
        .returning(|_| Ok(()));

    // bypass 1st update
    state_store_mock
        .expect_update_query()
        .times(1)
        .returning(|_| Ok(()));
    // verify 2nd update
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

    let state_store: Arc<dyn StateStore> = Arc::new(state_store_mock);

    let metastore = Arc::new(InMemoryMetastore::new());
    MetastoreBootstrapConfig::bootstrap()
        .apply(metastore.clone())
        .await
        .expect("Failed to bootstrap metastore");

    let execution_svc = CoreExecutionService::new_test_executor(
        metastore,
        state_store,
        Arc::new(Config::default()),
    )
    .await
    .expect("Failed to create execution service");

    timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.create_session(TEST_SESSION_ID),
    )
    .await
    .expect("Create session timed out")
    .expect("Failed to create session");

    // prepare table
    let _ = timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.query(
            TEST_SESSION_ID,
            "
        CREATE TABLE embucket.public.table AS SELECT 
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
            query_context.clone(),
        ),
    )
    .await
    .expect("Query timed out")
    .expect("Query execution failed");

    // update
    let _ = timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.query(
            TEST_SESSION_ID,
            "TRUNCATE TABLE embucket.public.table",
            query_context,
        ),
    )
    .await
    .expect("Query timed out")
    .expect("Query execution failed");
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle_ok_merge() {
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
        .times(3)
        .returning(|_| Ok(()));

    // bypass first two updates
    state_store_mock
        .expect_update_query()
        .times(2)
        .returning(|_| Ok(()));
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
                  "query_text": "MERGE INTO t1 USING (SELECT * FROM t2) AS t2 ON t1.a = t2.a WHEN MATCHED THEN UPDATE SET t1.c = t2.c WHEN NOT MATCHED THEN INSERT (a,c) VALUES(t2.a,t2.c)",
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
                  "query_hash": "16532873076018472935",
                  "query_hash_version": 1,
                  "query_metrics": "[query_metrics]"
                }
                "#);
            });
            true
        });

    let state_store: Arc<dyn StateStore> = Arc::new(state_store_mock);

    let metastore = Arc::new(InMemoryMetastore::new());
    MetastoreBootstrapConfig::bootstrap()
        .apply(metastore.clone())
        .await
        .expect("Failed to bootstrap metastore");

    let execution_svc = CoreExecutionService::new_test_executor(
        metastore,
        state_store,
        Arc::new(Config::default()),
    )
    .await
    .expect("Failed to create execution service");

    timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.create_session(TEST_SESSION_ID),
    )
    .await
    .expect("Create session timed out")
    .expect("Failed to create session");

    // prepare tables
    let _ = timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.query(
            TEST_SESSION_ID,
            "
        CREATE TABLE embucket.public.t1 AS SELECT 
        a,b,c
        FROM (VALUES 
            (1,'b1','c1'),
            (2,'b2','c2'),
            (2,'b3','c3'),
            (3,'b4','c4')
        ) AS t(a, b, c);",
            query_context.clone(),
        ),
    )
    .await
    .expect("Query timed out")
    .expect("Query execution failed");

    let _ = timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.query(
            TEST_SESSION_ID,
            "
        CREATE TABLE embucket.public.t2 AS SELECT
        a,b,c
        FROM (VALUES 
            (1,'b_5','c_5'),
            (3,'b_6','c_6'),
            (2,'b_7','c_7'),
            (4,'b_8','c_8')
        ) AS t(a, b, c);",
            query_context.clone(),
        ),
    )
    .await
    .expect("Query timed out")
    .expect("Query execution failed");

    let _ = timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.query(TEST_SESSION_ID, "MERGE INTO t1 USING (SELECT * FROM t2) AS t2 ON t1.a = t2.a WHEN MATCHED THEN UPDATE SET t1.c = t2.c WHEN NOT MATCHED THEN INSERT (a,c) VALUES(t2.a,t2.c)", query_context),
    )
    .await
    .expect("Query timed out")
    .expect("Query execution failed");
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

    let state_store: Arc<dyn StateStore> = Arc::new(state_store_mock);

    let metastore = Arc::new(InMemoryMetastore::new());
    let execution_svc = CoreExecutionService::new_test_executor(
        metastore,
        state_store,
        Arc::new(Config::default()),
    )
    .await
    .expect("Failed to create execution service");

    timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.create_session(TEST_SESSION_ID),
    )
    .await
    .expect("Create session timed out")
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

    let state_store: Arc<dyn StateStore> = Arc::new(state_store_mock);

    let metastore = Arc::new(InMemoryMetastore::new());
    let execution_svc = CoreExecutionService::new_test_executor(
        metastore,
        state_store,
        Arc::new(Config::default()),
    )
    .await
    .expect("Failed to create execution service");

    timeout(
        MOCK_RELATED_TIMEOUT_DURATION,
        execution_svc.create_session(TEST_SESSION_ID),
    )
    .await
    .expect("Create session timed out")
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
