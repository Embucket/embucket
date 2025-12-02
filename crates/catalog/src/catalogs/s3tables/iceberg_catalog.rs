use std::{collections::HashMap, sync::Arc};

use crate::error::{self as catalog_error, Result as CatalogResult};
use async_trait::async_trait;
use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_credential_types::provider::SharedCredentialsProvider;
use catalog_metastore::S3TablesVolume;
use iceberg_rest_catalog::apis::configuration::{AWSv4Key, Configuration};
use iceberg_rest_catalog::catalog::RestCatalog;
use iceberg_rust::object_store::ObjectStoreBuilder;
use iceberg_rust::{
    catalog::{
        Catalog,
        commit::{CommitTable, CommitView},
        create::{CreateMaterializedView, CreateTable, CreateView},
        identifier::Identifier,
        namespace::Namespace,
        tabular::Tabular,
    },
    error::Error as IcebergError,
    materialized_view::MaterializedView,
    spec::identifier::FullIdentifier,
    table::Table,
    view::View,
};
use iceberg_s3tables_catalog::S3TablesCatalog as S3Tables;
use secrecy::SecretString;
use snafu::ResultExt;

#[derive(Debug)]
pub struct S3TablesCatalog {
    inner: Arc<dyn Catalog>,
    rest: Arc<dyn Catalog>,
}

impl S3TablesCatalog {
    #[allow(clippy::result_large_err)]
    pub async fn new(
        access_key: String,
        secret_key: String,
        session_token: Option<String>,
        volume: S3TablesVolume,
    ) -> CatalogResult<Self> {
        let s3_tables_creds = Credentials::from_keys(
            access_key.clone(),
            secret_key.clone(),
            session_token.clone(),
        );
        let config = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(SharedCredentialsProvider::new(s3_tables_creds))
            .region(Region::new(volume.region()))
            .load()
            .await;
        let builder = ObjectStoreBuilder::S3(Box::new(volume.s3_builder()));
        let s3_tables_catalog: Arc<dyn Catalog> = Arc::new(
            S3Tables::new(&config, volume.arn.as_str(), builder)
                .context(catalog_error::S3TablesSnafu)?,
        );
        let base_path = volume.endpoint.clone().unwrap_or_else(|| {
            format!("https://s3tables.{}.amazonaws.com/iceberg", volume.region())
        });
        let rest_catalog: Arc<dyn Catalog> = Arc::new(RestCatalog::new(
            Some(volume.arn.as_str()),
            Configuration {
                base_path,
                aws_v4_key: Some(AWSv4Key {
                    access_key: access_key.clone(),
                    secret_key: SecretString::new(secret_key),
                    session_token: session_token.clone().map(SecretString::new),
                    region: volume.region(),
                    service: "s3tables".to_string(),
                }),
                ..Default::default()
            },
            Some(ObjectStoreBuilder::S3(Box::new(volume.s3_builder()))),
        ));
        Ok(Self {
            inner: s3_tables_catalog,
            rest: rest_catalog,
        })
    }
}

#[async_trait]
impl Catalog for S3TablesCatalog {
    fn name(&self) -> &str {
        self.inner.name()
    }
    async fn create_namespace(
        &self,
        namespace: &Namespace,
        properties: Option<HashMap<String, String>>,
    ) -> Result<HashMap<String, String>, IcebergError> {
        self.inner.create_namespace(namespace, properties).await
    }
    async fn drop_namespace(&self, namespace: &Namespace) -> Result<(), IcebergError> {
        self.inner.drop_namespace(namespace).await
    }
    async fn load_namespace(
        &self,
        namespace: &Namespace,
    ) -> Result<HashMap<String, String>, IcebergError> {
        self.inner.load_namespace(namespace).await
    }
    async fn update_namespace(
        &self,
        namespace: &Namespace,
        updates: Option<HashMap<String, String>>,
        removals: Option<Vec<String>>,
    ) -> Result<(), IcebergError> {
        self.inner
            .update_namespace(namespace, updates, removals)
            .await
    }
    async fn namespace_exists(&self, namespace: &Namespace) -> Result<bool, IcebergError> {
        self.rest.clone().namespace_exists(namespace).await
    }
    async fn list_tabulars(&self, namespace: &Namespace) -> Result<Vec<Identifier>, IcebergError> {
        self.inner.list_tabulars(namespace).await
    }
    async fn list_namespaces(&self, parent: Option<&str>) -> Result<Vec<Namespace>, IcebergError> {
        self.inner.list_namespaces(parent).await
    }
    async fn tabular_exists(&self, identifier: &Identifier) -> Result<bool, IcebergError> {
        self.inner.tabular_exists(identifier).await
    }
    async fn drop_table(&self, identifier: &Identifier) -> Result<(), IcebergError> {
        self.inner.drop_table(identifier).await
    }
    async fn drop_view(&self, identifier: &Identifier) -> Result<(), IcebergError> {
        self.inner.drop_view(identifier).await
    }
    async fn drop_materialized_view(&self, identifier: &Identifier) -> Result<(), IcebergError> {
        self.inner.drop_materialized_view(identifier).await
    }
    async fn load_tabular(
        self: Arc<Self>,
        identifier: &Identifier,
    ) -> Result<Tabular, IcebergError> {
        self.inner.clone().load_tabular(identifier).await
    }
    async fn create_table(
        self: Arc<Self>,
        identifier: Identifier,
        create_table: CreateTable,
    ) -> Result<Table, IcebergError> {
        self.rest
            .clone()
            .create_table(identifier, create_table)
            .await
    }
    async fn create_view(
        self: Arc<Self>,
        identifier: Identifier,
        create_view: CreateView<Option<()>>,
    ) -> Result<View, IcebergError> {
        self.rest.clone().create_view(identifier, create_view).await
    }
    async fn create_materialized_view(
        self: Arc<Self>,
        identifier: Identifier,
        create_view: CreateMaterializedView,
    ) -> Result<MaterializedView, IcebergError> {
        self.rest
            .clone()
            .create_materialized_view(identifier, create_view)
            .await
    }
    async fn update_table(self: Arc<Self>, commit: CommitTable) -> Result<Table, IcebergError> {
        self.inner.clone().update_table(commit).await
    }
    async fn update_view(
        self: Arc<Self>,
        commit: CommitView<Option<()>>,
    ) -> Result<View, IcebergError> {
        self.inner.clone().update_view(commit).await
    }
    async fn update_materialized_view(
        self: Arc<Self>,
        commit: CommitView<FullIdentifier>,
    ) -> Result<MaterializedView, IcebergError> {
        self.inner.clone().update_materialized_view(commit).await
    }

    async fn register_table(
        self: Arc<Self>,
        identifier: Identifier,
        metadata_location: &str,
    ) -> Result<Table, IcebergError> {
        self.inner
            .clone()
            .register_table(identifier, metadata_location)
            .await
    }
}
