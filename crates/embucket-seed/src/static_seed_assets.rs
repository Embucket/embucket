use clap::ValueEnum;

#[derive(Copy, Clone, ValueEnum, Debug)]
pub enum SeedVariant {
    Minimal,
    Typical,
    Extreme,
}

impl SeedVariant {
    pub fn seed_data(&self) -> &'static str {
        match self {
            SeedVariant::Minimal => include_str!("../templates/minimal_seed.yaml"),
            SeedVariant::Typical => include_str!("../templates/typical_seed.yaml"),
            SeedVariant::Extreme => include_str!("../templates/extreme_seed.yaml"),
        }
    }
}
