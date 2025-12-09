use super::run_test_rest_api_server;
use crate::server::core_state::MetastoreConfig;
use crate::server::logic::JWT_TOKEN_EXPIRATION_SECONDS;
use crate::tests::TEST_JWT_SECRET;
use crate::tests::snow_sql::{ACCESS_TOKEN_KEY, snow_sql};
use crate::tests::snow_sql::{PASSWORD_KEY, REQUEST_ID_KEY, USER_KEY};
use crate::{models::JsonResponse, server::server_models::RestApiConfig};
use api_snowflake_rest_sessions::helpers::{create_jwt, jwt_claims};
use arrow::record_batch::RecordBatch;
use executor::utils::Config as UtilsConfig;

pub const DEMO_USER: &str = "embucket";
pub const DEMO_PASSWORD: &str = "embucket";

pub const ARROW: &str = "arrow";
pub const JSON: &str = "json";

#[must_use]
pub fn insta_replace_filters() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            r"[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}",
            "UUID",
        ),
        (
            r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{6} UTC",
            "UTC_TIME6",
        ),
        (
            r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{9} UTC",
            "UTC_TIME9",
        ),
    ]
}

pub fn query_id_from_snapshot(
    snapshot: &JsonResponse,
) -> std::result::Result<String, Box<dyn std::error::Error>> {
    if let Some(data) = &snapshot.data {
        if let Some(query_id) = &data.query_id {
            Ok(query_id.clone())
        } else {
            Err("No query ID".into())
        }
    } else {
        Err("No data".into())
    }
}

pub fn arrow_record_batch_from_snapshot(
    snapshot: &JsonResponse,
) -> std::result::Result<Vec<RecordBatch>, Box<dyn std::error::Error>> {
    if let Some(data) = &snapshot.data {
        if let Some(row_set_base_64) = &data.row_set_base_64 {
            Ok(crate::tests::read_arrow_data::read_record_batches_from_arrow_data(row_set_base_64))
        } else {
            Err("No row set base 64".into())
        }
    } else {
        Err("No data".into())
    }
}

#[derive(Debug)]
pub struct HistoricalCodes {
    pub sql_state: String,
    pub error_code: String,
}

impl std::fmt::Display for HistoricalCodes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "sqlState={}; errorCode={};",
            self.sql_state, self.error_code
        )
    }
}

pub struct SqlTest {
    pub server_cfg: Option<RestApiConfig>,
    pub executor_cfg: Option<UtilsConfig>,
    pub metastore_cfg: MetastoreConfig,
    pub setup_queries: Vec<String>,
    pub params: Vec<(&'static str, String)>,
    pub sqls: Vec<String>,
    pub skip_login: bool,
}

impl SqlTest {
    #[must_use]
    pub fn new(sqls: &[&str]) -> Self {
        Self {
            server_cfg: None,
            executor_cfg: None,
            metastore_cfg: MetastoreConfig::None,
            setup_queries: vec![],
            params: vec![],
            sqls: sqls.iter().map(|&s| s.to_string()).collect(),
            skip_login: false,
        }
    }

    #[must_use]
    pub fn with_setup_queries(self, setup_queries: &[&str]) -> Self {
        Self {
            setup_queries: setup_queries.iter().map(|&s| s.to_string()).collect(),
            ..self
        }
    }

