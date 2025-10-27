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

#[cfg(feature = "velox")]
pub mod velox_impl {
    use super::{AcceleratorKind, ExternalAccelerator};
    use datafusion::execution::SessionState;
    use datafusion::logical_expr::LogicalPlan;
    use datafusion::execution::SendableRecordBatchStream;
    use datafusion_common::DataFusionError;
    use velox_ffi::{VeloxConfig, VeloxSession};
    use futures::StreamExt;
    use datafusion_physical_plan::stream::RecordBatchStreamAdapter;
    use prost::Message;
    use snafu::prelude::*;
    use datafusion_substrait::substrait::proto as sp;

    fn plan_has_local_files(plan: &sp::Plan) -> bool {
        fn rel_opt_box_has_local_files(rel: &Option<Box<sp::Rel>>) -> bool {
            match rel {
                Some(b) => rel_has_local_files(b.as_ref()),
                None => false,
            }
        }
        fn rel_opt_has_local_files(rel: &Option<sp::Rel>) -> bool {
            match rel {
                Some(r) => rel_has_local_files(r),
                None => false,
            }
        }
        fn rel_has_local_files(rel: &sp::Rel) -> bool {
            match rel.rel_type.as_ref() {
                Some(sp::rel::RelType::Read(r)) => match r.read_type.as_ref() {
                    Some(sp::read_rel::ReadType::LocalFiles(_)) => true,
                    _ => false,
                },
                Some(sp::rel::RelType::Project(p)) => rel_opt_box_has_local_files(&p.input),
                Some(sp::rel::RelType::Filter(f)) => rel_opt_box_has_local_files(&f.input),
                Some(sp::rel::RelType::Fetch(f)) => rel_opt_box_has_local_files(&f.input),
                Some(sp::rel::RelType::Aggregate(a)) => rel_opt_box_has_local_files(&a.input),
                Some(sp::rel::RelType::Join(j)) => rel_opt_box_has_local_files(&j.left) || rel_opt_box_has_local_files(&j.right),
                Some(sp::rel::RelType::Sort(s)) => rel_opt_box_has_local_files(&s.input),
                Some(sp::rel::RelType::Set(s)) => s.inputs.iter().any(|r| rel_has_local_files(r)),
                Some(sp::rel::RelType::Exchange(e)) => rel_opt_box_has_local_files(&e.input),
                Some(sp::rel::RelType::Expand(e)) => rel_opt_box_has_local_files(&e.input),
                Some(sp::rel::RelType::Cross(c)) => rel_opt_box_has_local_files(&c.left) || rel_opt_box_has_local_files(&c.right),
                _ => false,
            }
        }
        plan.relations.iter().any(|rr| match rr.rel_type.as_ref() {
            Some(sp::plan_rel::RelType::Rel(r)) => rel_has_local_files(r),
            Some(sp::plan_rel::RelType::Root(root)) => rel_opt_has_local_files(&root.input),
            _ => false,
        })
    }

    pub struct VeloxAccelerator {
        _cfg: VeloxConfig,
    }

    impl VeloxAccelerator {
        pub fn new(cfg: VeloxConfig) -> Self { Self { _cfg: cfg } }
    }

    #[async_trait::async_trait]
    impl ExternalAccelerator for VeloxAccelerator {
        fn kind(&self) -> AcceleratorKind { AcceleratorKind::Velox }

        async fn execute(&self, plan: &LogicalPlan, state: &SessionState) -> crate::Result<SendableRecordBatchStream> {
            let substrait = datafusion_substrait::logical_plan::producer::to_substrait_plan(plan, state)
                .context(crate::error::DataFusionSnafu)?;
            let has_local_files = plan_has_local_files(&substrait);
            let bytes = substrait.encode_to_vec();
            let mut session = VeloxSession::new(self._cfg.clone())
                .map_err(|e| DataFusionError::Execution(e.to_string()))
                .context(crate::error::DataFusionSnafu)?;
            // For now, detection is informational; later we will register Arrow tables when !has_local_files
            let _prefer_files = has_local_files;
            let stream = session.execute_substrait_to_arrow_stream(&bytes)
                .map_err(|e| DataFusionError::Execution(e.to_string()))
                .context(crate::error::DataFusionSnafu)?;
            let (schema, it) = stream.into_batches();
            // Adapt iterator of RecordBatch into async stream (collect into Vec first to avoid trait object map issues)
            let batches: Vec<_> = it.collect();
            let s = futures::stream::iter(batches.into_iter().map(|b| Ok::<_, DataFusionError>(b)));
            let s: SendableRecordBatchStream = Box::pin(RecordBatchStreamAdapter::new(schema, Box::pin(s)));
            Ok(s)
        }
    }
}



