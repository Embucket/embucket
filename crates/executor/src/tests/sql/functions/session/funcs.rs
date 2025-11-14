use crate::test_query;

test_query!(
    session_objects,
    "SELECT CURRENT_WAREHOUSE(), CURRENT_DATABASE(), CURRENT_SCHEMA()",
    snapshot_path = "session"
);
test_query!(
    session_objects_with_aliases,
    "SELECT CURRENT_WAREHOUSE() as wh, CURRENT_DATABASE() as db, CURRENT_SCHEMA() as sch",
    snapshot_path = "session"
);
test_query!(
    session_current_schemas,
    "SELECT CURRENT_SCHEMAS()",
    snapshot_path = "session"
);
test_query!(
    session_current_schemas_with_aliases,
    "SELECT CURRENT_SCHEMAS() as sc",
    snapshot_path = "session"
);
test_query!(
    session_general,
    "SELECT CURRENT_VERSION(), CURRENT_CLIENT()",
    snapshot_path = "session"
);
test_query!(
    session,
    "SELECT CURRENT_ROLE_TYPE(), CURRENT_ROLE()",
    snapshot_path = "session"
);
test_query!(
    session_current_session,
    // Check only length of session id since it is dynamic uuid
    "SELECT length(CURRENT_SESSION())",
    snapshot_path = "session"
);
test_query!(
    session_current_ip_address,
    "SELECT CURRENT_IP_ADDRESS()",
    snapshot_path = "session"
);
