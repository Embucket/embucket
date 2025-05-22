use clap::Parser;
use std::{net::SocketAddr, str::FromStr};

use crate::seed_assets::SeedVariant;

#[derive(Parser)]
#[command(version, about, long_about=None)]
pub struct CliOpts {
    #[arg(
        short,
        long,
        value_enum,
        env = "SEED_VARIANT",
        default_value = "typical",
        help = "Variant of seed to use"
    )]
    seed_variant: SeedVariant,

    #[arg(
        long,
        env = "SERVER_ADDRESS",
        required = true,
        default_value = "http://127.0.0.1:3000",
        help = "ip:port of embucket server"
    )]
    pub server_address: String,

    #[arg(long, env = "AUTH_USER", help = "User for auth")]
    pub auth_user: String,

    #[arg(long, env = "AUTH_PASSWORD", help = "Password for auth")]
    pub auth_password: String,
}

impl CliOpts {
    pub fn seed_variant(&self) -> SeedVariant {
        self.seed_variant
    }

    pub fn server_address(&self) -> SocketAddr {
        SocketAddr::from_str(&self.server_address).expect("Invalid address")
    }

    pub fn auth_user(&self) -> String {
        self.auth_user.clone()
    }

    pub fn auth_password(&self) -> String {
        self.auth_password.clone()
    }
}
