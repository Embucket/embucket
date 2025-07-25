use error_stack_trace;
use iceberg_rust::error::Error as IcebergError;
use iceberg_rust_spec::table_metadata::TableMetadataBuilderError;
use snafu::Location;
use snafu::prelude::*;
use strum_macros::AsRefStr;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Snafu, AsRefStr)]
#[snafu(visibility(pub))]
#[error_stack_trace::debug]
pub enum Error {
    #[snafu(display("Table data already exists at that location: {path}"))]
    TableDataExists {
        path: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Table requirement failed: {message}"))]
    TableRequirementFailed {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Volume: Validation failed. Reason: {reason}"))]
    VolumeValidationFailed {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Volume: Missing credentials"))]
    VolumeMissingCredentials {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Cloud provider not implemented"))]
    CloudProviderNotImplemented {
        provider: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("ObjectStore: {error}"))]
    ObjectStore {
        #[snafu(source)]
        error: object_store::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("ObjectStore path: {error}"))]
    ObjectStorePath {
        #[snafu(source)]
        error: object_store::path::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "Unable to create directory for File ObjectStore path {path}, error: {error}"
    ))]
    CreateDirectory {
        path: String,
        #[snafu(source)]
        error: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("SlateDB error: {error}"))]
    SlateDB {
        #[snafu(source)]
        error: slatedb::SlateDBError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("SlateDB error: {source}"))]
    UtilSlateDB {
        #[snafu(source(from(core_utils::Error, Box::new)))]
        source: Box<core_utils::Error>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Metastore object of type {type_name} with name {name} already exists"))]
    ObjectAlreadyExists {
        type_name: String,
        name: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Metastore object not found"))]
    ObjectNotFound {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Volume {volume} already exists"))]
    VolumeAlreadyExists {
        volume: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Volume {volume} not found"))]
    VolumeNotFound {
        volume: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Database {db} already exists"))]
    DatabaseAlreadyExists {
        db: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Database {db} not found"))]
    DatabaseNotFound {
        db: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Schema {schema} already exists in database {db}"))]
    SchemaAlreadyExists {
        schema: String,
        db: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Schema {schema} not found in database {db}"))]
    SchemaNotFound {
        schema: String,
        db: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Table {table} already exists in schema {schema} in database {db}"))]
    TableAlreadyExists {
        table: String,
        schema: String,
        db: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Table {table} not found in schema {schema} in database {db}"))]
    TableNotFound {
        table: String,
        schema: String,
        db: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "Table Object Store for table {table} in schema {schema} in database {db} not found"
    ))]
    TableObjectStoreNotFound {
        table: String,
        schema: String,
        db: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Volume in use by database(s): {database}"))]
    VolumeInUse {
        database: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Database {database} in use by schema(s): {schema}"))]
    DatabaseInUse {
        database: String,
        schema: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Iceberg error: {error}"))]
    Iceberg {
        #[snafu(source(from(IcebergError, Box::new)))]
        error: Box<IcebergError>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("TableMetadataBuilder error: {error}"))]
    TableMetadataBuilder {
        #[snafu(source)]
        error: TableMetadataBuilderError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Serialization error: {error}"))]
    Serde {
        #[snafu(source)]
        error: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Validation Error: {error}"))]
    Validation {
        #[snafu(source)]
        error: validator::ValidationErrors,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("UrlParse Error: {error}"))]
    UrlParse {
        #[snafu(source)]
        error: url::ParseError,
        #[snafu(implicit)]
        location: Location,
    },
}
