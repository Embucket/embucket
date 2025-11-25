use executor::utils::{Config as ExecutionConfig, DEFAULT_QUERY_HISTORY_ROWS_LIMIT, MemPoolType};
use std::{env, path::PathBuf};

#[derive(Debug, Clone)]
pub struct EnvConfig {
    pub data_format: String,
    pub auth_demo_user: String,
    pub auth_demo_password: String,
    pub sql_parser_dialect: Option<String>,
    pub query_timeout_secs: u64,
    pub max_concurrency_level: usize,
    pub mem_pool_type: MemPoolType,
    pub mem_pool_size_mb: Option<usize>,
    pub mem_enable_track_consumers_pool: Option<bool>,
    pub disk_pool_size_mb: Option<usize>,
    pub query_history_rows_limit: usize,
    pub bootstrap_default_entities: bool,
    pub embucket_version: String,
    pub metastore_config: Option<PathBuf>,
    pub jwt_secret: Option<String>,
    pub read_only: bool,
    pub max_concurrent_table_fetches: usize,
}

impl EnvConfig {
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            data_format: env_or_default("DATA_FORMAT", "json"),
            auth_demo_user: env_or_default("AUTH_DEMO_USER", "embucket"),
            auth_demo_password: env_or_default("AUTH_DEMO_PASSWORD", "embucket"),
            sql_parser_dialect: env::var("SQL_PARSER_DIALECT").ok(),
            query_timeout_secs: parse_env("QUERY_TIMEOUT_SECS").unwrap_or(1200),
            max_concurrency_level: parse_env("MAX_CONCURRENCY_LEVEL").unwrap_or(8),
            mem_pool_type: parse_mem_pool_type().unwrap_or_default(),
            mem_pool_size_mb: parse_env("MEM_POOL_SIZE_MB"),
            mem_enable_track_consumers_pool: parse_env("MEM_ENABLE_TRACK_CONSUMERS_POOL"),
            disk_pool_size_mb: parse_env("DISK_POOL_SIZE_MB"),
            query_history_rows_limit: parse_env("QUERY_HISTORY_ROWS_LIMIT")
                .unwrap_or(DEFAULT_QUERY_HISTORY_ROWS_LIMIT),
            bootstrap_default_entities: !env_bool("NO_BOOTSTRAP"),
            embucket_version: env_or_default("EMBUCKET_VERSION", "0.1.0"),
            metastore_config: env::var("METASTORE_CONFIG").ok().map(PathBuf::from),
            jwt_secret: env::var("JWT_SECRET").ok(),
            read_only: parse_env("READ_ONLY").unwrap_or(true),
            max_concurrent_table_fetches: parse_env("MAX_CONCURRENT_TABLE_FETCHES").unwrap_or(5),
        }
    }

    #[must_use]
    pub fn execution_config(&self) -> ExecutionConfig {
        ExecutionConfig {
            embucket_version: self.embucket_version.clone(),
            bootstrap_default_entities: self.bootstrap_default_entities,
            sql_parser_dialect: self.sql_parser_dialect.clone(),
            query_timeout_secs: self.query_timeout_secs,
            max_concurrency_level: self.max_concurrency_level,
            mem_pool_type: self.mem_pool_type,
            mem_pool_size_mb: self.mem_pool_size_mb,
            mem_enable_track_consumers_pool: self.mem_enable_track_consumers_pool,
            disk_pool_size_mb: self.disk_pool_size_mb,
            query_history_rows_limit: self.query_history_rows_limit,
            read_only: self.read_only,
            max_concurrent_table_fetches: self.max_concurrent_table_fetches,
        }
    }
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_string())
}

fn env_bool(name: &str) -> bool {
    env::var(name)
        .map(|value| matches!(value.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn parse_mem_pool_type() -> Option<MemPoolType> {
    env::var("MEM_POOL_TYPE")
        .ok()
        .and_then(|value| match value.to_lowercase().as_str() {
            "fair" => Some(MemPoolType::Fair),
            "greedy" => Some(MemPoolType::Greedy),
            _ => None,
        })
}

fn parse_env<T>(name: &str) -> Option<T>
where
    T: std::str::FromStr,
{
    env::var(name).ok().and_then(|value| value.parse().ok())
}
