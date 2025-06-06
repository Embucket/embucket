use datafusion::arrow::array::{ArrayRef, StringBuilder};
use datafusion::arrow::datatypes::DataType;
use datafusion::error::Result as DFResult;
use datafusion_common::cast::{as_int64_array, as_string_array};
use datafusion_common::{ScalarValue, exec_err};
use datafusion_expr::{
    ColumnarValue, ScalarFunctionArgs, ScalarUDFImpl, Signature, TypeSignature, Volatility,
};
use std::any::Any;
use std::collections::HashSet;
use std::sync::Arc;

// strtok SQL function
// Tokenizes a given string and returns the requested part.
// If the requested part does not exist, then NULL is returned. If any parameter
// is NULL, then NULL is returned.
// STRTOK(<string> [,<delimiter>] [,<partNr>])
// Arguments:
// Required: <string> Text to be tokenized.
// Optional:
// <delimiter> Text representing the set of delimiters to tokenize on. Each character
// in the delimiter string is a delimiter. If the delimiter is empty, and the <string>
// is empty, then the function returns NULL. If the delimiter is empty, and the <string>
// is non empty, then the whole string will be treated as one token. The default value
// of the delimiter is a single space character.
// <partNr> Requested token, which is 1-based (i.e. the first token is token number 1,
// not token number 0). If the token number is out of range, then NULL is returned.
// The default value is 1.
// Note: `strtok` returns
// The data type of the returned value is VARCHAR.
// Usage Notes:
// If the string starts or is terminated with the delimiter, the system considers empty
// space before or after the delimiter, respectively, as a valid token.
// Similar to Linux strtok(), STRTOK never returns an empty string as a token.
#[derive(Debug)]
pub struct StrtokFunc {
    signature: Signature,
}

impl Default for StrtokFunc {
    fn default() -> Self {
        Self::new()
    }
}

