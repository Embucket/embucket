use snafu::ResultExt;
use std::net::SocketAddr;

use crate::requests::rest_api_client::{RestApiClient, RestClient};
use crate::external_models;
use super::error::{LoadSeedSnafu, RequestSnafu, SeedResult};
use crate::seed_generator::parse_seed_template;
use crate::seed_models::Volume;
use crate::static_seed_assets::SeedVariant;

pub struct SeedClient {
    pub seed_data: Vec<Volume>,
    pub client: Box<dyn RestApiClient + Send>,
}

impl SeedClient {
    #[must_use]
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            seed_data: vec![],
            client: Box::new(RestClient::new(addr)),
        }
    }

    pub fn try_load_seed_template(&mut self, seed_variant: SeedVariant) -> SeedResult<()> {
        let raw_seed_data = parse_seed_template(seed_variant.seed_data()).context(LoadSeedSnafu)?;
        self.seed_data = raw_seed_data.generate();
        Ok(())
    }

    pub async fn login(&mut self, username: &str, password: &str) -> SeedResult<()> {
        self.client
            .login(username, password)
            .await
            .context(RequestSnafu)?;
        Ok(())
    }

    pub async fn seed_all(&mut self) -> SeedResult<usize> {
        let mut seeded_entities: usize = 0;
        for seed_volume in &self.seed_data {
            let volume: external_models::VolumePayload = seed_volume.clone().into();
            self.client
                .create_volume(volume)
                .await
                .context(RequestSnafu)?;
            tracing::debug!("Created volume: {}", seed_volume.volume_name);
            seeded_entities += 1;

            for seed_database in &seed_volume.databases {
                self.client
                    .create_database(&seed_volume.volume_name, &seed_database.database_name)
                    .await
                    .context(RequestSnafu)?;
                tracing::debug!("Created database: {}", seed_database.database_name);
                seeded_entities += 1;

                for seed_schema in &seed_database.schemas {
                    self.client
                        .create_schema(&seed_database.database_name, &seed_schema.schema_name)
                        .await
                        .context(RequestSnafu)?;
                    tracing::debug!("Created schema: {}", seed_schema.schema_name);
                    seeded_entities += 1;

                    for seed_table in &seed_schema.tables {
                        let table_columns: Vec<(String, String)> = seed_table
                            .columns
                            .iter()
                            .map(|col| (col.col_name.clone(), format!("{}", col.col_type)))
                            .collect();

                        self.client
                            .create_table(
                                &seed_database.database_name,
                                &seed_schema.schema_name,
                                &seed_table.table_name,
                                table_columns.as_slice(),
                            )
                            .await
                            .context(RequestSnafu)?;
                        tracing::debug!("Created table: {}", seed_table.table_name);
                        seeded_entities += 1;
                    }
                }
            }
        }
        Ok(seeded_entities)
    }
}

pub async fn seed_database(
    addr: SocketAddr,
    seed_variant: SeedVariant,
    user: String,
    pass: String,
) {
    let mut seed_client = SeedClient::new(addr);

    tracing::info!("Preparing seed data variant: {seed_variant:?}...");

    if let Err(err) = seed_client.try_load_seed_template(seed_variant) {
        tracing::warn!("Seed client failed to load seed template: {err}");
        return;
    }

    if let Err(err) = seed_client.login(&user, &pass).await {
        tracing::warn!("Seed client failed to login on server: {err}");
        return;
    }

    tracing::info!("Seeding started!");

    match seed_client.seed_all().await {
        Ok(seeded_entities_count) => {
            tracing::info!("Seeding finished, seeded {seeded_entities_count} entities!")
        }
        Err(err) => tracing::error!("Seeding error: {err}"),
    };
}
