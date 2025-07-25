use chrono::{DateTime, NaiveDateTime};
use datafusion::arrow::array::{
    Array, Decimal128Array, Int32Array, Int64Array, StringArray, StringViewArray,
    TimestampMillisecondBuilder, TimestampNanosecondBuilder, UInt32Array, UInt64Array,
    new_null_array,
};
use datafusion::arrow::compute::kernels;
use datafusion::arrow::compute::kernels::cast_utils::string_to_timestamp_nanos;
use datafusion::arrow::datatypes::{DataType, TimeUnit};
use datafusion::error::Result as DFResult;
use datafusion::logical_expr::ColumnarValue;
use datafusion_common::arrow::array::{
    ArrayRef, TimestampMicrosecondBuilder, TimestampSecondBuilder,
};
use datafusion_common::format::DEFAULT_CAST_OPTIONS;
use datafusion_common::{ScalarValue, internal_err};

use crate::conversion_errors::{
    ArgumentTwoNeedsToBeIntegerSnafu, CantAddLocalTimezoneSnafu, CantCastToSnafu,
    CantGetTimestampSnafu, CantParseTimestampSnafu, CantParseTimezoneSnafu, InvalidDataTypeSnafu,
    InvalidValueForFunctionAtPositionTwoSnafu,
};
use chrono_tz::Tz;
use datafusion_expr::{
    ReturnInfo, ReturnTypeArgs, ScalarFunctionArgs, ScalarUDFImpl, Signature, Volatility,
};
use regex::Regex;
use std::any::Any;
use std::sync::{Arc, LazyLock};

#[allow(clippy::unwrap_used)]
static RE_TIMEZONE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(Z|[+-]\d{2}:?\d{2}|\b(?:CEST|CET|EEST|EET|EST|EDT|PST|PDT|MST|MDT|CST|CDT|GMT|UTC|MSD|JST)\b)$"
    ).unwrap()
});

macro_rules! build_from_int_scale {
    ($tz:expr,$args:expr,$arr:expr, $type:ty) => {{
        let scale = if $args.len() == 1 {
            0
        } else {
            if let ColumnarValue::Scalar(v) = &$args[1] {
                let scale = v.cast_to(&DataType::Int64)?;
                if let ScalarValue::Int64(Some(v)) = &scale {
                    *v
                } else {
                    return ArgumentTwoNeedsToBeIntegerSnafu.fail()?;
                }
            } else {
                0
            }
        };

        let arr = $arr
            .as_any()
            .downcast_ref::<$type>()
            .ok_or_else(|| CantCastToSnafu { v: "integer" }.build())?;
        let arr: ArrayRef = match scale {
            0 => {
                let mut b = TimestampSecondBuilder::with_capacity(arr.len()).with_timezone_opt($tz);
                for v in arr {
                    match v {
                        None => b.append_null(),
                        Some(v) => b.append_value(v as i64),
                    }
                }
                Arc::new(b.finish())
            }
            3 => {
                let mut b =
                    TimestampMillisecondBuilder::with_capacity(arr.len()).with_timezone_opt($tz);
                for v in arr {
                    match v {
                        None => b.append_null(),
                        Some(v) => b.append_value(v as i64),
                    }
                }
                Arc::new(b.finish())
            }
            6 => {
                let mut b =
                    TimestampMicrosecondBuilder::with_capacity(arr.len()).with_timezone_opt($tz);
                for v in arr {
                    match v {
                        None => b.append_null(),
                        Some(v) => b.append_value(v as i64),
                    }
                }
                Arc::new(b.finish())
            }
            9 => {
                let mut b =
                    TimestampNanosecondBuilder::with_capacity(arr.len()).with_timezone_opt($tz);
                for v in arr {
                    match v {
                        None => b.append_null(),
                        Some(v) => b.append_value(v as i64),
                    }
                }
                Arc::new(b.finish())
            }
            _ => return InvalidValueForFunctionAtPositionTwoSnafu.fail()?,
        };

        if arr.len() == 1 {
            ColumnarValue::Scalar(ScalarValue::try_from_array(&arr, 0)?)
        } else {
            ColumnarValue::Array(Arc::new(arr))
        }
    }};
}

