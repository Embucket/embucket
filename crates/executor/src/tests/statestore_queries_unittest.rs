use crate::models::QueryContext;
use crate::service::{CoreExecutionService, ExecutionService};
use crate::utils::Config;
use catalog_metastore::InMemoryMetastore;
use state_store::{MockStateStore, SessionRecord, StateStore};
use std::sync::Arc;

const TEST_SESSION_ID: &str = "test_session_id";

// it stucks without multithread
#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle() {
    let mut state_store_mock = MockStateStore::new();
    state_store_mock
        .expect_put_new_session()
        .returning(|_| Ok(()));
    state_store_mock
        .expect_get_session()
        .returning(|_| Ok(SessionRecord::new(TEST_SESSION_ID)));
    state_store_mock.expect_put_query().returning(|_| Ok(()));
    state_store_mock.expect_update_query().returning(|_| Ok(()));

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

    let _ = execution_svc
        .query(
            TEST_SESSION_ID,
            "SELECT 1 AS a, 2.0 AS b, '3' AS c WHERE False",
            QueryContext::default(),
        )
        .await
        .expect("Failed to execute query");
}
