pub mod fake_provider;
pub mod generator;

pub use generator::*;

use crate::seed_models::VolumesRoot;

pub fn parse_seed_template(seed_template: &str) -> Result<VolumesRoot, serde_yaml::Error> {
    serde_yaml::from_str::<VolumesRoot>(seed_template)
}
