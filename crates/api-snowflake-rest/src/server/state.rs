use super::server_models::RestApiConfig;
use crate::server::core_state::CoreState;
use api_snowflake_rest_sessions::session::JwtSecret;
use executor::ExecutionAppState;
use executor::service::ExecutionService;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub execution_svc: Arc<dyn ExecutionService>,
    pub config: RestApiConfig,
}

impl ExecutionAppState for AppState {
    fn get_execution_svc(&self) -> Arc<dyn ExecutionService> {
        self.execution_svc.clone()
    }
}

impl JwtSecret for AppState {
    fn jwt_secret(&self) -> &str {
        self.config.auth.jwt_secret.as_str()
    }
}

impl From<&CoreState> for AppState {
    fn from(core_state: &CoreState) -> Self {
        Self {
            execution_svc: core_state.executor.clone(),
            config: core_state.rest_api_config.clone(),
        }
    }
}
