pub mod operations;
pub mod error;
pub mod models;

#[cfg(test)]
pub mod tests;

pub use models::QuerySource;
pub use models::QueryStatus;
pub use models::ResultFormat;
pub use models::list_params::*;

pub type Connection = Object<AsyncDieselConnectionManager<AsyncPgConnection>>;

use async_trait::async_trait;
use deadpool::managed::Object;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::{AsyncMigrationHarness, AsyncPgConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use error::Result;
use error::{GenericSnafu, PoolSnafu};
use models::Query;
use snafu::ResultExt;
use uuid::Uuid;
use tracing::instrument;

pub const QUERIES_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

#[async_trait]
pub trait Queries: Send + Sync {
    /// Add new query
    /// Returns inserted query
    async fn add(&self, item: Query) -> Result<Query>;

    /// Update those fields that makes sense for update
    /// Returns updated query
    async fn update(&self, item: Query) -> Result<Query>;

    /// Delete query by id
    /// Returns number of deleted rows
    async fn delete(&self, id: Uuid) -> Result<usize>;

    /// List queries
    /// Returns list of queries
    async fn list(&self, params: ListParams) -> Result<Vec<Query>>;
}

pub struct QueriesDb {
    pub pool: Pool<AsyncPgConnection>,
}

impl QueriesDb {
    pub async fn new(pool: Pool<AsyncPgConnection>) -> Result<Self> {
        let queries_db = Self { pool };
        queries_db.apply_migrations().await?;
        Ok(queries_db)
    }

    pub async fn connection(&self) -> Result<Connection> {
        self.pool.get().await.context(PoolSnafu)
    }

    #[instrument(
        name = "QueriesDb::apply_migrations",
        level = "debug",
        skip(self),
        fields(ok),
        err
    )]
    pub async fn apply_migrations(&self) -> Result<()> {
        let conn = self.connection().await?;
        let mut conn = AsyncMigrationHarness::from(conn);
        let migrations = conn
            .run_pending_migrations(QUERIES_MIGRATIONS)
            .context(GenericSnafu)?
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        tracing::info!("Applied migrations: {migrations:?}");
        Ok(())
    }
}

#[async_trait]
impl Queries for QueriesDb {
    #[instrument(name = "Queries::add", level = "debug", skip(self), err)]
    async fn add(&self, item: Query) -> Result<Query> {
        let mut conn = self.connection().await?;
        operations::create_query(&mut conn, item).await
    }

    #[instrument(name = "Queries::update", level = "debug", skip(self), err)]
    async fn update(&self, item: Query) -> Result<Query> {
        let mut conn = self.connection().await?;
        operations::update_query(&mut conn, item).await
    }

    #[instrument(name = "Queries::delete", level = "debug", skip(self), err)]
    async fn delete(&self, id: Uuid) -> Result<usize> {
        let mut conn = self.connection().await?;
        operations::delete_query(&mut conn, id).await
    }

    #[instrument(name = "Queries::list", level = "debug", skip(self), err)]
    async fn list(&self, params: ListParams) -> Result<Vec<Query>> {
        let mut conn = self.connection().await?;
        operations::list_queries(&mut conn, params).await
    }
}
