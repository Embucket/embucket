pub mod error;
pub mod models;
pub mod crud;

pub type Connection = Object<AsyncDieselConnectionManager<AsyncPgConnection>>;

use async_trait::async_trait;
use deadpool::managed::Object;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::{AsyncPgConnection, AsyncMigrationHarness};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use error::Result;
use error::{PoolSnafu, GenericSnafu};
use snafu::ResultExt;
use tracing::instrument;
use models::Query;

pub const QUERIES_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

#[async_trait]
pub trait Queries: Send + Sync {
    async fn add(&self, item: Query) -> Result<Query>;
}

pub struct QueriesDb {
    pub pool: Pool<AsyncPgConnection>,
}

impl QueriesDb {
    pub async fn new(pool: Pool<AsyncPgConnection>) -> Result<Self> {
        let queries_db = Self { pool };
        queries_db.ensure_tables().await?;
        Ok(queries_db)
    }

    pub async fn connection(&self) -> Result<Connection> {
        self.pool.get().await.context(PoolSnafu)
    }

    #[instrument(name = "QueriesDb::ensure_tables", level = "debug", skip(self), fields(ok), err)]
    pub async fn ensure_tables(&self) -> Result<()> {
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
        crud::create_query(&mut conn, item).await
    }
}
