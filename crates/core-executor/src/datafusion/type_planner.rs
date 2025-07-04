use arrow_schema::DECIMAL128_MAX_PRECISION;
use datafusion::arrow::datatypes::{DataType, TimeUnit};
use datafusion::common::Result;
use datafusion::logical_expr::planner::TypePlanner;
use datafusion::logical_expr::sqlparser::ast;
use datafusion::sql::sqlparser::ast::DataType as SQLDataType;
use datafusion::sql::utils::make_decimal_type;
use datafusion_common::{DataFusionError, not_impl_err};

#[derive(Debug)]
pub struct CustomTypePlanner {}

impl TypePlanner for CustomTypePlanner {
    fn plan_type(&self, sql_type: &ast::DataType) -> Result<Option<DataType>> {
        match sql_type {
            SQLDataType::Int32 => Ok(Some(DataType::Int32)),
            SQLDataType::Int64 => Ok(Some(DataType::Int64)),
            SQLDataType::UInt32 => Ok(Some(DataType::UInt32)),
            SQLDataType::Float(_) | SQLDataType::Float32 => Ok(Some(DataType::Float32)),
            SQLDataType::Float64 => Ok(Some(DataType::Float64)),
            SQLDataType::Blob(_) | SQLDataType::Binary(_) | SQLDataType::Varbinary(_) => {
                Ok(Some(DataType::Binary))
            }
            // https://github.com/apache/datafusion/issues/12644
            SQLDataType::JSON => Ok(Some(DataType::Utf8)),
            SQLDataType::Datetime(precision) => {
                let time_unit = parse_timestamp_precision(*precision)?;
                Ok(Some(DataType::Timestamp(time_unit, None)))
            }
            SQLDataType::Custom(a, b) => match a.to_string().to_uppercase().as_str() {
                "VARIANT" => Ok(Some(DataType::Utf8)),
                "TIMESTAMP_NTZ" | "TIMESTAMP_LTZ" | "TIMESTAMP_TZ" => {
                    let parsed_b: Option<u64> = b.iter().next().and_then(|s| s.parse().ok());
                    let time_unit = parse_timestamp_precision(parsed_b)?;
                    Ok(Some(DataType::Timestamp(time_unit, None)))
                }
                "NUMBER" => {
                    let (precision, scale) = match b.len() {
                        0 => (Some(u64::from(DECIMAL128_MAX_PRECISION)), None),
                        1 => {
                            let precision = b[0].parse().map_err(|_| {
                                DataFusionError::Plan(format!("Invalid precision: {}", b[0]))
                            })?;
                            (Some(precision), None)
                        }
                        2 => {
                            let precision = b[0].parse().map_err(|_| {
                                DataFusionError::Plan(format!("Invalid precision: {}", b[0]))
                            })?;
                            let scale = b[1].parse().map_err(|_| {
                                DataFusionError::Plan(format!("Invalid scale: {}", b[1]))
                            })?;
                            (Some(precision), Some(scale))
                        }
                        _ => {
                            return Err(DataFusionError::Plan(format!(
                                "Invalid NUMBER type format: {b:?}"
                            )));
                        }
                    };
                    make_decimal_type(precision, scale).map(Some)
                }
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }
}

fn parse_timestamp_precision(precision: Option<u64>) -> Result<TimeUnit> {
    match precision {
        Some(0) => Ok(TimeUnit::Second),
        Some(3) => Ok(TimeUnit::Millisecond),
        // We coerce nanoseconds to microseconds as Apache Iceberg v2 doesn't support nanosecond precision
        None | Some(6 | 9) => Ok(TimeUnit::Microsecond),
        _ => not_impl_err!("Unsupported SQL precision {precision:?}"),
    }
}
