use datafusion::arrow::datatypes::DataType;
use datafusion::arrow::{
    array::{Array, as_string_array},
    compute,
};
use datafusion::common::Result;
use datafusion::functions::crypto::md5::Md5Func as DataFusionMd5Func;
use datafusion::logical_expr::{ColumnarValue, ScalarUDFImpl};
use datafusion_common::ScalarValue;
use datafusion_expr::ScalarFunctionArgs;
use datafusion_expr::{Signature, TypeSignature, Volatility};
use std::any::Any;
use std::sync::Arc;

/// `MD5` SQL function
///
/// Returns a 32-character hex-encoded string containing the 128-bit MD5 message digest.
///
/// Syntax: `MD5`(<msg>), `MD5_HEX`(<msg>)
///
/// Arguments
/// - `msg`: A string expression, the message to be hashed.
///
/// Returns a 32-character hex-encoded string.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Md5Func {
    inner: DataFusionMd5Func,
    signature: Signature,
    aliases: Vec<String>,
}
impl Default for Md5Func {
    fn default() -> Self {
        Self::new()
    }
}

impl Md5Func {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: DataFusionMd5Func::new(),
            signature: Signature::one_of(
                vec![
                    TypeSignature::Exact(vec![DataType::Utf8]),
                    TypeSignature::Any(1),
                ],
                Volatility::Immutable,
            ),
            aliases: vec!["md5_hex".to_string()],
        }
    }
}
impl ScalarUDFImpl for Md5Func {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &'static str {
        "md5"
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        self.inner.return_type(_arg_types)
    }

    fn invoke_with_args(&self, mut args: ScalarFunctionArgs) -> Result<ColumnarValue> {
        let coerced_arg = match args.args.remove(0) {
            ColumnarValue::Scalar(value) => {
                ColumnarValue::Scalar(match value.cast_to(&DataType::Utf8)? {
                    ScalarValue::Utf8(v) => ScalarValue::Utf8(v),
                    ScalarValue::Utf8View(v) => ScalarValue::Utf8(v.map(|s| s.to_string())),
                    other => {
                        let array = compute::cast(&other.to_array()?, &DataType::Utf8)?;
                        let strings = as_string_array(&array);
                        ScalarValue::Utf8(
                            (!strings.is_null(0)).then(|| strings.value(0).to_string()),
                        )
                    }
                })
            }
            ColumnarValue::Array(array) => {
                ColumnarValue::Array(Arc::new(compute::cast(&array, &DataType::Utf8)?))
            }
        };

        args.args = vec![coerced_arg];
        self.inner.invoke_with_args(args)
    }

    fn aliases(&self) -> &[String] {
        &self.aliases
    }
}

crate::macros::make_udf_function!(Md5Func);
