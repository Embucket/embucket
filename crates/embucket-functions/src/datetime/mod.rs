use crate::date_part_extract::DatePartExtractFunc;
use datafusion_expr::ScalarUDF;
use datafusion_expr::registry::FunctionRegistry;
use std::sync::Arc;

pub mod add_months;
pub mod convert_timezone;
pub mod date_add;
pub mod date_diff;
pub mod date_from_parts;
pub mod date_part_extract;
pub mod dayname;
pub mod last_day;
pub mod monthname;
pub mod next_day;
pub mod previous_day;
pub mod time_from_parts;
pub mod timestamp_from_parts;

#[allow(clippy::too_many_lines)]
pub fn register_udfs(registry: &mut dyn FunctionRegistry) -> datafusion_common::Result<()> {
    let functions: Vec<Arc<ScalarUDF>> = vec![
        convert_timezone::get_udf(),
        date_add::get_udf(),
        date_diff::get_udf(),
        timestamp_from_parts::get_udf(),
        time_from_parts::get_udf(),
        date_from_parts::get_udf(),
        last_day::get_udf(),
        add_months::get_udf(),
        monthname::get_udf(),
        dayname::get_udf(),
        previous_day::get_udf(),
        next_day::get_udf(),
        Arc::new(ScalarUDF::from(DatePartExtractFunc::new(
            date_part_extract::Interval::Year,
        ))),
        Arc::new(ScalarUDF::from(DatePartExtractFunc::new(
            date_part_extract::Interval::YearOfWeek,
        ))),
        Arc::new(ScalarUDF::from(DatePartExtractFunc::new(
            date_part_extract::Interval::YearOfWeekIso,
        ))),
        Arc::new(ScalarUDF::from(DatePartExtractFunc::new(
            date_part_extract::Interval::Day,
        ))),
        Arc::new(ScalarUDF::from(DatePartExtractFunc::new(
            date_part_extract::Interval::DayOfMonth,
        ))),
        Arc::new(ScalarUDF::from(DatePartExtractFunc::new(
            date_part_extract::Interval::DayOfWeek,
        ))),
        Arc::new(ScalarUDF::from(DatePartExtractFunc::new(
            date_part_extract::Interval::DayOfWeekIso,
        ))),
        Arc::new(ScalarUDF::from(DatePartExtractFunc::new(
            date_part_extract::Interval::DayOfYear,
        ))),
        Arc::new(ScalarUDF::from(DatePartExtractFunc::new(
            date_part_extract::Interval::Week,
        ))),
        Arc::new(ScalarUDF::from(DatePartExtractFunc::new(
            date_part_extract::Interval::WeekOfYear,
        ))),
        Arc::new(ScalarUDF::from(DatePartExtractFunc::new(
            date_part_extract::Interval::WeekIso,
        ))),
        Arc::new(ScalarUDF::from(DatePartExtractFunc::new(
            date_part_extract::Interval::Month,
        ))),
        Arc::new(ScalarUDF::from(DatePartExtractFunc::new(
            date_part_extract::Interval::Quarter,
        ))),
        Arc::new(ScalarUDF::from(DatePartExtractFunc::new(
            date_part_extract::Interval::Hour,
        ))),
        Arc::new(ScalarUDF::from(DatePartExtractFunc::new(
            date_part_extract::Interval::Minute,
        ))),
        Arc::new(ScalarUDF::from(DatePartExtractFunc::new(
            date_part_extract::Interval::Second,
        ))),
    ];

    for func in functions {
        registry.register_udf(func)?;
    }

    Ok(())
}
