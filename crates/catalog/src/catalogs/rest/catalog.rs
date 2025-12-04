use async_trait::async_trait;
use iceberg_rest_catalog::apis::catalog_api_api;
use iceberg_rest_catalog::apis::configuration::Configuration;
use iceberg_rust::catalog::commit::CommitTable;
use iceberg_rust::{
    catalog::{
        Catalog,
        commit::CommitView,
        create::{CreateMaterializedView, CreateTable, CreateView},
        identifier::Identifier,
        namespace::Namespace,
        tabular::Tabular,
    },
    error::Error,
    materialized_view::MaterializedView,
    spec::identifier::FullIdentifier,
    table::Table,
    view::View,
};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug)]
pub struct RestCatalog {
    inner: Arc<dyn Catalog>,
    name: Option<String>,
    configuration: Configuration,
}

impl RestCatalog {
    pub fn new(
        name: Option<&str>,
        configuration: Configuration,
        catalog: Arc<dyn Catalog>,
    ) -> Self {
        Self {
            name: name.map(ToString::to_string),
            configuration,
            inner: catalog,
        }
    }
}

#[async_trait]
impl Catalog for RestCatalog {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn create_namespace(
        &self,
        namespace: &Namespace,
        properties: Option<HashMap<String, String>>,
    ) -> Result<HashMap<String, String>, Error> {
        self.inner.create_namespace(namespace, properties).await
    }

    async fn drop_namespace(&self, namespace: &Namespace) -> Result<(), Error> {
        self.inner.drop_namespace(namespace).await
    }

    async fn load_namespace(
        &self,
        namespace: &Namespace,
    ) -> Result<HashMap<String, String>, Error> {
        self.inner.load_namespace(namespace).await
    }

    async fn update_namespace(
        &self,
        namespace: &Namespace,
        updates: Option<HashMap<String, String>>,
        removals: Option<Vec<String>>,
    ) -> Result<(), Error> {
        self.inner
            .update_namespace(namespace, updates, removals)
            .await
    }

    async fn namespace_exists(&self, namespace: &Namespace) -> Result<bool, Error> {
        self.inner.namespace_exists(namespace).await
    }

    async fn list_tabulars(&self, namespace: &Namespace) -> Result<Vec<Identifier>, Error> {
        let tables = catalog_api_api::list_tables(
            &self.configuration,
            self.name.as_deref(),
            &namespace.to_string(),
            None,
            None,
        )
        .await
        .map_err(Into::<Error>::into)?;
        Ok(tables.identifiers.unwrap_or(Vec::new()))
    }

    async fn list_namespaces(&self, parent: Option<&str>) -> Result<Vec<Namespace>, Error> {
        self.inner.list_namespaces(parent).await
    }

    async fn tabular_exists(&self, identifier: &Identifier) -> Result<bool, Error> {
        self.inner.tabular_exists(identifier).await
    }

    async fn drop_table(&self, identifier: &Identifier) -> Result<(), Error> {
        self.inner.drop_table(identifier).await
    }

    async fn drop_view(&self, identifier: &Identifier) -> Result<(), Error> {
        self.inner.drop_view(identifier).await
    }

    async fn drop_materialized_view(&self, identifier: &Identifier) -> Result<(), Error> {
        self.inner.drop_materialized_view(identifier).await
    }

    async fn load_tabular(self: Arc<Self>, identifier: &Identifier) -> Result<Tabular, Error> {
        self.inner.clone().load_tabular(identifier).await
    }

    async fn create_table(
        self: Arc<Self>,
        identifier: Identifier,
        create_table: CreateTable,
    ) -> Result<Table, Error> {
        self.inner
            .clone()
            .create_table(identifier, create_table)
            .await
    }

    async fn create_view(
        self: Arc<Self>,
        identifier: Identifier,
        create_view: CreateView<Option<()>>,
    ) -> Result<View, Error> {
        self.inner
            .clone()
            .create_view(identifier, create_view)
            .await
    }

    async fn create_materialized_view(
        self: Arc<Self>,
        identifier: Identifier,
        create_view: CreateMaterializedView,
    ) -> Result<MaterializedView, Error> {
        self.inner
            .clone()
            .create_materialized_view(identifier, create_view)
            .await
    }

    async fn update_table(self: Arc<Self>, commit: CommitTable) -> Result<Table, Error> {
        self.inner.clone().update_table(commit).await
    }

    async fn update_view(self: Arc<Self>, commit: CommitView<Option<()>>) -> Result<View, Error> {
        self.inner.clone().update_view(commit).await
    }

    async fn update_materialized_view(
        self: Arc<Self>,
        commit: CommitView<FullIdentifier>,
    ) -> Result<MaterializedView, Error> {
        self.inner.clone().update_materialized_view(commit).await
    }

    async fn register_table(
        self: Arc<Self>,
        identifier: Identifier,
        metadata_location: &str,
    ) -> Result<Table, Error> {
        self.inner
            .clone()
            .register_table(identifier, metadata_location)
            .await
    }
}
