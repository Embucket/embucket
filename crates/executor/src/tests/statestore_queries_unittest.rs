use crate::models::QueryContext;
use crate::service::{CoreExecutionService, ExecutionService};
use crate::utils::Config;
use catalog_metastore::InMemoryMetastore;
use state_store::{SessionRecord, StateStore, MockStateStore, Query, Result};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

const TEST_SESSION_ID: &str = "test_session_id";

// Note: Run mocked async function with timeout as in case 
// if mocked_function.withf() not returning true then entire async function stuck

pub struct TestStateStore;

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_query_lifecycle() {
    let mut state_store_mock = MockStateStore::new();
    state_store_mock
        .expect_put_new_session()
        .returning(|_| Ok(()) );
    state_store_mock
        .expect_get_session()
        .returning(|_| Ok(SessionRecord::new(TEST_SESSION_ID)) );
    state_store_mock.expect_put_query().returning(|_| Ok(()) );
    state_store_mock.expect_update_query()
        .times(1)
        .returning(|_| Ok(()) )
        .withf(|query: &Query| {
            query.end_time.is_some() &&
            query.execution_time.is_some() &&
            query.error_code.is_some() &&
            query.error_message.is_some()
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
    let _ = timeout(Duration::from_millis(500), execution_svc
        .query(
            TEST_SESSION_ID,
            "SELECT 1 AS a, 2.0 AS b, '3' AS c WHERE False",
            QueryContext::default(),
        )
    ).await
    .expect("Query timed out")
    .expect("Query execution stopped by timeout");
}
