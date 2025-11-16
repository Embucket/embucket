use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use catalog_metastore::{
    Database, Metastore, RegisterTableFromMetadataArgs, Schema, SchemaIdent, TableFormat,
    TableIdent, Volume, VolumeIdent, VolumeType,
};
use iceberg_rust_spec::{
    error as iceberg_error,
    spec::{
        partition::PartitionSpec,
        schema::{Schema as IcebergSchema, SchemaV2},
        snapshot::{
            Operation, Snapshot, SnapshotBuilder, SnapshotReference, SnapshotRetention, Summary,
        },
        sort::SortOrder,
    },
    table_metadata::{FormatVersion, MAIN_BRANCH, MetadataLog, SnapshotLog, TableMetadata},
};
use object_store::path::Path as ObjectStorePath;
use serde::Deserialize;
use snafu::prelude::*;
use tokio::fs;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Deserialize, Default)]
pub struct MetastoreBootstrapConfig {
    #[serde(default)]
    volumes: Vec<VolumeEntry>,
    #[serde(default)]
    databases: Vec<DatabaseEntry>,
    #[serde(default)]
    schemas: Vec<SchemaEntry>,
    #[serde(default)]
    tables: Vec<TableEntry>,
}

#[derive(Debug, Deserialize, Clone)]
struct VolumeEntry {
    #[serde(flatten)]
    volume: Volume,
    #[serde(default)]
    database: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct DatabaseEntry {
    ident: String,
    volume: VolumeIdent,
}

#[derive(Debug, Deserialize, Clone)]
struct SchemaEntry {
    database: String,
    schema: String,
}

#[derive(Debug, Deserialize, Clone)]
struct TableEntry {
    database: String,
    schema: String,
    table: String,
    metadata_location: String,
    #[serde(default)]
    format: Option<TableFormat>,
    #[serde(default)]
    properties: Option<HashMap<String, String>>,
}

#[derive(Debug, Snafu)]
pub enum ConfigError {
    #[snafu(display("Failed to read metastore config {path:?}: {source}"))]
    ReadConfig {
        path: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Failed to parse metastore config {path:?}: {source}"))]
    ParseConfig {
        path: PathBuf,
        source: serde_yaml::Error,
    },
    #[snafu(display("Metastore bootstrap error: {source}"))]
    Metastore {
        source: catalog_metastore::error::Error,
    },
    #[snafu(display("Database {database} not found for table {table}"))]
    TableDatabaseMissing { table: String, database: String },
    #[snafu(display("Volume {volume} not found for table {table}"))]
    TableVolumeMissing { table: String, volume: VolumeIdent },
    #[snafu(display("Volume {volume} for table {table} must be backed by S3"))]
    UnsupportedVolume { table: String, volume: VolumeIdent },
    #[snafu(display("Invalid metadata location for table {table}: {reason}"))]
    InvalidMetadataLocation { table: String, reason: String },
    #[snafu(display(
        "No object store configured for volume {volume} while bootstrapping table {table}"
    ))]
    MissingVolumeObjectStore { table: String, volume: VolumeIdent },
    #[snafu(display("Failed to fetch metadata for table {table}: {source}"))]
    MetadataFetch {
        table: String,
        source: object_store::Error,
    },
    #[snafu(display("Failed to parse metadata for table {table}: {source}"))]
    MetadataParse {
        table: String,
        source: serde_json::Error,
    },
    #[snafu(display("Failed to convert metadata for table {table}: {source}"))]
    SnowflakeMetadata {
        table: String,
        source: SnowflakeMetadataError,
    },
}

#[derive(Debug, Snafu)]
pub enum SnowflakeMetadataError {
    #[snafu(display("Unsupported metadata format version {format_version}"))]
    UnsupportedFormat { format_version: u8 },
    #[snafu(display("Invalid schema definition: {source}"))]
    Schema { source: iceberg_error::Error },
    #[snafu(display("Invalid snapshot definition: {source}"))]
    Snapshot { source: iceberg_error::Error },
}

const DEFAULT_SCHEMA_NAME: &str = "public";

