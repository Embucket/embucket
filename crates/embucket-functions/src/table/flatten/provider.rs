use crate::df_error;
use crate::json::{PathToken, get_json_value};
use crate::table::errors;
use crate::table::flatten::func::{FlattenTableFunc, path_to_string};
use arrow_schema::{Field, Schema, SchemaRef};
use async_trait::async_trait;
use datafusion::arrow::array::{
    Array, ArrayRef, StringArray, StringBuilder, UInt64Array, UInt64Builder,
};
use datafusion::arrow::datatypes::DataType;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::catalog::{Session, TableProvider};
use datafusion::datasource::provider_as_source;
use datafusion::execution::{SendableRecordBatchStream, SessionState, TaskContext};
use datafusion::logical_expr::{ColumnarValue, Expr};
use datafusion::physical_expr::{EquivalenceProperties, Partitioning, create_physical_expr};
use datafusion_common::tree_node::{TreeNode, TreeNodeRecursion};
use datafusion_common::{
    Column, DFSchema, Result as DFResult, Result, ScalarValue, TableReference,
};
use datafusion_expr::execution_props::ExecutionProps;
use datafusion_expr::{LogicalPlanBuilder, TableType};
use datafusion_physical_plan::common::collect;
use datafusion_physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion_physical_plan::memory::MemoryStream;
use datafusion_physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties};
use serde_json::Value;
use snafu::ResultExt;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::Ordering;

#[derive(Debug, Clone, Copy)]
pub enum FlattenMode {
    Both,
    Array,
    Object,
}

impl FlattenMode {
    pub const fn is_object(self) -> bool {
        matches!(self, Self::Object | Self::Both)
    }

    pub const fn is_array(self) -> bool {
        matches!(self, Self::Array | Self::Both)
    }
}

#[derive(Debug, Clone)]
pub struct FlattenArgs {
    pub input_expr: Expr,
    pub path: Vec<PathToken>,
    pub is_outer: bool,
    pub is_recursive: bool,
    pub mode: FlattenMode,
}

pub struct Out {
    pub seq: UInt64Builder,
    pub key: StringBuilder,
    pub path: StringBuilder,
    pub index: UInt64Builder,
    pub value: StringBuilder,
    pub this: StringBuilder,
    pub last_outer: Option<Value>,
}

#[derive(Debug)]
pub struct FlattenTableProvider {
    pub args: FlattenArgs,
    pub schema: Arc<DFSchema>,
}

impl FlattenTableProvider {
    pub fn new(args: FlattenArgs) -> DFResult<Self> {
        let schema_fields = vec![
            Field::new("SEQ", DataType::UInt64, false),
            Field::new("KEY", DataType::Utf8, true),
            Field::new("PATH", DataType::Utf8, true),
            Field::new("INDEX", DataType::UInt64, true),
            Field::new("VALUE", DataType::Utf8, true),
            Field::new("THIS", DataType::Utf8, true),
        ];
        let qualified_fields = schema_fields
            .into_iter()
            .map(|f| (None, Arc::new(f)))
            .collect::<Vec<(Option<TableReference>, Arc<Field>)>>();
        let schema = Arc::new(DFSchema::new_with_metadata(
            qualified_fields,
            HashMap::default(),
        )?);
        Ok(Self { args, schema })
    }
}

#[async_trait]
impl TableProvider for FlattenTableProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        normalize_schema(&self.schema.clone())
    }

    fn table_type(&self) -> TableType {
        TableType::Temporary
    }

    async fn scan(
        &self,
        state: &dyn Session,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let session_state = state
            .as_any()
            .downcast_ref::<SessionState>()
            .ok_or_else(|| errors::ExpectedSessionStateInFlattenSnafu.build())?;
        let schema = match projection {
            // Use normalized schema for projections to avoid logical/physical schemas missmatch
            Some(projection) => Arc::new(self.schema().project(projection)?),
            None => self.schema.inner().clone(),
        };
        let properties = PlanProperties::new(
            EquivalenceProperties::new(schema),
            Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );
        Ok(Arc::new(FlattenExec {
            args: self.args.clone(),
            schema: self.schema.inner().clone(),
            session_state: Arc::new(session_state.clone()),
            projection: projection.cloned(),
            filters: filters.to_vec(),
            limit,
            properties,
        }))
    }
}