macro_rules! build_from_int_string {
    ($format:expr,$tz:expr,$args:expr,$arr:expr, $type:ty,$try:expr) => {{
        let format = if $args.len() == 1 {
            convert_snowflake_format_to_chrono($format)
        } else {
            if let ColumnarValue::Scalar(v) = &$args[1] {
                let format = v.cast_to(&DataType::Utf8)?;
                let ScalarValue::Utf8(Some(v)) = &format else {
                    return ArgumentTwoNeedsToBeIntegerSnafu.fail()?;
                };

                convert_snowflake_format_to_chrono(v)
            } else {
                convert_snowflake_format_to_chrono("YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM")
            }
        };

        let arr = $arr
            .as_any()
            .downcast_ref::<$type>()
            .ok_or_else(|| CantCastToSnafu { v: "string" }.build())?;

        let mut b = TimestampNanosecondBuilder::with_capacity(arr.len()).with_timezone_opt($tz);
        for v in arr {
            match v {
                None => b.append_null(),
                Some(s) => {
                    if contains_only_digits(s) {
                        let i = s
                            .parse::<i64>()
                            .map_err(|_| CantParseTimestampSnafu.build())?;
                        let scale = determine_timestamp_scale(i);
                        if scale == 0 {
                            b.append_value(i * 1000_000_000);
                        } else if scale == 3 {
                            b.append_value(i * 1000_000);
                        } else if scale == 6 {
                            b.append_value(i * 1000);
                        } else if scale == 9 {
                            b.append_value(i);
                        }
                    } else {
                        let s = remove_timezone(s);
                        let t = match NaiveDateTime::parse_from_str(s.as_str(), &format) {
                            Ok(v) => match v.and_utc().timestamp_nanos_opt() {
                                Some(v) => v,
                                None => {
                                    if $try {
                                        b.append_null();
                                        continue;
                                    }

                                    return CantGetTimestampSnafu.fail()?;
                                }
                            },
                            Err(_) => match string_to_timestamp_nanos(s.as_str()) {
                                Ok(v) => v,
                                Err(_) => {
                                    if $try {
                                        b.append_null();
                                        continue;
                                    }

                                    return CantGetTimestampSnafu.fail()?;
                                }
                            },
                        };

                        let t = if let Some(tz) = $tz {
                            let tz: Tz = tz.parse().map_err(|_| CantParseTimezoneSnafu.build())?;
                            let t = DateTime::from_timestamp_nanos(t);
                            let Some(t) = t.naive_utc().and_local_timezone(tz).single() else {
                                if $try {
                                    b.append_null();
                                    continue;
                                }

                                return CantAddLocalTimezoneSnafu.fail()?;
                            };

                            let Some(t) = t.naive_utc().and_utc().timestamp_nanos_opt() else {
                                if $try {
                                    b.append_null();
                                    continue;
                                }

                                return CantGetTimestampSnafu.fail()?;
                            };

                            t
                        } else {
                            t
                        };
                        b.append_value(t);
                    }
                }
            }
        }

        let arr = Arc::new(b.finish()) as ArrayRef;
        if arr.len() == 1 {
            ColumnarValue::Scalar(ScalarValue::try_from_array(&arr, 0)?)
        } else {
            ColumnarValue::Array(Arc::new(arr))
        }
    }};
}

#[derive(Debug)]
pub struct ToTimestampFunc {
    signature: Signature,
    timezone: Option<Arc<str>>,
    format: String,
    name: String,
    r#try: bool,
}

impl Default for ToTimestampFunc {
    fn default() -> Self {
        Self::new(
            None,
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timstamp".to_string(),
        )
    }
}

impl ToTimestampFunc {
    #[must_use]
    pub fn new(timezone: Option<Arc<str>>, format: String, r#try: bool, name: String) -> Self {
        Self {
            signature: Signature::variadic_any(Volatility::Immutable),
            timezone,
            format,
            name,
            r#try,
        }
    }
}

impl ScalarUDFImpl for ToTimestampFunc {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> DFResult<DataType> {
        internal_err!("return_type_from_args should be called")
    }

