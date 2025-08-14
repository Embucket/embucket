use std::any::Any;
use std::sync::Arc;

use crate::macros::make_udf_function;
use datafusion::arrow::array::{ArrayRef, Float64Builder};
use datafusion::arrow::datatypes::DataType;
use datafusion::common::{Result as DataFusionResult, ScalarValue};
use datafusion::logical_expr::{
    ColumnarValue, ScalarFunctionArgs, ScalarUDFImpl, Signature, Volatility,
};
use datafusion_functions_json::common::{
    GetError, InvokeResult, JsonPath, invoke, jiter_json_find, return_type_check,
};
use jiter::{NumberAny, Peek};

#[derive(Debug)]
pub struct JsonGetFloat {
    signature: Signature,
}

impl Default for JsonGetFloat {
    fn default() -> Self {
        Self {
            signature: Signature::variadic_any(Volatility::Immutable),
        }
    }
}

impl ScalarUDFImpl for JsonGetFloat {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &'static str {
        "json_get_float"
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, arg_types: &[DataType]) -> DataFusionResult<DataType> {
        return_type_check(arg_types, self.name(), DataType::Float64)
    }

    fn invoke_with_args(&self, args: ScalarFunctionArgs) -> DataFusionResult<ColumnarValue> {
        invoke::<Self>(&args.args, jiter_json_get_float)
    }
}

impl InvokeResult for JsonGetFloat {
    type Item = f64;

    type Builder = Float64Builder;

    // Cheaper to produce a float array rather than dict-encoded floats
    const ACCEPT_DICT_RETURN: bool = false;

    fn builder(capacity: usize) -> Self::Builder {
        Float64Builder::with_capacity(capacity)
    }

    fn append_value(builder: &mut Self::Builder, value: Option<Self::Item>) {
        builder.append_option(value);
    }

    fn finish(mut builder: Self::Builder) -> DataFusionResult<ArrayRef> {
        Ok(Arc::new(builder.finish()))
    }

    fn scalar(value: Option<Self::Item>) -> ScalarValue {
        ScalarValue::Float64(value)
    }
}

fn jiter_json_get_float(json_data: Option<&str>, path: &[JsonPath]) -> Result<f64, GetError> {
    if let Some((mut jiter, peek)) = jiter_json_find(json_data, path) {
        match peek {
            // numbers are represented by everything else in peek, hence doing it this way
            Peek::Null
            | Peek::True
            | Peek::False
            | Peek::Minus
            | Peek::Infinity
            | Peek::NaN
            | Peek::String
            | Peek::Array
            | Peek::Object => Err(GetError),
            _ => match jiter.known_number(peek)? {
                NumberAny::Float(f) => Ok(f),
                NumberAny::Int(int) => Ok(int.into()),
            },
        }
    } else {
        Err(GetError)
    }
}

make_udf_function!(JsonGetFloat);
