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
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into())
                .add_directive("hyper=off".parse().unwrap()),
        )
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