    #[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
    fn return_type_from_args(&self, args: ReturnTypeArgs) -> DFResult<ReturnInfo> {
        if args.scalar_arguments.len() == 1 {
            if args.arg_types[0].is_numeric() {
                return Ok(ReturnInfo::new_nullable(DataType::Timestamp(
                    TimeUnit::Second,
                    self.timezone.clone(),
                )));
            }
        } else if args.scalar_arguments.len() == 2 {
            if args.arg_types[0].is_numeric() {
                if let Some(v) = args.scalar_arguments[1] {
                    let scale = v.cast_to(&DataType::Int64)?;
                    let ScalarValue::Int64(Some(s)) = &scale else {
                        return ArgumentTwoNeedsToBeIntegerSnafu.fail()?;
                    };
                    let s = *s;
                    return match s {
                        0 => Ok(ReturnInfo::new_nullable(DataType::Timestamp(
                            TimeUnit::Second,
                            self.timezone.clone(),
                        ))),
                        3 => Ok(ReturnInfo::new_nullable(DataType::Timestamp(
                            TimeUnit::Millisecond,
                            self.timezone.clone(),
                        ))),
                        6 => Ok(ReturnInfo::new_nullable(DataType::Timestamp(
                            TimeUnit::Microsecond,
                            self.timezone.clone(),
                        ))),
                        9 => Ok(ReturnInfo::new_nullable(DataType::Timestamp(
                            TimeUnit::Nanosecond,
                            self.timezone.clone(),
                        ))),
                        _ => return InvalidValueForFunctionAtPositionTwoSnafu.fail()?,
                    };
                }
            } else if let Some(ScalarValue::TimestampSecond(_, Some(tz))) = args.scalar_arguments[0]
            {
                return Ok(ReturnInfo::new_nullable(DataType::Timestamp(
                    TimeUnit::Second,
                    Some(tz.to_owned()),
                )));
            } else if let Some(ScalarValue::TimestampMillisecond(_, Some(tz))) =
                args.scalar_arguments[0]
            {
                return Ok(ReturnInfo::new_nullable(DataType::Timestamp(
                    TimeUnit::Millisecond,
                    Some(tz.to_owned()),
                )));
            } else if let Some(ScalarValue::TimestampMicrosecond(_, Some(tz))) =
                args.scalar_arguments[0]
            {
                return Ok(ReturnInfo::new_nullable(DataType::Timestamp(
                    TimeUnit::Microsecond,
                    Some(tz.to_owned()),
                )));
            } else if let Some(ScalarValue::TimestampNanosecond(_, Some(tz))) =
                args.scalar_arguments[0]
            {
                return Ok(ReturnInfo::new_nullable(DataType::Timestamp(
                    TimeUnit::Nanosecond,
                    Some(tz.to_owned()),
                )));
            }
        }

        Ok(ReturnInfo::new_nullable(DataType::Timestamp(
            TimeUnit::Nanosecond,
            self.timezone.clone(),
        )))
    }
    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        clippy::cast_possible_wrap,
        clippy::as_conversions,
        clippy::cast_lossless,
        clippy::cast_possible_truncation
    )]
    fn invoke_with_args(&self, args: ScalarFunctionArgs) -> DFResult<ColumnarValue> {
        let ScalarFunctionArgs { args, .. } = args;

        let arr = match args[0].clone() {
            ColumnarValue::Array(arr) => arr,
            ColumnarValue::Scalar(v) => v.to_array()?,
        };

        Ok(match arr.data_type() {
            DataType::Int64 => {
                build_from_int_scale!(self.timezone.clone(), args, arr, Int64Array)
            }
            DataType::UInt64 => {
                build_from_int_scale!(self.timezone.clone(), args, arr, UInt64Array)
            }
            DataType::Int32 => {
                build_from_int_scale!(self.timezone.clone(), args, arr, Int32Array)
            }
            DataType::UInt32 => {
                build_from_int_scale!(self.timezone.clone(), args, arr, UInt32Array)
            }
            DataType::Decimal128(_, s) => parse_decimal(&arr, &args, self.timezone.clone(), *s)?,
            DataType::Utf8 => {
                build_from_int_string!(
                    &self.format,
                    self.timezone.clone(),
                    args,
                    arr,
                    StringArray,
                    self.r#try
                )
            }
            DataType::Utf8View => {
                build_from_int_string!(
                    &self.format,
                    self.timezone.clone(),
                    args,
                    arr,
                    StringViewArray,
                    self.r#try
                )
            }
            DataType::Timestamp(_, tz) => {
                let tz = if let Some(tz) = tz {
                    Some(tz.clone())
                } else {
                    self.timezone.clone()
                };

                let arr = kernels::cast::cast_with_options(
                    &arr,
                    &DataType::Timestamp(TimeUnit::Nanosecond, tz),
                    &DEFAULT_CAST_OPTIONS,
                )?;

                if arr.len() == 1 {
                    ColumnarValue::Scalar(ScalarValue::try_from_array(&arr, 0)?)
                } else {
                    ColumnarValue::Array(Arc::new(arr))
                }
            }
            DataType::Date32 | DataType::Date64 => {
                let arr = kernels::cast::cast_with_options(
                    &arr,
                    &DataType::Timestamp(TimeUnit::Nanosecond, self.timezone.clone()),
                    &DEFAULT_CAST_OPTIONS,
                )?;

                if arr.len() == 1 {
                    ColumnarValue::Scalar(ScalarValue::try_from_array(&arr, 0)?)
                } else {
                    ColumnarValue::Array(Arc::new(arr))
                }
            }
            DataType::Null => {
                let null_arr = new_null_array(
                    &DataType::Timestamp(TimeUnit::Nanosecond, self.timezone.clone()),
                    arr.len(),
                );

                if arr.len() == 1 {
                    ColumnarValue::Scalar(ScalarValue::try_from_array(&null_arr, 0)?)
                } else {
                    ColumnarValue::Array(null_arr)
                }
            }
            _ => InvalidDataTypeSnafu.fail()?,
        })
    }
}

