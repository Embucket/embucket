use arrow::datatypes::{DataType, IntervalUnit, TimeUnit};
use datafusion::{common::exec_err, logical_expr::{ColumnarValue, ScalarUDFImpl, Signature, TypeSignature, Volatility}, scalar::ScalarValue};

#[derive(Debug)]
pub struct DateAddFunc {
    signature: Signature,
}

impl Default for DateAddFunc {
    fn default() -> Self {
        DateAddFunc::new()
    }
}
impl DateAddFunc {
    pub fn new() -> Self {
        Self {
            signature: Signature::one_of(
                vec![
                    //Add to Dates
                    //Interval
                    TypeSignature::Exact(vec![
                        DataType::Interval(IntervalUnit::YearMonth),
                        DataType::Int64,
                        DataType::Date32,
                    ]),
                    TypeSignature::Exact(vec![
                        DataType::Interval(IntervalUnit::MonthDayNano),
                        DataType::Int64,
                        DataType::Date32,
                    ]),
                    TypeSignature::Exact(vec![
                        DataType::Interval(IntervalUnit::DayTime),
                        DataType::Int64,
                        DataType::Date32,
                    ]),
                    //Add to Timestamps
                    //Duration
                    TypeSignature::Exact(vec![
                        DataType::Duration(TimeUnit::Second),
                        DataType::Int64,
                        DataType::Timestamp(TimeUnit::Second, None),
                    ]),
                    TypeSignature::Exact(vec![
                        DataType::Duration(TimeUnit::Millisecond),
                        DataType::Int64,
                        DataType::Timestamp(TimeUnit::Millisecond, None),
                    ]),
                    TypeSignature::Exact(vec![
                        DataType::Duration(TimeUnit::Microsecond),
                        DataType::Int64,
                        DataType::Timestamp(TimeUnit::Microsecond, None),
                    ]),
                    TypeSignature::Exact(vec![
                        DataType::Duration(TimeUnit::Nanosecond),
                        DataType::Int64,
                        DataType::Timestamp(TimeUnit::Nanosecond, None),
                    ]),
                    //testing for such a query in curl "select dateadd('\''0 days'\'', 3,'\''2024-12-26'\'')"
                    //it asked for this specifc signature
                    TypeSignature::Exact(vec![
                        DataType::Interval(IntervalUnit::MonthDayNano),
                        DataType::Int64,
                        DataType::Timestamp(TimeUnit::Nanosecond, None),
                    ]),
                    //Add time support
                ], 
                Volatility::Immutable
            )
        }
    }
}

impl ScalarUDFImpl for DateAddFunc {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "dateadd"
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, arg_types: &[DataType]) -> datafusion::error::Result<DataType> {
        //TODO: add a match
        Ok(arg_types[2].clone())
    }
    fn invoke(&self, args: &[ColumnarValue]) -> datafusion::error::Result<ColumnarValue> {
        //DON'T DISREGARD
        // ScalarValue
        // if let (
        //     ColumnarValue::Scalar(date_or_time_part), 
        //     ColumnarValue::Scalar(value),
        //     ColumnarValue::Scalar(date_or_time_expr)
        // ) = (&args[0], &args[1], &args[2])  {
        //     match &date_or_time_part.data_type() {
        //         DataType::Duration(time_unit) => {
        //             date_or_time_expr.add(date_or_time_part)
        //         },
        //         DataType::Interval(interval_unit) => {

        //         },  
        //         _ => {
        //             exec_err!("Unsupported data format, found {}", &date_or_time_part.data_type())
        //         }
        //     }   
        // }
        Ok(args[2].clone())
    }
    //TODO add aliases
}