pub struct FlattenExec {
    args: FlattenArgs,
    schema: Arc<Schema>,
    session_state: Arc<SessionState>,
    properties: PlanProperties,
    projection: Option<Vec<usize>>,
    filters: Vec<Expr>,
    limit: Option<usize>,
}

impl Debug for FlattenExec {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "FlattenExec")
    }
}

impl DisplayAs for FlattenExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut Formatter) -> fmt::Result {
        write!(f, "FlattenExec")
    }
}

impl ExecutionPlan for FlattenExec {
    fn name(&self) -> &'static str {
        "FlattenExec"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn properties(&self) -> &PlanProperties {
        &self.properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![]
    }

    fn with_new_children(
        self: Arc<Self>,
        _new_children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(Arc::new(Self {
            session_state: self.session_state.clone(),
            args: self.args.clone(),
            schema: self.schema.clone(),
            properties: self.properties.clone(),
            projection: self.projection.clone(),
            filters: self.filters.clone(),
            limit: self.limit,
        }))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let batches = self.get_input_batches(partition, context)?;
        let flatten_func = FlattenTableFunc::new();

        let mut all_batches = vec![];
        let mut last_outer: Option<Value> = None;
        for batch in batches {
            let array = batch
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| errors::ExpectedInputColumnToBeUtf8Snafu.build())?;

            flatten_func.row_id.fetch_add(1, Ordering::Acquire);

            let out = Rc::new(RefCell::new(Out {
                seq: UInt64Builder::new(),
                key: StringBuilder::new(),
                path: StringBuilder::new(),
                index: UInt64Builder::new(),
                value: StringBuilder::new(),
                this: StringBuilder::new(),
                last_outer: None,
            }));

            for i in 0..array.len() {
                let json_str = array.value(i);
                let json_val: Value = serde_json::from_str(json_str)
                    .context(df_error::FailedToDeserializeJsonSnafu)?;

                let Some(input) = get_json_value(&json_val, &self.args.path) else {
                    continue;
                };

                flatten_func.flatten(
                    input,
                    &self.args.path,
                    self.args.is_outer,
                    self.args.is_recursive,
                    &self.args.mode,
                    &out,
                )?;
            }

            let mut out = out.borrow_mut();
            let cols: Vec<ArrayRef> = vec![
                Arc::new(out.seq.finish()),
                Arc::new(out.key.finish()),
                Arc::new(out.path.finish()),
                Arc::new(out.index.finish()),
                Arc::new(out.value.finish()),
                Arc::new(out.this.finish()),
            ];

            last_outer.clone_from(&out.last_outer);
            let batch = RecordBatch::try_new(self.schema.clone(), cols)?;
            if batch.num_rows() > 0 {
                all_batches.push(batch);
            }
        }

        if all_batches.is_empty() {
            return Ok(Box::pin(MemoryStream::try_new(
                vec![Self::empty_record_batch(
                    self.schema.clone(),
                    &self.args.path,
                    last_outer,
                    self.args.is_outer,
                    flatten_func.row_id.load(Ordering::Acquire),
                )],
                self.schema.clone(),
                self.projection.clone(),
            )?));
        }

        Ok(Box::pin(
            MemoryStream::try_new(all_batches, self.schema.clone(), self.projection.clone())?
                .with_fetch(self.limit),
        ))
    }
}

impl FlattenExec {
    fn get_input_batches(
        &self,
        _partition: usize,
        _context: Arc<TaskContext>,
    ) -> Result<Vec<RecordBatch>> {
        let expr = self.args.input_expr.clone();

        // Fast path for literal input
        if let Expr::Literal(ScalarValue::Utf8(Some(s))) = expr {
            let array: ArrayRef = Arc::new(StringArray::from(vec![s]));
            let batch = RecordBatch::try_from_iter(vec![("input", array)])?;
            return Ok(vec![batch]);
        }

        // Evaluate the expression or plan to get the input batches
        let session_state = self.session_state.clone();
        futures::executor::block_on(async move {
            evaluate_expr_or_plan(&expr, session_state.as_ref()).await
        })
    }