impl MetastoreBootstrapConfig {
    pub async fn load(path: &Path) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path).await.context(ReadConfigSnafu {
            path: path.to_path_buf(),
        })?;
        let config = serde_yaml::from_str(&contents).context(ParseConfigSnafu {
            path: path.to_path_buf(),
        })?;
        Ok(config)
    }

    pub async fn apply(&self, metastore: Arc<dyn Metastore>) -> Result<(), ConfigError> {
        for volume_entry in &self.volumes {
            self.apply_volume(volume_entry, metastore.clone()).await?;
        }

        for db in &self.databases {
            self.ensure_database(metastore.clone(), &db.ident, &db.volume)
                .await?;
        }

        for schema in &self.schemas {
            self.ensure_schema(metastore.clone(), &schema.database, &schema.schema)
                .await?;
        }

        for table in &self.tables {
            self.apply_table(table, metastore.clone()).await?;
        }

        Ok(())
    }

    async fn apply_volume(
        &self,
        entry: &VolumeEntry,
        metastore: Arc<dyn Metastore>,
    ) -> Result<(), ConfigError> {
        if metastore
            .get_volume(&entry.volume.ident)
            .await
            .context(MetastoreSnafu)?
            .is_none()
        {
            tracing::info!(
                volume = %entry.volume.ident,
                "Creating volume from metastore config"
            );
            metastore
                .create_volume(&entry.volume.ident, entry.volume.clone())
                .await
                .context(MetastoreSnafu)?;
        } else {
            tracing::debug!(
                volume = %entry.volume.ident,
                "Volume already exists, skipping config create"
            );
        }

        if let Some(database) = &entry.database {
            self.ensure_database(metastore, database, &entry.volume.ident)
                .await?;
        }

        Ok(())
    }

    async fn ensure_database(
        &self,
        metastore: Arc<dyn Metastore>,
        ident: &str,
        volume: &str,
    ) -> Result<(), ConfigError> {
        if metastore
            .get_database(&ident.to_string())
            .await
            .context(MetastoreSnafu)?
            .is_none()
        {
            tracing::info!(database = ident, volume, "Creating database from config");
            metastore
                .create_database(
                    &ident.to_string(),
                    Database {
                        ident: ident.to_string(),
                        volume: volume.to_string(),
                        properties: None,
                    },
                )
                .await
                .context(MetastoreSnafu)?;
        }
        self.ensure_schema(metastore, ident, DEFAULT_SCHEMA_NAME)
            .await?;
        Ok(())
    }

    async fn ensure_schema(
        &self,
        metastore: Arc<dyn Metastore>,
        database: &str,
        schema: &str,
    ) -> Result<(), ConfigError> {
        let schema_ident = SchemaIdent::new(database.to_string(), schema.to_string());
        if metastore
            .get_schema(&schema_ident)
            .await
            .context(MetastoreSnafu)?
            .is_none()
        {
            tracing::info!(
                schema = schema,
                database = database,
                "Creating schema from config"
            );
            metastore
                .create_schema(
                    &schema_ident,
                    Schema {
                        ident: schema_ident.clone(),
                        properties: None,
                    },
                )
                .await
                .context(MetastoreSnafu)?;
        }
        Ok(())
    }

    fn parse_table_metadata(bytes: &[u8], table: &str) -> Result<TableMetadata, ConfigError> {
        match serde_json::from_slice(bytes) {
            Ok(metadata) => Ok(metadata),
            Err(primary_err) => match serde_json::from_slice::<TableMetadataSnowflake>(bytes) {
                Ok(value) => value.try_into_table_metadata().map_err(|source| {
                    ConfigError::SnowflakeMetadata {
                        table: table.to_string(),
                        source,
                    }
                }),
                Err(_) => Err(ConfigError::MetadataParse {
                    table: table.to_string(),
                    source: primary_err,
                }),
            },
        }
    }

    async fn apply_table(
        &self,
        entry: &TableEntry,
        metastore: Arc<dyn Metastore>,
    ) -> Result<(), ConfigError> {
        let table_name = entry.fq_name();
        let table_ident = entry.table_ident();

        if metastore
            .table_exists(&table_ident)
            .await
            .context(MetastoreSnafu)?
        {
            tracing::debug!(table = %table_name, "Table already exists, skipping config create");
            return Ok(());
        }

        let database = metastore
            .get_database(&entry.database)
            .await
            .context(MetastoreSnafu)?
            .ok_or_else(|| ConfigError::TableDatabaseMissing {
                table: table_name.clone(),
                database: entry.database.clone(),
            })?;

        self.ensure_schema(metastore.clone(), &entry.database, &entry.schema)
            .await?;

        let volume_ident = database.volume.clone();
        let volume = metastore
            .get_volume(&volume_ident)
            .await
            .context(MetastoreSnafu)?
            .ok_or_else(|| ConfigError::TableVolumeMissing {
                table: table_name.clone(),
                volume: volume_ident.clone(),
            })?;

        let relative_path = Self::metadata_relative_path(entry, &volume)?;
        let object_store = metastore
            .volume_object_store(&volume.ident)
            .await
            .context(MetastoreSnafu)?
            .ok_or_else(|| ConfigError::MissingVolumeObjectStore {
                table: table_name.clone(),
                volume: volume.ident.clone(),
            })?;

        let metadata_bytes = object_store
            .get(&ObjectStorePath::from(relative_path.clone()))
            .await
            .map_err(|source| ConfigError::MetadataFetch {
                table: table_name.clone(),
                source,
            })?
            .bytes()
            .await
            .map_err(|source| ConfigError::MetadataFetch {
                table: table_name.clone(),
                source,
            })?;
        let metadata = Self::parse_table_metadata(metadata_bytes.as_ref(), &table_name)?;
        let volume_location = if metadata.location.is_empty() {
            None
        } else {
            Some(metadata.location.clone())
        };

        metastore
            .register_table_from_metadata(
                &table_ident,
                RegisterTableFromMetadataArgs {
                    metadata,
                    metadata_location: relative_path,
                    volume_ident: volume.ident.clone(),
                    volume_location,
                    format: entry.format.clone().unwrap_or(TableFormat::Iceberg),
                    properties: entry.properties.clone(),
                    is_temporary: false,
                },
            )
            .await
            .context(MetastoreSnafu)?;

        tracing::info!(table = %table_name, "Registered table from config");
        Ok(())
    }

    fn metadata_relative_path(entry: &TableEntry, volume: &Volume) -> Result<String, ConfigError> {
        let table = entry.fq_name();
        let url = Url::parse(&entry.metadata_location).map_err(|e| {
            ConfigError::InvalidMetadataLocation {
                table: table.clone(),
                reason: e.to_string(),
            }
        })?;
        if url.scheme() != "s3" {
            return Err(ConfigError::InvalidMetadataLocation {
                table,
                reason: "metadata_location must use the s3:// scheme".to_string(),
            });
        }
        let bucket = url
            .host_str()
            .ok_or_else(|| ConfigError::InvalidMetadataLocation {
                table: table.clone(),
                reason: "metadata_location must include an S3 bucket".to_string(),
            })?;

        let expected_bucket = match &volume.volume {
            VolumeType::S3(s3) => {
                s3.bucket
                    .clone()
                    .ok_or_else(|| ConfigError::InvalidMetadataLocation {
                        table: table.clone(),
                        reason: format!(
                            "Volume {} is missing a bucket configuration",
                            volume.ident
                        ),
                    })?
            }
            VolumeType::S3Tables(s3_tables) => {
                s3_tables
                    .bucket()
                    .ok_or_else(|| ConfigError::InvalidMetadataLocation {
                        table: table.clone(),
                        reason: format!(
                            "Volume {} is missing a bucket configuration",
                            volume.ident
                        ),
                    })?
            }
            _ => {
                return Err(ConfigError::UnsupportedVolume {
                    table,
                    volume: volume.ident.clone(),
                });
            }
        };

        if bucket != expected_bucket.as_str() {
            return Err(ConfigError::InvalidMetadataLocation {
                table,
                reason: format!(
                    "metadata bucket {bucket} does not match configured volume bucket {expected_bucket}"
                ),
            });
        }

        let path = url.path().trim_start_matches('/').to_string();
        if path.is_empty() {
            return Err(ConfigError::InvalidMetadataLocation {
                table,
                reason: "metadata_location must include a key within the bucket".to_string(),
            });
        }

        if !path.ends_with("metadata.json") {
            return Err(ConfigError::InvalidMetadataLocation {
                table,
                reason: "metadata_location must point to a metadata.json file".to_string(),
            });
        }
        Ok(path)
    }
}

