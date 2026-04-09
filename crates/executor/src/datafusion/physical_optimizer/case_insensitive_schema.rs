use datafusion::error::Result as DFResult;
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion_common::config::ConfigOptions;
use datafusion_physical_plan::ExecutionPlan;
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct CaseInsensitiveSchemaDataSourceExec;

impl CaseInsensitiveSchemaDataSourceExec {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl PhysicalOptimizerRule for CaseInsensitiveSchemaDataSourceExec {
    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        _config: &ConfigOptions,
    ) -> DFResult<Arc<dyn ExecutionPlan>> {
        Ok(plan)
    }

    fn name(&self) -> &'static str {
        "CaseInsensitiveSchemaDataSourceExec"
    }

    fn schema_check(&self) -> bool {
        true
    }
}
