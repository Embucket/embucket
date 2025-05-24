use serde::{Deserialize, Serialize};

use crate::seed_generator::Generator;
use crate::seed_models::{Volume, VolumeGenerator};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VolumesRoot {
    // every volume added explicitely, no volume items auto-generated
    pub volumes: Vec<VolumeGenerator>,
}

impl VolumesRoot {
    #[must_use]
    pub fn generate(&self) -> Vec<Volume> {
        self.volumes
            .iter()
            .enumerate()
            .map(|(i, v)| v.generate(i))
            .collect()
    }
}