#[allow(
    clippy::cast_possible_wrap,
    clippy::as_conversions,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::needless_pass_by_value
)]
fn parse_decimal(
    arr: &ArrayRef,
    args: &[ColumnarValue],
    timezone: Option<Arc<str>>,
    s: i8,
) -> DFResult<ColumnarValue> {
    let s = i128::from(s).pow(10);
    let scale = if args.len() == 1 {
        0
    } else if let ColumnarValue::Scalar(v) = &args[1] {
        let scale = v.cast_to(&DataType::Int64)?;
        let ScalarValue::Int64(Some(v)) = &scale else {
            return ArgumentTwoNeedsToBeIntegerSnafu.fail()?;
        };

        *v
    } else {
        0
    };

    let arr = arr
        .as_any()
        .downcast_ref::<Decimal128Array>()
        .ok_or_else(|| CantCastToSnafu { v: "decimal128" }.build())?;
    let arr: ArrayRef = match scale {
        0 => {
            let mut b =
                TimestampSecondBuilder::with_capacity(arr.len()).with_timezone_opt(timezone);
            for v in arr {
                match v {
                    None => b.append_null(),
                    Some(v) => b.append_value((v / s) as i64),
                }
            }
            Arc::new(b.finish())
        }
        3 => {
            let mut b =
                TimestampMillisecondBuilder::with_capacity(arr.len()).with_timezone_opt(timezone);
            for v in arr {
                match v {
                    None => b.append_null(),
                    Some(v) => b.append_value((v / s) as i64),
                }
            }
            Arc::new(b.finish())
        }
        6 => {
            let mut b =
                TimestampMicrosecondBuilder::with_capacity(arr.len()).with_timezone_opt(timezone);
            for v in arr {
                match v {
                    None => b.append_null(),
                    Some(v) => b.append_value((v / s) as i64),
                }
            }
            Arc::new(b.finish())
        }
        9 => {
            let mut b =
                TimestampNanosecondBuilder::with_capacity(arr.len()).with_timezone_opt(timezone);
            for v in arr {
                match v {
                    None => b.append_null(),
                    Some(v) => b.append_value((v / s) as i64),
                }
            }
            Arc::new(b.finish())
        }
        _ => return InvalidValueForFunctionAtPositionTwoSnafu.fail()?,
    };
    if arr.len() == 1 {
        Ok(ColumnarValue::Scalar(ScalarValue::try_from_array(&arr, 0)?))
    } else {
        Ok(ColumnarValue::Array(Arc::new(arr)))
    }
}
fn remove_timezone(datetime_str: &str) -> String {
    let s = datetime_str.trim();

    if let Some(caps) = RE_TIMEZONE.find(s) {
        return s[0..caps.start()].to_string();
    }

    s.to_string()
}

