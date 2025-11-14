use catalog_metastore::{Database, Metastore, Schema, SchemaIdent, Volume, VolumeIdent};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::fs;
use tracing::{debug, info};

#[derive(Debug, Deserialize, Default)]
pub struct MetastoreBootstrapConfig {
    #[serde(default)]
    volumes: Vec<VolumeEntry>,
    #[serde(default)]
    databases: Vec<DatabaseEntry>,
    #[serde(default)]
    schemas: Vec<SchemaEntry>,
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

#[derive(Debug)]
pub enum BootstrapError {
    ReadConfig {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseConfig {
        path: PathBuf,
        source: serde_yaml::Error,
    },
    Metastore {
        source: catalog_metastore::error::Error,
    },
}

impl std::fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadConfig { path, source } => {
                write!(
                    f,
                    "Failed to read metastore config {}: {source}",
                    path.display()
                )
            }
            Self::ParseConfig { path, source } => {
                write!(
                    f,
                    "Failed to parse metastore config {}: {source}",
                    path.display()
                )
            }
            Self::Metastore { source } => write!(f, "Metastore bootstrap error: {source}"),
        }
    }
}

impl std::error::Error for BootstrapError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReadConfig { source, .. } => Some(source),
            Self::ParseConfig { source, .. } => Some(source),
            Self::Metastore { source } => Some(source),
        }
    }
}

impl MetastoreBootstrapConfig {
    pub async fn load(path: &Path) -> Result<Self, BootstrapError> {
        let contents =
            fs::read_to_string(path)
                .await
                .map_err(|source| BootstrapError::ReadConfig {
                    path: path.to_path_buf(),
                    source,
                })?;
        serde_yaml::from_str(&contents).map_err(|source| BootstrapError::ParseConfig {
            path: path.to_path_buf(),
            source,
        })
    }

    pub async fn apply(&self, metastore: Arc<dyn Metastore>) -> Result<(), BootstrapError> {
        for volume_entry in &self.volumes {
            self.apply_volume(volume_entry, metastore.clone()).await?;
        }

        for database_entry in &self.databases {
            self.ensure_database(
                metastore.clone(),
                &database_entry.ident,
                &database_entry.volume,
            )
            .await?;
        }

        for schema_entry in &self.schemas {
            self.ensure_schema(
                metastore.clone(),
                &schema_entry.database,
                &schema_entry.schema,
            )
            .await?;
        }

        Ok(())
    }

    async fn apply_volume(
        &self,
        entry: &VolumeEntry,
        metastore: Arc<dyn Metastore>,
    ) -> Result<(), BootstrapError> {
        if metastore
            .get_volume(&entry.volume.ident)
            .await
            .map_err(|source| BootstrapError::Metastore { source })?
            .is_none()
        {
            info!(
                volume = %entry.volume.ident,
                "Creating volume from metastore config"
            );
            metastore
                .create_volume(&entry.volume.ident, entry.volume.clone())
                .await
                .map_err(|source| BootstrapError::Metastore { source })?;
        } else {
            debug!(
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
    ) -> Result<(), BootstrapError> {
        if metastore
            .get_database(&ident.to_string())
            .await
            .map_err(|source| BootstrapError::Metastore { source })?
            .is_none()
        {
            info!(database = ident, volume, "Creating database from config");
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
                .map_err(|source| BootstrapError::Metastore { source })?;
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
    ) -> Result<(), BootstrapError> {
        let schema_ident = SchemaIdent::new(database.to_string(), schema.to_string());
        if metastore
            .get_schema(&schema_ident)
            .await
            .map_err(|source| BootstrapError::Metastore { source })?
            .is_none()
        {
            info!(
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
                .map_err(|source| BootstrapError::Metastore { source })?;
        }
        Ok(())
    }
}

const DEFAULT_SCHEMA_NAME: &str = "public";
