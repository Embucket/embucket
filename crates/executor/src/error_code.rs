use crate::error::OperationOn;
use crate::snowflake_error::Entity;
use std::fmt::Display;

// So far our ErrorCodes completely different from Snowflake error codes.
// For reference: https://github.com/snowflakedb/snowflake-cli/blob/main/src/snowflake/cli/api/errno.py
// Some of our error codes may be mapped to Snowflake error codes

// Do not set values for error codes, they are assigned in Display trait
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum ErrorCode {
    Db,
    Metastore,
    #[cfg(feature = "state-store")]
    StateStore,
    ObjectStore,
    Datafusion,
    DatafusionEmbucketFn,
    DatafusionEmbucketFnAggregate,
    DatafusionEmbucketFnConversion,
    DatafusionEmbucketFnDateTime,
    DatafusionEmbucketFnNumeric,
    DatafusionEmbucketFnSemiStructured,
    DatafusionEmbucketFnStringBinary,
    DatafusionEmbucketFnTable,
    DatafusionEmbucketFnCrate,
    DatafusionEmbucketFnRegexp,
    DatafusionEmbucketFnSystem,
    Arrow,
    Catalog,
    Iceberg,
    Internal,
    HistoricalQueryError,
    DataFusionSqlParse,
    DataFusionSql,
    EntityNotFound(Entity, OperationOn),
    Other,
    UnsupportedFeature,
    Timeout,
    Cancelled,
    LimitExceeded,
    QueryTask,
}

impl Display for ErrorCode {
    #[allow(clippy::unnested_or_patterns)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = match self {
            Self::UnsupportedFeature => 2,
            Self::Timeout => 630,
            Self::Cancelled => 684,
            Self::HistoricalQueryError => 1001,
            Self::DataFusionSqlParse => 1003,
            Self::DataFusionSql => 2003,
            Self::EntityNotFound(entity, operation) => match (entity, operation) {
                (Entity::Table, OperationOn::Table(..))
                | (Entity::Schema, OperationOn::Table(..))
                | (Entity::Database, OperationOn::Table(..)) => 2003,
                _ => 2043,
            },
            _ => 10001,
        };
        write!(f, "{code:06}")
    }
}
