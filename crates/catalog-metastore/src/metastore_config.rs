use crate::{
    Database, Metastore, Schema, SchemaIdent, TableFormat, TableIdent, Volume, VolumeIdent,
};
use iceberg_rust::spec::table_metadata::TableMetadata;
use object_store::ObjectStore;
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
    #[serde(default)]
    should_refresh: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct DatabaseEntry {
    ident: String,
    volume: VolumeIdent,
    #[serde(default)]
    should_refresh: bool,
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
    Metastore { source: crate::error::Error },
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
            self.ensure_database(metastore.clone(), &db.ident, &db.volume, db.should_refresh)
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
            self.ensure_database(
                metastore,
                database,
                &entry.volume.ident,
                entry.should_refresh,
            )
            .await?;
        }

        Ok(())
    }

    async fn ensure_database(
        &self,
        metastore: Arc<dyn Metastore>,
        ident: &str,
        volume: &str,
        should_refresh: bool,
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
                        should_refresh,
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

    #[allow(clippy::too_many_lines)]
    async fn apply_table(
        &self,
        entry: &TableEntry,
        metastore: Arc<dyn Metastore>,
    ) -> Result<(), ConfigError> {
        use crate::models::volumes::AwsCredentials;
        use object_store::aws::AmazonS3Builder;

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

        // Parse metadata location to extract bucket and path
        // For S3 Tables, the metadata_location contains a different bucket than the volume
        let metadata_location = &entry.metadata_location;
        let url = url::Url::parse(metadata_location).map_err(|e| {
            ConfigError::InvalidMetadataLocation {
                table: table_name.clone(),
                reason: format!("Failed to parse metadata location URL: {e}"),
            }
        })?;

        // Extract bucket name and path from metadata location
        let bucket_name = url
            .host_str()
            .ok_or_else(|| ConfigError::InvalidMetadataLocation {
                table: table_name.clone(),
                reason: "Metadata location URL missing bucket name".to_string(),
            })?;
        let metadata_path = url.path().trim_start_matches('/');

        // Create object store for the metadata bucket using volume credentials
        // Get credentials from the volume configuration

        let s3_builder = match &volume.volume {
            crate::models::volumes::VolumeType::S3(s3_vol) => {
                // Use the S3 volume's credentials but with the metadata location's bucket
                let mut builder = AmazonS3Builder::new()
                    .with_bucket_name(bucket_name)
                    .with_region(
                        s3_vol
                            .region
                            .clone()
                            .unwrap_or_else(|| "us-east-2".to_string()),
                    );

                if let Some(credentials) = &s3_vol.credentials {
                    match credentials {
                        AwsCredentials::AccessKey(creds) => {
                            builder = builder
                                .with_access_key_id(creds.aws_access_key_id.clone())
                                .with_secret_access_key(creds.aws_secret_access_key.clone());
                        }
                        AwsCredentials::Token(token) => {
                            builder = builder.with_token(token.clone());
                        }
                    }
                }
                builder
            }
            crate::models::volumes::VolumeType::S3Tables(s3tables_vol) => {
                // Use S3 Tables credentials but with the metadata location's bucket
                let mut builder = AmazonS3Builder::new()
                    .with_bucket_name(bucket_name)
                    .with_region(s3tables_vol.region());

                match &s3tables_vol.credentials {
                    AwsCredentials::AccessKey(creds) => {
                        builder = builder
                            .with_access_key_id(creds.aws_access_key_id.clone())
                            .with_secret_access_key(creds.aws_secret_access_key.clone());
                    }
                    AwsCredentials::Token(token) => {
                        builder = builder.with_token(token.clone());
                    }
                }
                builder
            }
            _ => {
                return Err(ConfigError::InvalidMetadataLocation {
                    table: table_name.clone(),
                    reason: "Only S3 and S3Tables volumes are supported for metadata locations"
                        .to_string(),
                });
            }
        };

        let table_object_store: Arc<dyn ObjectStore> = Arc::new(s3_builder.build().map_err(
            |e| ConfigError::InvalidMetadataLocation {
                table: table_name.clone(),
                reason: format!("Failed to build object store: {e}"),
            },
        )?);

        let bytes = table_object_store
            .get(&metadata_path.into())
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

        let stored_table = crate::Table {
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
