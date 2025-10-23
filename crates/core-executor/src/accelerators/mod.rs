use datafusion::execution::SessionState;
use datafusion::logical_expr::LogicalPlan;

pub mod flight;

/// Backends supported for external acceleration
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcceleratorKind {
    Acero,
    Velox,
}

impl AcceleratorKind {
    #[must_use]
    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "acero" => Some(Self::Acero),
            "velox" => Some(Self::Velox),
            _ => None,
        }
    }
}

/// Unified interface for external accelerators that consume Substrait plans
#[async_trait::async_trait]
pub trait ExternalAccelerator: Send + Sync {
    /// A human-readable name for diagnostics
    fn kind(&self) -> AcceleratorKind;

    /// Execute a Substrait plan represented by the DataFusion logical plan and session state.
    /// Implementations are expected to convert the logical plan to Substrait and execute it.
    async fn execute(
        &self,
        _plan: &LogicalPlan,
        _state: &SessionState,
    ) -> crate::Result<datafusion::execution::SendableRecordBatchStream>;
}