    #[must_use]
    pub fn with_params(self, params: Vec<(&'static str, String)>) -> Self {
        Self { params, ..self }
    }

    #[must_use]
    pub fn with_server_config(self, server_cfg: RestApiConfig) -> Self {
        Self {
            server_cfg: Some(server_cfg),
            ..self
        }
    }

    #[must_use]
    pub fn with_executor_config(self, executor_cfg: UtilsConfig) -> Self {
        Self {
            executor_cfg: Some(executor_cfg),
            ..self
        }
    }

    #[must_use]
    pub fn with_skip_login(self) -> Self {
        Self {
            skip_login: true,
            ..self
        }
    }

    #[must_use]
    pub fn with_metastore_config(self, metastore_cfg: MetastoreConfig) -> Self {
        Self {
            metastore_cfg,
            ..self
        }
    }

    fn create_access_token(&self, addr: std::net::SocketAddr) -> String {
        // host is required to check token audience claim
        let host = addr.to_string();
        let jwt_secret = &self.server_cfg.as_ref().map_or_else(
            || TEST_JWT_SECRET.to_string(),
            |cfg| cfg.auth.jwt_secret.clone(),
        );
        let jwt_claims = jwt_claims(
            DEMO_USER,
            &host,
            time::Duration::seconds(JWT_TOKEN_EXPIRATION_SECONDS.into()),
        );

        let jwt_token = create_jwt(&jwt_claims, jwt_secret).expect("Failed to create JWT token");

        jwt_token
    }
}

pub async fn sql_test_wrapper<F>(sql_test: SqlTest, check_response_cb: F) -> bool
where
    F: Fn((String, String), &JsonResponse) -> bool,
{
    let mut result = true;
    let server_addr = run_test_rest_api_server(
        sql_test.server_cfg.clone(),
        sql_test.executor_cfg.clone(),
        sql_test.metastore_cfg.clone(),
    );
    let skip_login_token = sql_test
        .skip_login
        .then(|| sql_test.create_access_token(server_addr));

    let mut prev_response: Option<JsonResponse> = None;
    let test_start = std::time::Instant::now();
    let mut submitted_queries_handles = Vec::new();

    // run setup queries
    let mut setup_queries_params = std::collections::HashMap::from([
        (USER_KEY, DEMO_USER.to_string()),
        (PASSWORD_KEY, DEMO_PASSWORD.to_string()),
        (REQUEST_ID_KEY, uuid::Uuid::new_v4().to_string()),
    ]);
    if let Some(access_token) = &skip_login_token {
        setup_queries_params.insert(ACCESS_TOKEN_KEY, access_token.clone());
    }

    for setup_query in &sql_test.setup_queries {
        eprintln!("Setup: {setup_query}");
        // on login we add access_token to params
        let (res, task_handle) =
            snow_sql(&server_addr, setup_query, &mut setup_queries_params).await;
        if let Some(handle) = task_handle {
            let _resp = handle.await;
        }
        assert_eq!(res.success, true);
    }

    for (idx, sql) in sql_test.sqls.iter().enumerate() {
        let mut sql = sql.to_string();
        let sql_start = std::time::Instant::now();

        // replace $LAST_QUERY_ID by query_id from previous response
        if sql.contains("$LAST_QUERY_ID") {
            let resp = prev_response.expect("No previous response");
            let last_query_id =
                query_id_from_snapshot(&resp).expect("Can't acquire value for $LAST_QUERY_ID");
            sql = sql.replace("$LAST_QUERY_ID", &last_query_id);
        }

        let mut params = std::collections::HashMap::from([
            (USER_KEY, DEMO_USER.to_string()),
            (PASSWORD_KEY, DEMO_PASSWORD.to_string()),
            (REQUEST_ID_KEY, uuid::Uuid::new_v4().to_string()),
        ]);
        params.extend(sql_test.params.clone());
        if let Some(access_token) = &skip_login_token {
            params.insert(ACCESS_TOKEN_KEY, access_token.clone());
        }

        // on login we add access_token to params
        let (snapshot, task_handle) = snow_sql(&server_addr, &sql, &mut params).await;
        if let Some(handle) = task_handle {
            submitted_queries_handles.push(handle);
        }

        let test_duration = test_start.elapsed().as_millis();
        let sql_duration = sql_start.elapsed().as_millis();
        let async_query = sql.ends_with(";>").then(|| "Async ").unwrap_or("");
        let query_num = idx + 1;
        let sql_info = format!(
            "{async_query}SQL #{query_num} [spent: {sql_duration}/{test_duration}ms]: {sql}"
        );

        if !check_response_cb((sql, sql_info), &snapshot) {
            result = false;
        }

        prev_response = Some(snapshot);
    }
    // wait async queries, to prevent canceling queries when test finishes
    futures::future::join_all(submitted_queries_handles).await;
    result
}

#[macro_export]
macro_rules! sql_test {
    ($name:ident, $sql_test:expr) => {
        #[tokio::test(flavor = "multi_thread")]
        async fn $name() {
            use $crate::tests::sql_test_macro::arrow_record_batch_from_snapshot;
            use $crate::tests::sql_test_macro::{ insta_replace_filters, query_id_from_snapshot };
            use $crate::models::JsonResponse;

            let mod_name = module_path!().split("::").last().unwrap();

            let snapshot_cb = move |sql_info: (String, String), response: &JsonResponse| {
                let (sql, sql_info) = sql_info;

                println!("{sql_info}");
                insta::with_settings!({
                    snapshot_path => format!("snapshots/{mod_name}/"),
                    prepend_module_to_snapshot => false,
                    // for debug purposes fetch query_id of current query
                    description => format!("{}\nQuery UUID: {}{}",
                        sql_info,
                        query_id_from_snapshot(response)
                            .map_or_else(|_| "No query ID".to_string(), |id| id)
                        ,
                        arrow_record_batch_from_snapshot(response)
                            .map_or_else(
                                |_| String::new(),
                                |batches| format!("\nArrow record batches:\n{batches:#?}"))
                    ),
                    sort_maps => true,
                    filters => insta_replace_filters()
                }, {
                    // Converting json to string here, as for some reason when raw snapshot put to assert_snapshot
                    // serialized data contains "$serde_json::private::Number" or "$serde_json::private::RawValue"
                    // artifacts
                    let snapshot = serde_json::to_string_pretty(response).expect("Failed to serialize snapshot");
                    let snapshot = format!("{}\n{snapshot}", sql);
                    insta::assert_snapshot!(snapshot);
                    true
                })
            };

            sql_test_wrapper($sql_test, snapshot_cb).await;
        }
    };
}