impl StrtokFunc {
    #[must_use]
    pub fn new() -> Self {
        Self {
            signature: Signature::one_of(
                vec![
                    TypeSignature::String(1),
                    TypeSignature::String(2),
                    TypeSignature::Exact(vec![DataType::Utf8, DataType::Utf8, DataType::Int64]),
                    TypeSignature::Exact(vec![
                        DataType::LargeUtf8,
                        DataType::LargeUtf8,
                        DataType::Int64,
                    ]),
                    TypeSignature::Exact(vec![
                        DataType::Utf8View,
                        DataType::Utf8View,
                        DataType::Int64,
                    ]),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl ScalarUDFImpl for StrtokFunc {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &'static str {
        "strtok"
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> DFResult<DataType> {
        Ok(DataType::Utf8)
    }

    #[allow(clippy::unwrap_used)]
    fn invoke_with_args(&self, args: ScalarFunctionArgs) -> DFResult<ColumnarValue> {
        let ScalarFunctionArgs { args, .. } = args;

        let (first_arg, second_arg, third_arg) = match args.len() {
            1 => (
                resolve_arg(&args[0])?,
                ScalarValue::Utf8(Some(" ".to_string())).to_array()?,
                ScalarValue::UInt64(Some(1)).to_array()?,
            ),
            2 => (
                resolve_arg(&args[0])?,
                resolve_arg(&args[1])?,
                ScalarValue::UInt64(Some(1)).to_array()?,
            ),
            _ => (
                resolve_arg(&args[0])?,
                resolve_arg(&args[1])?,
                resolve_arg(&args[2])?,
            ),
        };

        let strs = as_string_array(&first_arg)?;
        let delms = as_string_array(&second_arg)?;
        let part_nrs = as_int64_array(&third_arg)?;

        let zipped = crate::macros::izip!(strs.iter(), delms.iter(), part_nrs.iter());

        let mut res = StringBuilder::new();

        for (string, delimiter, part_nr) in zipped {
            if string.is_none() || delimiter.is_none() || part_nr.is_none() {
                res.append_null();
                continue;
            }

            let mut tokens = vec![];
            let string = string.unwrap();
            let delimiter = delimiter.unwrap();
            let part_nr: usize = part_nr.unwrap().try_into().unwrap();

            if part_nr < 1 {
                return exec_err!("partNr cannot be less than 1");
            }

            let delimiter_set: HashSet<char> = delimiter.chars().collect();
            let mut last_split_index: usize = 0;
            for (i, ch) in string.chars().enumerate() {
                if delimiter_set.contains(&ch) {
                    let value = &string[last_split_index..i];
                    if !value.is_empty() {
                        tokens.push(value);
                    }
                    last_split_index = i + ch.len_utf8();
                }
            }

            let tail = &string[last_split_index..];
            if !tail.is_empty() {
                tokens.push(tail);
            }

            if part_nr > tokens.len() {
                res.append_null();
            } else {
                res.append_value(tokens[part_nr - 1]);
            }
        }

        Ok(ColumnarValue::Array(Arc::new(res.finish())))
    }
}

fn resolve_arg(arg: &ColumnarValue) -> DFResult<ArrayRef> {
    match arg {
        ColumnarValue::Array(arr) => Ok(Arc::clone(arr)),
        ColumnarValue::Scalar(scalar) => scalar.to_array(),
    }
}

crate::macros::make_udf_function!(StrtokFunc);

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::prelude::SessionContext;
    use datafusion_common::{DataFusionError, assert_batches_eq};
    use datafusion_expr::ScalarUDF;

    #[tokio::test]
    async fn test_it_works() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(StrtokFunc::new()));

        let q = "SELECT STRTOK('a.b.c', '.', 1);";
        let result = ctx.sql(q).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+------------------------------------------+",
                "| strtok(Utf8(\"a.b.c\"),Utf8(\".\"),Int64(1)) |",
                "+------------------------------------------+",
                "| a                                        |",
                "+------------------------------------------+",
            ],
            &result
        );

        let q = "SELECT STRTOK('user@snowflake.com', '@.', 1);";
        let result = ctx.sql(q).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+--------------------------------------------------------+",
                "| strtok(Utf8(\"user@snowflake.com\"),Utf8(\"@.\"),Int64(1)) |",
                "+--------------------------------------------------------+",
                "| user                                                   |",
                "+--------------------------------------------------------+",
            ],
            &result
        );

        let q = "SELECT STRTOK('user@snowflake.com', '@.', 2);";
        let result = ctx.sql(q).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+--------------------------------------------------------+",
                "| strtok(Utf8(\"user@snowflake.com\"),Utf8(\"@.\"),Int64(2)) |",
                "+--------------------------------------------------------+",
                "| snowflake                                              |",
                "+--------------------------------------------------------+",
            ],
            &result
        );

        let q = "SELECT STRTOK('user@snowflake.com', '@.', 3);";
        let result = ctx.sql(q).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+--------------------------------------------------------+",
                "| strtok(Utf8(\"user@snowflake.com\"),Utf8(\"@.\"),Int64(3)) |",
                "+--------------------------------------------------------+",
                "| com                                                    |",
                "+--------------------------------------------------------+",
            ],
            &result
        );

        // Indexing past the last possible token returns NULL
        let q = "SELECT STRTOK('user@snowflake.com.', '@.', 4);";
        let result = ctx.sql(q).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+---------------------------------------------------------+",
                "| strtok(Utf8(\"user@snowflake.com.\"),Utf8(\"@.\"),Int64(4)) |",
                "+---------------------------------------------------------+",
                "|                                                         |",
                "+---------------------------------------------------------+",
            ],
            &result
        );

        // In this example, because the input string is empty, there are 0 elements, and therefore element #1
        // is past the end of the string, so the function returns NULL rather than an empty string
        let q = "SELECT STRTOK('', '', 1);";
        let result = ctx.sql(q).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+------------------------------------+",
                "| strtok(Utf8(\"\"),Utf8(\"\"),Int64(1)) |",
                "+------------------------------------+",
                "|                                    |",
                "+------------------------------------+",
            ],
            &result
        );

        // Empty delimeter
        let q = "SELECT STRTOK('a.b', '', 1);";
        let result = ctx.sql(q).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+---------------------------------------+",
                "| strtok(Utf8(\"a.b\"),Utf8(\"\"),Int64(1)) |",
                "+---------------------------------------+",
                "| a.b                                   |",
                "+---------------------------------------+",
            ],
            &result
        );

        let q = "SELECT STRTOK(NULL, '.', 1);";
        let result = ctx.sql(q).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+---------------------------------+",
                "| strtok(NULL,Utf8(\".\"),Int64(1)) |",
                "+---------------------------------+",
                "|                                 |",
                "+---------------------------------+",
            ],
            &result
        );

        let q = "SELECT STRTOK('a.b', NULL, 1);";
        let result = ctx.sql(q).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+-----------------------------------+",
                "| strtok(Utf8(\"a.b\"),NULL,Int64(1)) |",
                "+-----------------------------------+",
                "|                                   |",
                "+-----------------------------------+",
            ],
            &result
        );

        let q = "SELECT STRTOK('a.b', '.', NULL);";
        let result = ctx.sql(q).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+------------------------------------+",
                "| strtok(Utf8(\"a.b\"),Utf8(\".\"),NULL) |",
                "+------------------------------------+",
                "|                                    |",
                "+------------------------------------+",
            ],
            &result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_argument_fails() {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(StrtokFunc::new()));

        // Zero arguments
        let q = "SELECT STRTOK('a.b.c', '.', 0);";
        let result = ctx.sql(q).await;

        if let Ok(df) = result {
            let result = df.collect().await;

            match result {
                Err(e) => assert!(
                    matches!(e, DataFusionError::Execution(_)),
                    "Expected Execution error for partNr less than 1, got: {e}",
                ),
                Ok(_) => panic!("Expected error but partNr less than 1 succeeded"),
            }
        }
    }
}
