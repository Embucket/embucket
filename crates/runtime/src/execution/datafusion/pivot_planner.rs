use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;

use datafusion::common::Result;
use datafusion::execution::context::SessionState;
use datafusion::logical_expr::{LogicalPlan, UserDefinedLogicalNode};
use datafusion::physical_plan::ExecutionPlan;
use datafusion::physical_plan::PhysicalExpr;
use datafusion::physical_planner::{DefaultPhysicalPlanner, ExtensionPlanner, PhysicalPlanner};
use datafusion_common::DFSchema;

use super::pivot::PivotPlan;

/// A physical planner that knows how to plan our custom PivotPlan extension
pub struct PivotExtensionPlanner {
    // The base physical planner we'll delegate to for planning non-extension nodes
    inner: DefaultPhysicalPlanner,
}

impl PivotExtensionPlanner {
    pub fn new() -> Self {
        Self {
            inner: DefaultPhysicalPlanner::default(),
        }
    }
}

impl ExtensionPlanner for PivotExtensionPlanner {
    /// Create a physical plan for our custom PivotPlan extension node
    #[allow(clippy::type_complexity)]
    fn plan_extension<'life0, 'life1, 'life2, 'life3, 'life4, 'life5, 'life6, 'async_trait>(
        &'life0 self,
        planner: &'life1 dyn PhysicalPlanner,
        node: &'life2 dyn UserDefinedLogicalNode,
        _logical_inputs: &'life3 [&'life4 LogicalPlan],
        _physical_inputs: &'life5 [Arc<dyn ExecutionPlan + 'static>],
        session_state: &'life6 SessionState,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Arc<dyn ExecutionPlan>>>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        'life3: 'async_trait,
        'life4: 'async_trait,
        'life5: 'async_trait,
        'life6: 'async_trait,
    {
        Box::pin(async move {
            // Try to downcast the extension node to our PivotPlan
            if let Some(pivot_plan) = node.as_any().downcast_ref::<PivotPlan>() {
                // Convert the logical PivotPlan to a standard aggregate plan
                let aggregate_plan = pivot_plan.to_aggregate_plan()?;
                
                // Plan the transformed aggregate plan using the regular planner
                let physical_plan = planner.create_physical_plan(&aggregate_plan, session_state).await?;
                Ok(Some(physical_plan))
            } else {
                // This ExtensionPlanner can only plan PivotPlan nodes
                Ok(None)
            }
        })
    }
}

impl PhysicalPlanner for PivotExtensionPlanner {
    fn create_physical_expr(
        &self,
        expr: &datafusion_expr::Expr,
        input_schema: &DFSchema,
        session_state: &SessionState,
    ) -> Result<Arc<dyn PhysicalExpr>> {
        // Delegate to the inner planner
        self.inner.create_physical_expr(expr, input_schema, session_state)
    }

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
            match logical_plan {
                LogicalPlan::Extension(extension) => {
                    // Try to downcast to our specific plan type first
                    if let Some(result) = self.plan_extension(
                        self,
                        extension.node.as_ref(),
                        &extension.node.inputs(),
                        &[], // No physical inputs yet
                        session_state,
                    ).await? {
                        Ok(result)
                    } else {
                        // If it's another type of extension, delegate to the inner planner
                        self.inner.create_physical_plan(logical_plan, session_state).await
                    }
                }
                // For all other logical plan nodes, delegate to the inner planner
                _ => self.inner.create_physical_plan(logical_plan, session_state).await,
            }
        })
    }
} 