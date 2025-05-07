use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;

use datafusion::common::Result;
use datafusion::execution::context::{QueryPlanner, SessionState};
use datafusion::logical_expr::LogicalPlan;
use datafusion::physical_plan::ExecutionPlan;
use datafusion_iceberg::planner::IcebergQueryPlanner;

use super::pivot_planner::PivotExtensionPlanner;

/// Custom query planner that extends the IcebergQueryPlanner with our PivotExtensionPlanner
#[derive(Debug)]
pub struct CustomQueryPlanner {
    iceberg_planner: IcebergQueryPlanner,
    pivot_planner: PivotExtensionPlanner,
}

impl CustomQueryPlanner {
    pub fn new() -> Self {
        Self {
            iceberg_planner: IcebergQueryPlanner::new(),
            pivot_planner: PivotExtensionPlanner::new(),
        }
    }
    
    pub fn with_pivot_support() -> Self {
        Self::new()
    }
}

impl QueryPlanner for CustomQueryPlanner {
    #[allow(clippy::type_complexity)]
    fn create_physical_plan<'life0, 'life1, 'life2, 'async_trait>(
        &'life0 self,
        logical_plan: &'life1 LogicalPlan,
        session_state: &'life2 SessionState,
    ) -> Pin<Box<dyn Future<Output = Result<Arc<dyn ExecutionPlan>>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
    {
        Box::pin(async move {
            // Try the pivot planner first for Extension nodes
            if let LogicalPlan::Extension(extension) = logical_plan {
                if let Some(pivot_node) = extension.node.as_any().downcast_ref::<super::pivot::PivotPlan>() {
                    // Convert the logical PivotPlan to a standard aggregate plan
                    let aggregate_plan = match pivot_node.to_aggregate_plan() {
                        Ok(plan) => plan,
                        Err(e) => return Err(e),
                    };
                    
                    // Then delegate to the iceberg planner for the transformed plan
                    return self.iceberg_planner.create_physical_plan(&aggregate_plan, session_state).await;
                }
            }
            
            // If not a pivot node or conversion failed, delegate to the iceberg planner
            self.iceberg_planner.create_physical_plan(logical_plan, session_state).await
        })
    }
} 