fn contains_only_digits(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_digit())
}

#[must_use]
pub const fn determine_timestamp_scale(value: i64) -> u8 {
    const MILLIS_PER_YEAR: i64 = 31_536_000_000;
    const MICROS_PER_YEAR: i64 = 31_536_000_000_000;
    const NANOS_PER_YEAR: i64 = 31_536_000_000_000_000;

    let abs_value = value.abs();

    if abs_value < MILLIS_PER_YEAR {
        0
    } else if abs_value < MICROS_PER_YEAR {
        3
    } else if abs_value < NANOS_PER_YEAR {
        6
    } else {
        9
    }
}

#[must_use]
pub fn convert_snowflake_format_to_chrono(snowflake_format: &str) -> String {
    let mut chrono_format = snowflake_format.to_string().to_lowercase();

    chrono_format = chrono_format.replace("yyyy", "%Y");
    chrono_format = chrono_format.replace("yy", "%y");

    chrono_format = chrono_format.replace("mm", "%m");
    chrono_format = chrono_format.replace("mon", "%b");
    chrono_format = chrono_format.replace("month", "%B");

    chrono_format = chrono_format.replace("dd", "%d");
    chrono_format = chrono_format.replace("dy", "%a");
    chrono_format = chrono_format.replace("day", "%A");

    chrono_format = chrono_format.replace("hh24", "%H");
    chrono_format = chrono_format.replace("hh", "%I");
    chrono_format = chrono_format.replace("am", "%P");
    chrono_format = chrono_format.replace("pm", "%P");

    chrono_format = chrono_format.replace("mi", "%M");

    chrono_format = chrono_format.replace("ss", "%S");

    chrono_format = chrono_format.replace(".ff9", "%.9f");
    chrono_format = chrono_format.replace(".ff6", "%.6f");
    chrono_format = chrono_format.replace(".ff3", "%.3f");
    chrono_format = chrono_format.replace(".ff", "%.f");

    chrono_format = chrono_format.replace("tzh:tzm", "%z");
    chrono_format = chrono_format.replace("tzhtzm", "%Z");

    chrono_format
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::visitors::timestamp;
    use datafusion::prelude::SessionContext;
    use datafusion::sql::parser::Statement;
    use datafusion_common::assert_batches_eq;
    use datafusion_expr::ScalarUDF;

    #[tokio::test]
    async fn test_scale() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            None,
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp".to_string(),
        )));

        let sql = r#"SELECT
       TO_TIMESTAMP(1000000000, 0) AS "Scale in seconds",
       TO_TIMESTAMP(1000000000, 3) AS "Scale in milliseconds",
       TO_TIMESTAMP(1000000000, 6) AS "Scale in microseconds",
       TO_TIMESTAMP(1000000000, 9) AS "Scale in nanoseconds";"#;
        let result = ctx.sql(sql).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+---------------------+-----------------------+-----------------------+----------------------+",
                "| Scale in seconds    | Scale in milliseconds | Scale in microseconds | Scale in nanoseconds |",
                "+---------------------+-----------------------+-----------------------+----------------------+",
                "| 2001-09-09T01:46:40 | 1970-01-12T13:46:40   | 1970-01-01T00:16:40   | 1970-01-01T00:00:01  |",
                "+---------------------+-----------------------+-----------------------+----------------------+",
            ],
            &result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_scaled() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            None,
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp".to_string(),
        )));

        let sql = r#"SELECT
       TO_TIMESTAMP(1000000000) AS "Scale in seconds",
       TO_TIMESTAMP(1000000000000, 3) AS "Scale in milliseconds",
       TO_TIMESTAMP(1000000000000000, 6) AS "Scale in microseconds",
       TO_TIMESTAMP(1000000000000000000, 9) AS "Scale in nanoseconds";"#;
        let result = ctx.sql(sql).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+---------------------+-----------------------+-----------------------+----------------------+",
                "| Scale in seconds    | Scale in milliseconds | Scale in microseconds | Scale in nanoseconds |",
                "+---------------------+-----------------------+-----------------------+----------------------+",
                "| 2001-09-09T01:46:40 | 2001-09-09T01:46:40   | 2001-09-09T01:46:40   | 2001-09-09T01:46:40  |",
                "+---------------------+-----------------------+-----------------------+----------------------+",
            ],
            &result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_scaled_tz() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            Some(Arc::from("America/Los_Angeles")),
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp".to_string(),
        )));

        let sql = r#"SELECT
       TO_TIMESTAMP(1000000000) AS "Scale in seconds",
       TO_TIMESTAMP(1000000000000, 3) AS "Scale in milliseconds",
       TO_TIMESTAMP(1000000000000000, 6) AS "Scale in microseconds",
       TO_TIMESTAMP(1000000000000000000, 9) AS "Scale in nanoseconds";"#;
        let result = ctx.sql(sql).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+---------------------------+---------------------------+---------------------------+---------------------------+",
                "| Scale in seconds          | Scale in milliseconds     | Scale in microseconds     | Scale in nanoseconds      |",
                "+---------------------------+---------------------------+---------------------------+---------------------------+",
                "| 2001-09-08T18:46:40-07:00 | 2001-09-08T18:46:40-07:00 | 2001-09-08T18:46:40-07:00 | 2001-09-08T18:46:40-07:00 |",
                "+---------------------------+---------------------------+---------------------------+---------------------------+",
            ],
            &result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_scale_decimal() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            None,
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp".to_string(),
        )));

        let sql = r#"SELECT
       TO_TIMESTAMP(1000000000::DECIMAL, 0) AS "Scale in seconds",
       TO_TIMESTAMP(1000000000::DECIMAL, 3) AS "Scale in milliseconds",
       TO_TIMESTAMP(1000000000::DECIMAL, 6) AS "Scale in microseconds",
       TO_TIMESTAMP(1000000000::DECIMAL, 9) AS "Scale in nanoseconds";"#;
        let result = ctx.sql(sql).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+---------------------+-----------------------+-----------------------+----------------------+",
                "| Scale in seconds    | Scale in milliseconds | Scale in microseconds | Scale in nanoseconds |",
                "+---------------------+-----------------------+-----------------------+----------------------+",
                "| 2001-09-09T01:46:40 | 1970-01-12T13:46:40   | 1970-01-01T00:16:40   | 1970-01-01T00:00:01  |",
                "+---------------------+-----------------------+-----------------------+----------------------+",
            ],
            &result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_scale_decimal_scaled() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            None,
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp".to_string(),
        )));

        let sql = r#"SELECT
       TO_TIMESTAMP(1000000000::DECIMAL, 0) AS "Scale in seconds",
       TO_TIMESTAMP(1000000000000::DECIMAL, 3) AS "Scale in milliseconds",
       TO_TIMESTAMP(1000000000000000::DECIMAL, 6) AS "Scale in microseconds",
       TO_TIMESTAMP(1000000000000000000::DECIMAL, 9) AS "Scale in nanoseconds";"#;
        let result = ctx.sql(sql).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+---------------------+-----------------------+-----------------------+----------------------+",
                "| Scale in seconds    | Scale in milliseconds | Scale in microseconds | Scale in nanoseconds |",
                "+---------------------+-----------------------+-----------------------+----------------------+",
                "| 2001-09-09T01:46:40 | 2001-09-09T01:46:40   | 2001-09-09T01:46:40   | 2001-09-09T01:46:40  |",
                "+---------------------+-----------------------+-----------------------+----------------------+",
            ],
            &result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_scale_int_str() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            None,
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp".to_string(),
        )));

        let sql = r#"SELECT
       TO_TIMESTAMP('1000000000') AS "Scale in seconds",
       TO_TIMESTAMP('1000000000000') AS "Scale in milliseconds",
       TO_TIMESTAMP('1000000000000000') AS "Scale in microseconds",
       TO_TIMESTAMP('1000000000000000000') AS "Scale in nanoseconds";"#;
        let result = ctx.sql(sql).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+---------------------+-----------------------+-----------------------+----------------------+",
                "| Scale in seconds    | Scale in milliseconds | Scale in microseconds | Scale in nanoseconds |",
                "+---------------------+-----------------------+-----------------------+----------------------+",
                "| 2001-09-09T01:46:40 | 2001-09-09T01:46:40   | 2001-09-09T01:46:40   | 2001-09-09T01:46:40  |",
                "+---------------------+-----------------------+-----------------------+----------------------+",
            ],
            &result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_different_formats() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            None,
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp".to_string(),
        )));

        let sql = "SELECT to_timestamp('2021-03-02 15:55:18.539000') as a, to_timestamp('2020-09-08T13:42:29.190855+00:00') as b";
        let result = ctx.sql(sql).await?.collect().await?;
        assert_batches_eq!(
            &[
                "+-------------------------+----------------------------+",
                "| a                       | b                          |",
                "+-------------------------+----------------------------+",
                "| 2021-03-02T15:55:18.539 | 2020-09-08T13:42:29.190855 |",
                "+-------------------------+----------------------------+",
            ],
            &result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_drop_timezone() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            None,
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp".to_string(),
        )));

        let sql = "SELECT to_timestamp('2020-09-08T13:42:29.190855+01:00') as a, to_timestamp('1970-01-01T00:00:00Z') as b";
        let result = ctx.sql(sql).await?.collect().await?;
        assert_batches_eq!(
            &[
                "+----------------------------+---------------------+",
                "| a                          | b                   |",
                "+----------------------------+---------------------+",
                "| 2020-09-08T13:42:29.190855 | 1970-01-01T00:00:00 |",
                "+----------------------------+---------------------+",
            ],
            &result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_timezone_str() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            Some(Arc::from("America/Los_Angeles")),
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp_tz".to_string(),
        )));

        let sql = "SELECT to_timestamp_tz('2020-09-08T13:42:29.190855+01:00') as a, to_timestamp_tz('2024-04-05 01:02:03') as b";
        let result = ctx.sql(sql).await?.collect().await?;
        assert_batches_eq!(
            &[
                "+----------------------------------+---------------------------+",
                "| a                                | b                         |",
                "+----------------------------------+---------------------------+",
                "| 2020-09-08T13:42:29.190855-07:00 | 2024-04-05T01:02:03-07:00 |",
                "+----------------------------------+---------------------------+",
            ],
            &result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_str_format() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            None,
            "mm/dd/yyyy hh24:mi:ss".to_string(),
            false,
            "to_timestamp".to_string(),
        )));

        let sql = r#"SELECT
       TO_TIMESTAMP('04/05/2024 01:02:03', 'mm/dd/yyyy hh24:mi:ss') as "a",
       TO_TIMESTAMP('04/05/2024 01:02:03') as "b"
       "#;
        let result = ctx.sql(sql).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+---------------------+---------------------+",
                "| a                   | b                   |",
                "+---------------------+---------------------+",
                "| 2024-04-05T01:02:03 | 2024-04-05T01:02:03 |",
                "+---------------------+---------------------+",
            ],
            &result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_null() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            None,
            "mm/dd/yyyy hh24:mi:ss".to_string(),
            false,
            "to_timestamp".to_string(),
        )));

        let sql = "SELECT TO_TIMESTAMP(NULL) as a";
        let result = ctx.sql(sql).await?.collect().await?;
        assert_batches_eq!(&["+---+", "| a |", "+---+", "|   |", "+---+",], &result);
        Ok(())
    }

    #[tokio::test]
    async fn test_timestamp() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            None,
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp".to_string(),
        )));

        let sql = r#"SELECT
       TO_TIMESTAMP(1000000000::TIMESTAMP) as "a""#;
        let result = ctx.sql(sql).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+---------------------+",
                "| a                   |",
                "+---------------------+",
                "| 2001-09-09T01:46:40 |",
                "+---------------------+",
            ],
            &result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_date() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            None,
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp".to_string(),
        )));

        let sql = r#"SELECT
       TO_TIMESTAMP('2022-01-01 11:30:00'::date) as "a""#;
        let result = ctx.sql(sql).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+---------------------+",
                "| a                   |",
                "+---------------------+",
                "| 2022-01-01T00:00:00 |",
                "+---------------------+",
            ],
            &result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_timezone() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            Some(Arc::from("America/Los_Angeles")),
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp".to_string(),
        )));

        let sql = r#"SELECT
       TO_TIMESTAMP(1000000000) as "a""#;
        let result = ctx.sql(sql).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+---------------------------+",
                "| a                         |",
                "+---------------------------+",
                "| 2001-09-08T18:46:40-07:00 |",
                "+---------------------------+",
            ],
            &result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_different_names() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            None,
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp".to_string(),
        )));
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            None,
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp_ntz".to_string(),
        )));
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            Some(Arc::from("America/Los_Angeles")),
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp_tz".to_string(),
        )));

        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            Some(Arc::from("America/Los_Angeles")),
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp_ltz".to_string(),
        )));

        let sql = r#"SELECT
       TO_TIMESTAMP(1000000000) as "a",
       TO_TIMESTAMP_NTZ(1000000000) as "b",
       TO_TIMESTAMP_TZ(1000000000) as "c",
       TO_TIMESTAMP_LTZ(1000000000) as "d"
       "#;
        let result = ctx.sql(sql).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+---------------------+---------------------+---------------------------+---------------------------+",
                "| a                   | b                   | c                         | d                         |",
                "+---------------------+---------------------+---------------------------+---------------------------+",
                "| 2001-09-09T01:46:40 | 2001-09-09T01:46:40 | 2001-09-08T18:46:40-07:00 | 2001-09-08T18:46:40-07:00 |",
                "+---------------------+---------------------+---------------------------+---------------------------+",
            ],
            &result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_try() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            None,
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            true,
            "try_to_timestamp".to_string(),
        )));

        let sql = "SELECT TRY_TO_TIMESTAMP('sdfsdf')";
        let result = ctx.sql(sql).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+----------------------------------+",
                "| try_to_timestamp(Utf8(\"sdfsdf\")) |",
                "+----------------------------------+",
                "|                                  |",
                "+----------------------------------+",
            ],
            &result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_visitor() -> DFResult<()> {
        let ctx = SessionContext::new();
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            None,
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp".to_string(),
        )));
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            None,
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp_ntz".to_string(),
        )));
        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            Some(Arc::from("America/Los_Angeles")),
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp_tz".to_string(),
        )));

        ctx.register_udf(ScalarUDF::from(ToTimestampFunc::new(
            Some(Arc::from("America/Los_Angeles")),
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM".to_string(),
            false,
            "to_timestamp_ltz".to_string(),
        )));

        let sql = "SELECT
        1000000000::TIMESTAMP as a,
        1000000000::TIMESTAMP_NTZ as b,
        1000000000::TIMESTAMP_TZ as c,
        1000000000::TIMESTAMP_LTZ as d,
         '2025-07-04 19:16:30+02:00'::TIMESTAMP_TZ as e";
        let mut statement = ctx.state().sql_to_statement(sql, "snowflake")?;
        if let Statement::Statement(ref mut stmt) = statement {
            timestamp::visit(stmt);
        }
        let plan = ctx.state().statement_to_plan(statement).await?;
        let result = ctx.execute_logical_plan(plan).await?.collect().await?;

        assert_batches_eq!(
            &[
                "+---------------------+---------------------+---------------------------+---------------------------+---------------------------+",
                "| a                   | b                   | c                         | d                         | e                         |",
                "+---------------------+---------------------+---------------------------+---------------------------+---------------------------+",
                "| 2001-09-09T01:46:40 | 2001-09-09T01:46:40 | 2001-09-08T18:46:40-07:00 | 2001-09-08T18:46:40-07:00 | 2025-07-04T19:16:30-07:00 |",
                "+---------------------+---------------------+---------------------------+---------------------------+---------------------------+",
            ],
            &result
        );
        Ok(())
    }
}
