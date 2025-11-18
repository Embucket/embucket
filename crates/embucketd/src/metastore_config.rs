use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use iceberg_file_catalog::FileCatalog;
use object_store::local::LocalFileSystem;
use object_store::ObjectStore;
use catalog_metastore::{Database, Metastore, Schema, SchemaIdent, TableIdent, Volume, VolumeIdent, VolumeType};
use serde::Deserialize;
use snafu::prelude::*;
use tokio::fs;
use object_store::path::Path as ObjectStorePath;


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
}

impl TableEntry {
    fn table_ident(&self) -> TableIdent {
        TableIdent::new(&self.database, &self.schema, &self.table)
    }
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
    #[snafu(display(
        "No object store configured for volume {volume} while bootstrapping table {table}"
    ))]
    MissingVolumeObjectStore { table: String, volume: VolumeIdent },
    #[snafu(display("Invalid metadata location for table {table}: {reason}"))]
    InvalidMetadataLocation { table: String, reason: String },
    #[snafu(display("Failed to fetch metadata for table {table}: {source}"))]
    MetadataFetch {
        table: String,
        source: object_store::Error,
    },
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

    async fn apply_table(
        &self,
        entry: &TableEntry,
        metastore: Arc<dyn Metastore>,
    ) -> Result<(), ConfigError> {
        let table_ident = entry.table_ident();
        let table_name = entry.table;
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

        let table_object_store = metastore
            .table_object_store(&table_ident)
            .await
            .context(MetastoreSnafu)?
            .ok_or_else(|| ConfigError::MissingVolumeObjectStore {
            table: table_name.clone(),
            volume: database.volume.clone(),
        })?;
        let path = &ObjectStorePath::from_url_path(&entry.metadata_location)
            .map_err(|e| { ConfigError::InvalidMetadataLocation {
                table: table_name.clone(),
                reason: "".to_string(),
            }})?;

        let metadata = table_object_store
            .get(path)
            .await
            .context(ConfigError::MetadataFetch)?;

        let volume_ident = database.volume.clone();
        let volume = metastore
            .get_volume(&volume_ident)
            .await
            .context(MetastoreSnafu)?
            .ok_or_else(|| ConfigError::TableVolumeMissing {
                table: table_name.clone(),
                volume: volume_ident.clone(),
            })?;
        volume.get_object_store()
        let object_store = metastore
            .volume_object_store(&volume.ident)
            .await
            .context(MetastoreSnafu)?
            .ok_or_else(|| ConfigError::MissingVolumeObjectStore {
                table: table_name.clone(),
                volume: volume.ident.clone(),
            })?;
        let file_catalog = FileCatalog::new("s3://base/path", ObjectStoreBuilder::memory())
            .await
            .unwrap();

        let file_catalog = FileCatalog::new()
        let metadata = object_store
            .get(&ObjectStorePath::from_url_path(&entry.metadata_location))
            .await
            .context(TestObjectStoreSnafu);
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
}


fn get_object_store_builder(volume: Volume) -> ObjectStoreBuilder {
    match volume.volume {
        VolumeType::S3(volume) => {
            let builder = volume.get_s3_builder();

        }
        VolumeType::S3Tables(volume) => {
            let builder = volume.s3_builder();

        }
        VolumeType::File(_) => {}
        VolumeType::Memory => {}
    }
}

pub fn get_object_store(&self) -> catalog_metastore::error::Result<Arc<dyn ObjectStore>> {
    match &self.volume {
        VolumeType::S3(volume) => {
            let s3_builder = volume.get_s3_builder();
            s3_builder
                .build()
                .map(|s3| Arc::new(s3) as Arc<dyn ObjectStore>)
                .context(metastore_error::ObjectStoreSnafu)
        }
        VolumeType::S3Tables(volume) => {
            let s3_builder = volume.s3_builder();
            s3_builder
                .build()
                .map(|s3| Arc::new(s3) as Arc<dyn ObjectStore>)
                .context(metastore_error::ObjectStoreSnafu)
        }
        VolumeType::File(file) => Ok(Arc::new(
            LocalFileSystem::new_with_prefix(file.path.clone())
                .context(metastore_error::ObjectStoreSnafu)?
                .with_automatic_cleanup(true),
        ) as Arc<dyn ObjectStore>),
        VolumeType::Memory => {
            Ok(Arc::new(object_store::memory::InMemory::new()) as Arc<dyn ObjectStore>)
        }
    }
}