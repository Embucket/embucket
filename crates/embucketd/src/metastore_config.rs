use catalog_metastore::{
    Database, Metastore, Schema, SchemaIdent, TableFormat, TableIdent, Volume, VolumeIdent,
};
use iceberg_rust::spec::table_metadata::TableMetadata;
use iceberg_rust::spec::util::strip_prefix;
use serde::Deserialize;
use serde_json::Value;
use snafu::prelude::*;
use std::collections::HashMap;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::fs;

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
    #[snafu(display("Invalid metadata location for table {table}: {reason}"))]
    InvalidMetadataLocation { table: String, reason: String },
    #[snafu(display("Invalid metadata"))]
    InvalidMetadata,
    #[snafu(display("Failed to fetch metadata for table {table}: {source}"))]
    MetadataFetch {
        table: String,
        #[snafu(source)]
        source: object_store::Error,
    },
    #[snafu(display("Failed to parse metadata for table {table}: {source}"))]
    MetadataParse {
        table: String,
        #[snafu(source)]
        source: serde_json::Error,
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
        let table_name = entry.table.clone();
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
        let table_object_store = volume.get_object_store().context(MetastoreSnafu)?;

        let bytes = table_object_store
            .get(
                &strip_prefix(&entry.metadata_location.clone())
                    .as_str()
                    .into(),
            )
            .await
            .map_err(|e| ConfigError::InvalidMetadataLocation {
                table: table_name.clone(),
                reason: e.to_string(),
            })?
            .bytes()
            .await
            .context(MetadataFetchSnafu {
                table: table_name.clone(),
            })?;

        let json_val: Value = serde_json::from_slice(&bytes).context(MetadataParseSnafu {
            table: table_name.clone(),
        })?;

        // Patch missing iceberg spec fields
        let json_val = patch_missing_operation(json_val)?;

        // Convert back to bytes
        let patched_bytes = serde_json::to_vec(&json_val).context(MetadataParseSnafu {
            table: table_name.clone(),
        })?;
        // Deserialize normally
        let metadata: TableMetadata =
            serde_json::from_slice(&patched_bytes).context(MetadataParseSnafu {
                table: table_name.clone(),
            })?;

        let stored_table = catalog_metastore::Table {
            ident: table_ident.clone(),
            metadata,
            metadata_location: entry.metadata_location.clone(),
            properties: HashMap::default(),
            volume_ident: Some(volume.ident.clone()),
            volume_location: None,
            is_temporary: false,
            format: TableFormat::Iceberg,
        };
        metastore
            .register_table(&table_ident, stored_table)
            .await
            .context(MetastoreSnafu)?;
        Ok(())
    }
}

fn patch_missing_operation(mut value: Value) -> Result<Value, ConfigError> {
    if let Some(snapshots) = value.get_mut("snapshots").and_then(|v| v.as_array_mut()) {
        for snapshot in snapshots {
            if let Some(summary) = snapshot.get_mut("summary")
                && summary.get("operation").is_none()
            {
                summary
                    .as_object_mut()
                    .context(InvalidMetadataSnafu)?
                    .insert("operation".to_string(), Value::String("append".into()));
            }
        }
    }
    Ok(value)
}
