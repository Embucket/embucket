use crate::macros::make_udf_function;
use datafusion::arrow::datatypes::DataType;
use datafusion::error::Result as DFResult;
use datafusion::logical_expr::{ColumnarValue, Signature, Volatility};
use datafusion::physical_plan::internal_err;
use datafusion_expr::{ReturnInfo, ReturnTypeArgs, ScalarFunctionArgs, ScalarUDFImpl};
use std::any::Any;

#[derive(Debug)]
pub struct ToVariantFunc {
    signature: Signature,
}

impl Default for ToVariantFunc {
    fn default() -> Self {
        Self::new()
    }
}

impl ToVariantFunc {
    #[must_use]
    pub fn new() -> Self {
        Self {
            signature: Signature::any(1, Volatility::Immutable),
        }
    }
}

impl ScalarUDFImpl for ToVariantFunc {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &'static str {
        "to_variant"
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> datafusion_common::Result<DataType> {
        internal_err!("return_type_from_args should be called")
    }
    fn return_type_from_args(&self, args: ReturnTypeArgs) -> datafusion_common::Result<ReturnInfo> {
        Ok(ReturnInfo::new_nullable(args.arg_types[0].clone()))
    }

    fn invoke_with_args(&self, args: ScalarFunctionArgs) -> DFResult<ColumnarValue> {
        Ok(args.args[0].clone())
    }
}

make_udf_function!(ToVariantFunc);