    #[allow(clippy::unwrap_used, clippy::as_conversions)]
    #[must_use]
    pub fn empty_record_batch(
        schema: SchemaRef,
        path: &[PathToken],
        last_outer: Option<Value>,
        null: bool,
        row_id: u64,
    ) -> RecordBatch {
        let arrays: Vec<ArrayRef> = if null {
            let last_outer_ = last_outer.map(|v| serde_json::to_string_pretty(&v).unwrap());
            vec![
                Arc::new(UInt64Array::from(vec![row_id])) as ArrayRef,
                Arc::new(StringArray::new_null(1)) as ArrayRef,
                Arc::new(StringArray::from(vec![path_to_string(path)])) as ArrayRef,
                Arc::new(UInt64Array::new_null(1)) as ArrayRef,
                Arc::new(StringArray::new_null(1)) as ArrayRef,
                Arc::new(StringArray::from(vec![last_outer_])) as ArrayRef,
            ]
        } else {
            vec![
                Arc::new(UInt64Array::new_null(0)) as ArrayRef,
                Arc::new(StringArray::new_null(0)) as ArrayRef,
                Arc::new(StringArray::new_null(0)) as ArrayRef,
                Arc::new(UInt64Array::new_null(0)) as ArrayRef,
                Arc::new(StringArray::new_null(0)) as ArrayRef,
                Arc::new(StringArray::new_null(0)) as ArrayRef,
            ]
        };
        RecordBatch::try_new(schema, arrays).unwrap()
    }
}

fn extract_table_ref(expr: &Expr) -> Option<TableReference> {
    let mut table_ref: Option<TableReference> = None;
    let _ = expr.apply(&mut |e: &Expr| {
        if let Expr::Column(Column {
            relation: Some(r), ..
        }) = e
        {
            table_ref = Some(r.clone());
        }
        Ok(TreeNodeRecursion::Continue)
    });
    table_ref
}

async fn evaluate_expr_or_plan(
    expr: &Expr,
    session_state: &SessionState,
) -> Result<Vec<RecordBatch>> {
    match extract_table_ref(expr) {
        // Evaluates the expression directly without column references
        None => {
            let exec_props = ExecutionProps::new();
            let phys_expr = create_physical_expr(expr, &DFSchema::empty(), &exec_props)?;
            let batch = RecordBatch::new_empty(Arc::new(Schema::empty()));
            let result = phys_expr.evaluate(&batch)?;

            let array = match result {
                ColumnarValue::Scalar(scalar) => scalar.to_array()?,
                ColumnarValue::Array(array) => array,
            };
            let batch = RecordBatch::try_from_iter(vec![("input", array)])?;
            Ok(vec![batch])
        }
        // If the expression contains a table reference, execute the plan
        Some(table_ref) => {
            let table_ref_cloned = table_ref.clone();
            let expr_cloned = expr.clone();

            let table = session_state
                .schema_for_ref(table_ref)?
                .table(table_ref_cloned.table())
                .await?
                .ok_or_else(|| errors::NoTableFoundForReferenceInExpressionSnafu.build())?;

            let plan = LogicalPlanBuilder::scan(
                table_ref_cloned.table(),
                provider_as_source(table),
                None,
            )?
            .project(vec![expr_cloned.alias("input")])?
            .build()?;
            let physical_plan = session_state.create_physical_plan(&plan).await?;
            let input_stream = physical_plan.execute(0, session_state.task_ctx())?;
            collect(input_stream).await
        }
    }
}

pub fn normalize_schema(schema: &DFSchema) -> SchemaRef {
    let fields = schema
        .fields()
        .iter()
        .map(|field| {
            Arc::new(Field::new(
                field.name().to_ascii_lowercase(),
                field.data_type().clone(),
                field.is_nullable(),
            ))
        })
        .collect::<Vec<_>>();

    Arc::new(Schema::new(fields))
}
