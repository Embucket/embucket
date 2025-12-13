use executor::utils::{Config as ExecutionConfig, MemPoolType};
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
    pub embucket_version: String,
    pub metastore_config: Option<PathBuf>,
    pub jwt_secret: Option<String>,
    pub max_concurrent_table_fetches: usize,
    pub aws_sdk_connect_timeout_secs: u64,
    pub aws_sdk_operation_timeout_secs: u64,
    pub aws_sdk_operation_attempt_timeout_secs: u64,
    pub iceberg_table_timeout_secs: u64,
    pub iceberg_catalog_timeout_secs: u64,
    pub object_store_timeout_secs: u64,
    pub object_store_connect_timeout_secs: u64,
    pub otel_grpc: bool,
    pub tracing_level: String,
    pub log_format: String,
    pub log_filter: String,
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
            embucket_version: env_or_default("EMBUCKET_VERSION", "0.1.0"),
            metastore_config: env::var("METASTORE_CONFIG").ok().map(PathBuf::from),
            jwt_secret: env::var("JWT_SECRET").ok(),
            max_concurrent_table_fetches: parse_env("MAX_CONCURRENT_TABLE_FETCHES").unwrap_or(5),
            aws_sdk_connect_timeout_secs: parse_env("AWS_SDK_CONNECT_TIMEOUT_SECS").unwrap_or(3),
            aws_sdk_operation_timeout_secs: parse_env("AWS_SDK_OPERATION_TIMEOUT_SECS")
                .unwrap_or(30),
            aws_sdk_operation_attempt_timeout_secs: parse_env(
                "AWS_SDK_OPERATION_ATTEMPT_TIMEOUT_SECS",
            )
            .unwrap_or(10),
            iceberg_table_timeout_secs: parse_env("ICEBERG_CREATE_TABLE_TIMEOUT_SECS")
                .unwrap_or(30),
            iceberg_catalog_timeout_secs: parse_env("ICEBERG_CATALOG_TIMEOUT_SECS").unwrap_or(10),
            object_store_timeout_secs: parse_env("OBJECT_STORE_TIMEOUT_SECS").unwrap_or(30),
            object_store_connect_timeout_secs: parse_env("OBJECT_STORE_CONNECT_TIMEOUT_SECS")
                .unwrap_or(3),
            otel_grpc: parse_env("OTEL_GRPC").unwrap_or(true),
            tracing_level: env_or_default("TRACING_LEVEL", "INFO"),
            log_format: env_or_default("LOG_FORMAT", "pretty"),
            log_filter: parse_env("RUST_LOG").unwrap_or("INFO".to_string()),
        }
    }

    #[must_use]
    pub fn execution_config(&self) -> ExecutionConfig {
        ExecutionConfig {
            embucket_version: self.embucket_version.clone(),
            sql_parser_dialect: self.sql_parser_dialect.clone(),
            query_timeout_secs: self.query_timeout_secs,
            max_concurrency_level: self.max_concurrency_level,
            mem_pool_type: self.mem_pool_type,
            mem_pool_size_mb: self.mem_pool_size_mb,
            mem_enable_track_consumers_pool: self.mem_enable_track_consumers_pool,
            disk_pool_size_mb: self.disk_pool_size_mb,
            max_concurrent_table_fetches: self.max_concurrent_table_fetches,
            #[cfg(not(feature = "rest-catalog"))]
            aws_sdk_connect_timeout_secs: self.aws_sdk_connect_timeout_secs,
            #[cfg(not(feature = "rest-catalog"))]
            aws_sdk_operation_timeout_secs: self.aws_sdk_operation_timeout_secs,
            #[cfg(not(feature = "rest-catalog"))]
            aws_sdk_operation_attempt_timeout_secs: self.aws_sdk_operation_attempt_timeout_secs,
            iceberg_table_timeout_secs: self.iceberg_table_timeout_secs,
            iceberg_catalog_timeout_secs: self.iceberg_catalog_timeout_secs,
        }
    }
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_string())
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