impl TableEntry {
    fn fq_name(&self) -> String {
        format!("{}.{}.{}", self.database, self.schema, self.table)
    }

    fn table_ident(&self) -> TableIdent {
        TableIdent::new(&self.database, &self.schema, &self.table)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct TableMetadataSnowflake {
    #[serde(rename = "format-version")]
    format_version: u8,
    #[serde(rename = "table-uuid")]
    table_uuid: Uuid,
    location: String,
    #[serde(rename = "last-sequence-number")]
    last_sequence_number: i64,
    #[serde(rename = "last-updated-ms")]
    last_updated_ms: i64,
    #[serde(rename = "last-column-id")]
    last_column_id: i32,
    schemas: Vec<SchemaV2>,
    #[serde(rename = "current-schema-id")]
    current_schema_id: i32,
    #[serde(rename = "partition-specs")]
    partition_specs: Vec<PartitionSpec>,
    #[serde(rename = "default-spec-id")]
    default_spec_id: i32,
    #[serde(rename = "last-partition-id")]
    last_partition_id: i32,
    #[serde(default)]
    properties: HashMap<String, String>,
    #[serde(rename = "current-snapshot-id")]
    current_snapshot_id: Option<i64>,
    #[serde(default)]
    snapshots: Vec<SnowflakeSnapshot>,
    #[serde(rename = "snapshot-log", default)]
    snapshot_log: Vec<SnapshotLog>,
    #[serde(rename = "metadata-log", default)]
    metadata_log: Vec<MetadataLog>,
    #[serde(rename = "sort-orders", default)]
    sort_orders: Vec<SortOrder>,
    #[serde(rename = "default-sort-order-id")]
    default_sort_order_id: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct SnowflakeSnapshot {
    snapshot_id: i64,
    #[serde(rename = "parent-snapshot-id")]
    parent_snapshot_id: Option<i64>,
    #[serde(rename = "sequence-number")]
    sequence_number: i64,
    #[serde(rename = "timestamp-ms")]
    timestamp_ms: i64,
    #[serde(rename = "manifest-list")]
    manifest_list: String,
    summary: HashMap<String, String>,
    #[serde(rename = "schema-id")]
    schema_id: Option<i32>,
}

impl TableMetadataSnowflake {
    fn try_into_table_metadata(self) -> Result<TableMetadata, SnowflakeMetadataError> {
        let format_version = FormatVersion::try_from(self.format_version).map_err(|_| {
            SnowflakeMetadataError::UnsupportedFormat {
                format_version: self.format_version,
            }
        })?;

        if format_version != FormatVersion::V2 {
            return Err(SnowflakeMetadataError::UnsupportedFormat {
                format_version: self.format_version,
            });
        }

        let schemas = self
            .schemas
            .into_iter()
            .map(|schema| {
                let schema_id = schema.schema_id;
                let schema: IcebergSchema = schema
                    .try_into()
                    .map_err(|source| SnowflakeMetadataError::Schema { source })?;
                Ok((schema_id, schema))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        let partition_specs = self
            .partition_specs
            .into_iter()
            .map(|spec| (*spec.spec_id(), spec))
            .collect::<HashMap<_, _>>();

        let sort_orders = self
            .sort_orders
            .into_iter()
            .map(|order| (order.order_id, order))
            .collect::<HashMap<_, _>>();

        let snapshots = self
            .snapshots
            .into_iter()
            .map(|snapshot| {
                let snapshot_id = snapshot.snapshot_id;
                let snapshot = snapshot
                    .into_snapshot()
                    .map_err(|source| SnowflakeMetadataError::Snapshot { source })?;
                Ok((snapshot_id, snapshot))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        let current_snapshot_id = self.current_snapshot_id.filter(|id| *id != -1);
        let mut refs = HashMap::new();
        if let Some(snapshot_id) = current_snapshot_id {
            refs.insert(
                MAIN_BRANCH.to_string(),
                SnapshotReference {
                    snapshot_id,
                    retention: SnapshotRetention::default(),
                },
            );
        }

        Ok(TableMetadata {
            format_version,
            table_uuid: self.table_uuid,
            location: self.location,
            last_sequence_number: self.last_sequence_number,
            last_updated_ms: self.last_updated_ms,
            last_column_id: self.last_column_id,
            schemas,
            current_schema_id: self.current_schema_id,
            partition_specs,
            default_spec_id: self.default_spec_id,
            last_partition_id: self.last_partition_id,
            properties: self.properties,
            current_snapshot_id,
            snapshots,
            snapshot_log: self.snapshot_log,
            metadata_log: self.metadata_log,
            sort_orders,
            default_sort_order_id: self.default_sort_order_id,
            refs,
        })
    }
}

impl SnowflakeSnapshot {
    fn into_snapshot(self) -> Result<Snapshot, iceberg_error::Error> {
        let (operation, other) = Self::split_operation(self.summary);
        let mut builder = SnapshotBuilder::default();
        builder
            .with_snapshot_id(self.snapshot_id)
            .with_sequence_number(self.sequence_number)
            .with_timestamp_ms(self.timestamp_ms)
            .with_manifest_list(self.manifest_list)
            .with_summary(Summary { operation, other });
        if let Some(parent_snapshot_id) = self.parent_snapshot_id {
            builder.with_parent_snapshot_id(parent_snapshot_id);
        }
        if let Some(schema_id) = self.schema_id {
            builder.with_schema_id(schema_id);
        }
        builder.build()
    }

    fn split_operation(
        mut summary: HashMap<String, String>,
    ) -> (Operation, HashMap<String, String>) {
        let operation = summary
            .remove("operation")
            .map_or(Operation::Append, |value| {
                match value.to_ascii_lowercase().as_str() {
                    "replace" => Operation::Replace,
                    "overwrite" => Operation::Overwrite,
                    "delete" => Operation::Delete,
                    other => {
                        tracing::warn!(
                            operation = other,
                            "Unknown snapshot operation, defaulting to APPEND"
                        );
                        Operation::Append
                    }
                }
            });
        (operation, summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use catalog_metastore::DatabaseIdent;
    use catalog_metastore::models::volumes::{AwsAccessKeyCredentials, AwsCredentials, S3Volume};
    use catalog_metastore::{
        RwObject, Table, TableCreateRequest, TableUpdate, error as metastore_error,
    };
    use object_store::{
        ObjectStore, PutPayload, memory::InMemory as InMemoryObjectStore, path::Path as StorePath,
    };
    use std::path::PathBuf;
    use tokio::sync::RwLock;

    #[derive(Default)]
    struct SimpleMetastore {
        state: Arc<RwLock<SimpleState>>,
        stores: Arc<RwLock<HashMap<VolumeIdent, Arc<dyn ObjectStore>>>>,
    }

    #[derive(Default)]
    struct SimpleState {
        volumes: HashMap<VolumeIdent, RwObject<Volume>>,
        databases: HashMap<DatabaseIdent, RwObject<Database>>,
        schemas: HashMap<(DatabaseIdent, String), RwObject<Schema>>,
        tables: HashMap<(DatabaseIdent, String, String), RwObject<Table>>,
    }

    impl std::fmt::Debug for SimpleMetastore {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SimpleMetastore").finish()
        }
    }

    impl SimpleMetastore {
        fn new() -> Self {
            Self {
                state: Arc::new(RwLock::new(SimpleState::default())),
                stores: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        async fn set_object_store(&self, volume: &str, store: Arc<dyn ObjectStore>) {
            self.stores.write().await.insert(volume.to_string(), store);
        }

        fn table_key(ident: &TableIdent) -> (DatabaseIdent, String, String) {
            (
                ident.database.to_ascii_lowercase(),
                ident.schema.to_ascii_lowercase(),
                ident.table.to_ascii_lowercase(),
            )
        }

        fn schema_key(ident: &SchemaIdent) -> (DatabaseIdent, String) {
            (
                ident.database.to_ascii_lowercase(),
                ident.schema.to_ascii_lowercase(),
            )
        }
    }

    #[async_trait::async_trait]
    impl Metastore for SimpleMetastore {
        async fn list_volumes(&self) -> metastore_error::Result<Vec<RwObject<Volume>>> {
            let state = self.state.read().await;
            Ok(state.volumes.values().cloned().collect())
        }

        async fn create_volume(
            &self,
            name: &VolumeIdent,
            volume: Volume,
        ) -> metastore_error::Result<RwObject<Volume>> {
            let mut state = self.state.write().await;
            if state.volumes.contains_key(name) {
                return metastore_error::VolumeAlreadyExistsSnafu {
                    volume: name.clone(),
                }
                .fail();
            }
            let row = RwObject::new(volume);
            state.volumes.insert(name.clone(), row.clone());
            Ok(row)
        }

        async fn get_volume(
            &self,
            name: &VolumeIdent,
        ) -> metastore_error::Result<Option<RwObject<Volume>>> {
            let state = self.state.read().await;
            Ok(state.volumes.get(name).cloned())
        }

        async fn update_volume(
            &self,
            name: &VolumeIdent,
            volume: Volume,
        ) -> metastore_error::Result<RwObject<Volume>> {
            let mut state = self.state.write().await;
            let entry = state.volumes.get_mut(name).ok_or_else(|| {
                metastore_error::VolumeNotFoundSnafu {
                    volume: name.clone(),
                }
                .build()
            })?;
            entry.update(volume);
            Ok(entry.clone())
        }

        async fn delete_volume(
            &self,
            _name: &VolumeIdent,
            _cascade: bool,
        ) -> metastore_error::Result<()> {
            unimplemented!("not required for tests")
        }

        async fn volume_object_store(
            &self,
            name: &VolumeIdent,
        ) -> metastore_error::Result<Option<Arc<dyn ObjectStore>>> {
            let stores = self.stores.read().await;
            Ok(stores.get(name).cloned())
        }

        async fn list_databases(&self) -> metastore_error::Result<Vec<RwObject<Database>>> {
            let state = self.state.read().await;
            Ok(state.databases.values().cloned().collect())
        }

        async fn create_database(
            &self,
            name: &DatabaseIdent,
            database: Database,
        ) -> metastore_error::Result<RwObject<Database>> {
            let mut state = self.state.write().await;
            if !state.volumes.contains_key(&database.volume) {
                return metastore_error::VolumeNotFoundSnafu {
                    volume: database.volume.clone(),
                }
                .fail();
            }
            if state.databases.contains_key(name) {
                return metastore_error::DatabaseAlreadyExistsSnafu { db: name }.fail();
            }
            let row = RwObject::new(database);
            state.databases.insert(name.clone(), row.clone());
            Ok(row)
        }

        async fn get_database(
            &self,
            name: &DatabaseIdent,
        ) -> metastore_error::Result<Option<RwObject<Database>>> {
            let state = self.state.read().await;
            Ok(state.databases.get(name).cloned())
        }

        async fn update_database(
            &self,
            name: &DatabaseIdent,
            database: Database,
        ) -> metastore_error::Result<RwObject<Database>> {
            let mut state = self.state.write().await;
            let entry = state.databases.get_mut(name).ok_or_else(|| {
                metastore_error::DatabaseNotFoundSnafu { db: name.clone() }.build()
            })?;
            entry.update(database);
            Ok(entry.clone())
        }

        async fn delete_database(
            &self,
            _name: &DatabaseIdent,
            _cascade: bool,
        ) -> metastore_error::Result<()> {
            unimplemented!("not required for tests")
        }

        async fn list_schemas(
            &self,
            database: &DatabaseIdent,
        ) -> metastore_error::Result<Vec<RwObject<Schema>>> {
            let state = self.state.read().await;
            Ok(state
                .schemas
                .iter()
                .filter(|((db, _), _)| db == database)
                .map(|(_, schema)| schema.clone())
                .collect())
        }

        async fn create_schema(
            &self,
            ident: &SchemaIdent,
            schema: Schema,
        ) -> metastore_error::Result<RwObject<Schema>> {
            let mut state = self.state.write().await;
            if !state.databases.contains_key(&ident.database) {
                return metastore_error::DatabaseNotFoundSnafu {
                    db: ident.database.clone(),
                }
                .fail();
            }
            let key = SimpleMetastore::schema_key(ident);
            if state.schemas.contains_key(&key) {
                return metastore_error::SchemaAlreadyExistsSnafu {
                    schema: ident.schema.clone(),
                    db: ident.database.clone(),
                }
                .fail();
            }
            let row = RwObject::new(schema);
            state.schemas.insert(key, row.clone());
            Ok(row)
        }

        async fn get_schema(
            &self,
            ident: &SchemaIdent,
        ) -> metastore_error::Result<Option<RwObject<Schema>>> {
            let state = self.state.read().await;
            Ok(state
                .schemas
                .get(&SimpleMetastore::schema_key(ident))
                .cloned())
        }

        async fn update_schema(
            &self,
            ident: &SchemaIdent,
            schema: Schema,
        ) -> metastore_error::Result<RwObject<Schema>> {
            let mut state = self.state.write().await;
            let entry = state
                .schemas
                .get_mut(&SimpleMetastore::schema_key(ident))
                .ok_or_else(|| {
                    metastore_error::SchemaNotFoundSnafu {
                        schema: ident.schema.clone(),
                        db: ident.database.clone(),
                    }
                    .build()
                })?;
            entry.update(schema);
            Ok(entry.clone())
        }

        async fn delete_schema(
            &self,
            _ident: &SchemaIdent,
            _cascade: bool,
        ) -> metastore_error::Result<()> {
            unimplemented!("not required for tests")
        }

        async fn list_tables(
            &self,
            schema: &SchemaIdent,
        ) -> metastore_error::Result<Vec<RwObject<Table>>> {
            let state = self.state.read().await;
            let db = schema.database.to_ascii_lowercase();
            let sch = schema.schema.to_ascii_lowercase();
            Ok(state
                .tables
                .iter()
                .filter(|((table_db, table_schema, _), _)| table_db == &db && table_schema == &sch)
                .map(|(_, table)| table.clone())
                .collect())
        }

        async fn create_table(
            &self,
            _ident: &TableIdent,
            _table: TableCreateRequest,
        ) -> metastore_error::Result<RwObject<Table>> {
            unimplemented!("not required for tests")
        }

        async fn register_table_from_metadata(
            &self,
            ident: &TableIdent,
            args: RegisterTableFromMetadataArgs,
        ) -> metastore_error::Result<RwObject<Table>> {
            let RegisterTableFromMetadataArgs {
                metadata,
                metadata_location,
                volume_ident,
                volume_location,
                format,
                properties,
                is_temporary,
            } = args;
            let mut state = self.state.write().await;
            let schema_ident: SchemaIdent = ident.clone().into();
            if !state
                .schemas
                .contains_key(&SimpleMetastore::schema_key(&schema_ident))
            {
                return metastore_error::SchemaNotFoundSnafu {
                    schema: ident.schema.clone(),
                    db: ident.database.clone(),
                }
                .fail();
            }
            if state
                .tables
                .contains_key(&SimpleMetastore::table_key(ident))
            {
                return metastore_error::TableAlreadyExistsSnafu {
                    table: ident.table.clone(),
                    schema: ident.schema.clone(),
                    db: ident.database.clone(),
                }
                .fail();
            }
            if !state.volumes.contains_key(&volume_ident) {
                return metastore_error::VolumeNotFoundSnafu {
                    volume: volume_ident,
                }
                .fail();
            }
            let mut properties = properties.unwrap_or_default();
            let now = chrono::Utc::now().to_rfc3339();
            properties.insert("created_at".to_string(), now.clone());
            properties.insert("updated_at".to_string(), now);
            let table = Table {
                ident: ident.clone(),
                metadata,
                metadata_location,
                properties,
                volume_ident: Some(volume_ident),
                volume_location,
                is_temporary,
                format,
            };
            let row = RwObject::new(table);
            state
                .tables
                .insert(SimpleMetastore::table_key(ident), row.clone());
            Ok(row)
        }

        async fn get_table(
            &self,
            ident: &TableIdent,
        ) -> metastore_error::Result<Option<RwObject<Table>>> {
            let state = self.state.read().await;
            Ok(state
                .tables
                .get(&SimpleMetastore::table_key(ident))
                .cloned())
        }

        async fn update_table(
            &self,
            _ident: &TableIdent,
            _update: TableUpdate,
        ) -> metastore_error::Result<RwObject<Table>> {
            unimplemented!("not required for tests")
        }

        async fn delete_table(
            &self,
            _ident: &TableIdent,
            _cascade: bool,
        ) -> metastore_error::Result<()> {
            unimplemented!("not required for tests")
        }

        async fn table_object_store(
            &self,
            ident: &TableIdent,
        ) -> metastore_error::Result<Option<Arc<dyn ObjectStore>>> {
            let state = self.state.read().await;
            if let Some(table) = state.tables.get(&SimpleMetastore::table_key(ident)) {
                if let Some(volume_ident) = table.volume_ident.as_ref() {
                    let stores = self.stores.read().await;
                    return Ok(stores.get(volume_ident).cloned());
                }
            }
            Ok(None)
        }

        async fn table_exists(&self, ident: &TableIdent) -> metastore_error::Result<bool> {
            let state = self.state.read().await;
            Ok(state
                .tables
                .contains_key(&SimpleMetastore::table_key(ident)))
        }

        async fn url_for_table(&self, ident: &TableIdent) -> metastore_error::Result<String> {
            let state = self.state.read().await;
            if let Some(table) = state.tables.get(&SimpleMetastore::table_key(ident)) {
                Ok(table
                    .volume_location
                    .clone()
                    .unwrap_or_else(|| format!("memory://{}", ident.table)))
            } else {
                metastore_error::TableNotFoundSnafu {
                    table: ident.table.clone(),
                    schema: ident.schema.clone(),
                    db: ident.database.clone(),
                }
                .fail()
            }
        }

        async fn volume_for_table(
            &self,
            ident: &TableIdent,
        ) -> metastore_error::Result<Option<RwObject<Volume>>> {
            let state = self.state.read().await;
            if let Some(table) = state.tables.get(&SimpleMetastore::table_key(ident)) {
                if let Some(volume_ident) = table.volume_ident.as_ref() {
                    return Ok(state.volumes.get(volume_ident).cloned());
                }
            }
            Ok(None)
        }
    }

    #[tokio::test]
    async fn bootstraps_tables_from_metadata() {
        let metastore = Arc::new(SimpleMetastore::new());
        let store: Arc<dyn ObjectStore> = Arc::new(InMemoryObjectStore::new());
        metastore.set_object_store("tpch", store.clone()).await;

        let metadata_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/lineitem.metadata.json");
        let metadata_bytes = std::fs::read(metadata_path).expect("metadata file");
        let object_path = "SERGEI_DB/SF_1/LINEITEM.G8xni6nE/metadata/00002-59fbe249-8f70-4c71-91e5-8eee75a0559c.metadata.json";
        store
            .put(
                &StorePath::from(object_path),
                PutPayload::from_bytes(Bytes::from(metadata_bytes)),
            )
            .await
            .expect("write metadata");

        let config = MetastoreBootstrapConfig {
            volumes: vec![VolumeEntry {
                volume: Volume {
                    ident: "tpch".to_string(),
                    volume: VolumeType::S3(S3Volume {
                        region: Some("us-east-2".to_string()),
                        bucket: Some("embucket-lakehouse".to_string()),
                        endpoint: Some("https://example.com".to_string()),
                        credentials: Some(AwsCredentials::AccessKey(AwsAccessKeyCredentials {
                            aws_access_key_id: "ABCDEFGHIJKLMNOPQRST".to_string(),
                            aws_secret_access_key: "ABCDEFGHIJKLMNOPQRSTUVWX1234567890ABCD"
                                .to_string(),
                        })),
                    }),
                },
                database: None,
            }],
            databases: vec![DatabaseEntry {
                ident: "SERGEI_DB".to_string(),
                volume: "tpch".to_string(),
            }],
            schemas: vec![SchemaEntry {
                database: "SERGEI_DB".to_string(),
                schema: "SF_1".to_string(),
            }],
            tables: vec![TableEntry {
                database: "SERGEI_DB".to_string(),
                schema: "SF_1".to_string(),
                table: "LINEITEM".to_string(),
                metadata_location: format!("s3://embucket-lakehouse/{object_path}"),
                format: None,
                properties: None,
            }],
        };

        config.apply(metastore.clone()).await.expect("apply config");

        let table_ident = TableIdent::new("SERGEI_DB", "SF_1", "LINEITEM");
        let table = metastore
            .get_table(&table_ident)
            .await
            .expect("get table")
            .expect("table created");
        assert_eq!(table.volume_ident.as_deref(), Some("tpch"));
        assert_eq!(
            table.volume_location.as_deref(),
            Some("s3://embucket-lakehouse/SERGEI_DB/SF_1/LINEITEM.G8xni6nE/")
        );
        assert_eq!(
            table.metadata_location,
            "SERGEI_DB/SF_1/LINEITEM.G8xni6nE/metadata/00002-59fbe249-8f70-4c71-91e5-8eee75a0559c.metadata.json"
        );
        assert!(
            table
                .metadata
                .schemas
                .get(&table.metadata.current_schema_id)
                .is_some()
        );
    }
}
