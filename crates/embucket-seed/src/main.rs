pub(crate) mod cli;
pub(crate) mod client;
pub(crate) mod external_models;
pub(crate) mod seed;
pub(crate) mod seed_assets;
pub(crate) mod seed_database;

#[cfg(test)]
mod tests;

use clap::Parser;
use tracing_subscriber;

use seed_database::seed_database;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let opts = cli::CliOpts::parse();

    seed_database(
        opts.server_address(),
        opts.seed_variant(),
        opts.auth_user(),
        opts.auth_password(),
    )
    .await;
}
