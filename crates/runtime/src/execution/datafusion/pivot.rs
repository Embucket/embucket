use datafusion::common::Result;
use datafusion::common::{plan_err, DFSchemaRef};
use datafusion::logical_expr::UserDefinedLogicalNode;
use datafusion::logical_expr::LogicalPlan;
use datafusion_common::Column;
use datafusion_common::ScalarValue;
use datafusion_expr::logical_plan::Aggregate;
use datafusion_expr::InvariantLevel;
use std::any::Any;
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Custom implementation of PIVOT as a UserDefinedLogicalNode
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PivotPlan {
    pub input: Arc<LogicalPlan>,
    pub aggregate_expr: datafusion_expr::Expr,
    pub pivot_column: Column,
    pub pivot_values: Vec<ScalarValue>,
    pub schema: DFSchemaRef,
    pub value_subquery: Option<Arc<LogicalPlan>>,
}

impl PivotPlan {
    pub fn try_new(
        input: Arc<LogicalPlan>,
        aggregate_expr: datafusion_expr::Expr,
        pivot_column: Column,
        pivot_values: Vec<ScalarValue>,
    ) -> Result<Self> {
        // Simplified schema for now - this would need proper implementation
        let schema = input.schema().clone();

        Ok(Self {
            input,
            aggregate_expr,
            pivot_column,
            pivot_values,
            schema,
            value_subquery: None,
        })
    }

    pub fn try_new_with_subquery(
        input: Arc<LogicalPlan>,
        aggregate_expr: datafusion_expr::Expr,
        pivot_column: Column,
        subquery: Arc<LogicalPlan>,
    ) -> Result<Self> {
        // Simplified schema for now - this would need proper implementation
        let schema = input.schema().clone();

        Ok(Self {
            input,
            aggregate_expr,
            pivot_column,
            pivot_values: vec![],
            schema,
            value_subquery: Some(subquery),
        })
    }

    /// Convert this pivot plan to a more standard aggregate + projection plan
    pub fn to_aggregate_plan(&self) -> Result<LogicalPlan> {
        // Simplified implementation - return a placeholder aggregate plan
        // that can be properly executed
        let dummy_plan = Aggregate::try_new(
            self.input.clone(),
            vec![],  // No grouping
            vec![],  // No aggregates
        )?;
        
        Ok(LogicalPlan::Aggregate(dummy_plan))
    }
}

impl UserDefinedLogicalNode for PivotPlan {
    fn name(&self) -> &str {
        "Pivot"
    }

    fn inputs(&self) -> Vec<&LogicalPlan> {
        match &self.value_subquery {
            Some(subquery) => vec![self.input.as_ref(), subquery.as_ref()],
            None => vec![self.input.as_ref()],
        }
    }

    fn schema(&self) -> &DFSchemaRef {
        &self.schema
    }

    fn expressions(&self) -> Vec<datafusion_expr::Expr> {
        vec![self.aggregate_expr.clone()]
    }

    fn fmt_for_explain(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Pivot: FOR {} IN ({} values)",
            self.pivot_column.name,
            self.pivot_values.len()
        )
    }

    fn with_exprs_and_inputs(
        &self,
        exprs: Vec<datafusion_expr::Expr>,
        inputs: Vec<LogicalPlan>,
    ) -> Result<Arc<dyn UserDefinedLogicalNode>> {
        if exprs.len() != 1 {
            return plan_err!("Pivot requires exactly one aggregate expression");
        }
        
        let aggregate_expr = exprs[0].clone();
        
        let (input, value_subquery) = match (inputs.len(), &self.value_subquery) {
            (1, None) => (Arc::new(inputs[0].clone()), None),
            (2, Some(_)) => (Arc::new(inputs[0].clone()), Some(Arc::new(inputs[1].clone()))),
            _ => return plan_err!("Pivot requires one input and optionally one subquery"),
        };
        
        let pivot = PivotPlan {
            input,
            aggregate_expr,
            pivot_column: self.pivot_column.clone(),
            pivot_values: self.pivot_values.clone(),
            schema: self.schema.clone(),
            value_subquery,
        };
        
        Ok(Arc::new(pivot))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_hash(&self, state: &mut dyn Hasher) {
        let mut s = state;
        self.hash(&mut s);
    }

    fn dyn_eq(&self, other: &dyn UserDefinedLogicalNode) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Self>() {
            self == other
        } else {
            false
        }
    }

    fn dyn_ord(&self, other: &dyn UserDefinedLogicalNode) -> Option<std::cmp::Ordering> {
        if let Some(other) = other.as_any().downcast_ref::<Self>() {
            Some(self.cmp(other))
        } else {
            None
        }
    }
    
    fn check_invariants(&self, _: InvariantLevel, _: &LogicalPlan) -> Result<()> {
        Ok(())
    }
}

impl PartialOrd for PivotPlan {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PivotPlan {
    fn cmp(&self, _other: &Self) -> std::cmp::Ordering {
        // Simple implementation for now
        std::cmp::Ordering::Equal
    }
}

impl Display for PivotPlan {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Pivot: FOR {} IN ({:?})",
            self.pivot_column.name, self.pivot_values
        )
    }
} 