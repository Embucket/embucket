pub(crate) mod error;
pub(crate) mod fake_provider;
pub(crate) mod models;

pub use models::*;

pub fn parse_seed_template(seed_template: &str) -> Result<VolumesRoot, serde_yaml::Error> {
    serde_yaml::from_str::<VolumesRoot>(seed_template)
}
