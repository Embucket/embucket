use crate::aggregate::macros::make_udaf_function;
use datafusion::arrow::array::{Array, ArrayRef, AsArray, as_string_array};
use datafusion::arrow::compute;
use datafusion::arrow::datatypes::{DataType, Field, FieldRef};
use datafusion::physical_expr::{PhysicalExpr, expressions::Literal};
use datafusion_common::utils::take_function_args;
use datafusion_common::{Result, ScalarValue, exec_err};
use datafusion_expr::function::{AccumulatorArgs, StateFieldsArgs};
use datafusion_expr::utils::{AggregateOrderSensitivity, format_state_name};
use datafusion_expr::{Accumulator, AggregateUDFImpl, Signature, Volatility};
use std::any::Any;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::mem::size_of_val;
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ListAggUDAF {
    signature: Signature,
    is_input_pre_ordered: bool,
}

impl Default for ListAggUDAF {
    fn default() -> Self {
        Self::new()
    }
}

impl ListAggUDAF {
    pub fn new() -> Self {
        Self {
            signature: Signature::variadic_any(Volatility::Immutable),
            is_input_pre_ordered: false,
        }
    }
}

impl AggregateUDFImpl for ListAggUDAF {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &'static str {
        "listagg"
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Utf8)
    }

    fn accumulator(&self, acc_args: AccumulatorArgs) -> Result<Box<dyn Accumulator>> {
        let input_arg_count = acc_args
            .exprs
            .len()
            .saturating_sub(acc_args.order_bys.len());

        if input_arg_count == 0 || input_arg_count > 2 {
            return exec_err!("LISTAGG requires 1 or 2 arguments, got {}", input_arg_count);
        }

        let delimiter = extract_delimiter(acc_args.exprs);

        if acc_args.order_bys.is_empty() {
            return Ok(Box::new(ListAggAccumulator::new(
                input_arg_count,
                acc_args.is_distinct,
                false,
                delimiter,
            )));
        }

        if acc_args.order_bys.len() != 1 {
            return exec_err!("LISTAGG supports a single ORDER BY expression");
        }

        let order_by = &acc_args.order_bys[0];
        let ordering_dtype = order_by.expr.data_type(acc_args.schema)?;

        Ok(Box::new(OrderSensitiveListAggAccumulator::new(
            input_arg_count,
            ordering_dtype,
            order_by.options.descending,
            order_by.options.nulls_first,
            self.is_input_pre_ordered,
            acc_args.is_reversed,
            acc_args.is_distinct,
            delimiter,
        )))
    }

    fn state_fields(&self, args: StateFieldsArgs) -> Result<Vec<FieldRef>> {
        let mut fields = vec![
            Arc::new(Field::new(
                format_state_name(args.name, "agg"),
                DataType::List(Arc::new(Field::new_list_field(DataType::Utf8, true))),
                true,
            )),
            Arc::new(Field::new(
                format_state_name(args.name, "delimiter"),
                DataType::Utf8,
                false,
            )),
        ];

        if let Some(ordering_field) = args.ordering_fields.first() {
            fields.push(Arc::new(Field::new(
                format_state_name(args.name, "ordering_values"),
                DataType::List(Arc::new(Field::new_list_field(
                    ordering_field.data_type().clone(),
                    true,
                ))),
                false,
            )));
        }

        Ok(fields)
    }

    fn supports_within_group_clause(&self) -> bool {
        true
    }

    fn order_sensitivity(&self) -> AggregateOrderSensitivity {
        AggregateOrderSensitivity::SoftRequirement
    }

    fn with_beneficial_ordering(
        self: Arc<Self>,
        beneficial_ordering: bool,
    ) -> Result<Option<Arc<dyn AggregateUDFImpl>>> {
        Ok(Some(Arc::new(Self {
            signature: self.signature.clone(),
            is_input_pre_ordered: beneficial_ordering,
        })))
    }
}

#[derive(Debug)]
struct ListAggAccumulator {
    values: Vec<String>,
    delimiter: String,
    delimiter_set: bool,
    input_arg_count: usize,
    is_distinct: bool,
    reverse: bool,
}

impl ListAggAccumulator {
    fn new(
        input_arg_count: usize,
        is_distinct: bool,
        reverse: bool,
        delimiter: Option<String>,
    ) -> Self {
        Self {
            values: vec![],
            delimiter: delimiter.clone().unwrap_or_default(),
            delimiter_set: delimiter.is_some(),
            input_arg_count,
            is_distinct,
            reverse,
        }
    }

    fn update_delimiter(&mut self, values: &[ArrayRef]) -> Result<()> {
        if self.input_arg_count > 1 && values.len() > 1 && !self.delimiter_set {
            if values[1].data_type() != &DataType::Null {
                let delim_string_arr = compute::cast(&values[1], &DataType::Utf8)?;
                let delim_arr = as_string_array(&delim_string_arr);
                for i in 0..delim_arr.len() {
                    if !delim_arr.is_null(i) {
                        self.delimiter = delim_arr.value(i).to_string();
                        self.delimiter_set = true;
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn evaluate_to_string(&self) -> String {
        let iter: Box<dyn Iterator<Item = String>> = if self.reverse {
            Box::new(self.values.iter().rev().cloned())
        } else {
            Box::new(self.values.iter().cloned())
        };

        let mut seen = HashSet::new();
        let mut parts = vec![];
        for value in iter {
            if self.is_distinct && !seen.insert(value.clone()) {
                continue;
            }
            parts.push(value);
        }
        combine_parts(parts, &self.delimiter)
    }
}

impl Accumulator for ListAggAccumulator {
    fn update_batch(&mut self, values: &[ArrayRef]) -> Result<()> {
        if values.is_empty() {
            return Ok(());
        }

        self.update_delimiter(values)?;

        let string_arr = compute::cast(&values[0], &DataType::Utf8)?;
        let string_arr = as_string_array(&string_arr);
        for i in 0..string_arr.len() {
            if !string_arr.is_null(i) {
                self.values.push(string_arr.value(i).to_string());
            }
        }

        Ok(())
    }

    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        if states.is_empty() {
            return Ok(());
        }

        let [agg_values, delim_values] =
            take_function_args("ListAggAccumulator::merge_batch", states)?;

        self.update_delimiter(&[agg_values.clone(), delim_values.clone()])?;

        for partition in ScalarValue::convert_array_to_scalar_vec(agg_values.as_list::<i32>())? {
            let Some(partition) = partition else {
                continue;
            };
            for value in partition {
                if let ScalarValue::Utf8(Some(value)) = value {
                    self.values.push(value);
                }
            }
        }

        Ok(())
    }

    fn state(&mut self) -> Result<Vec<ScalarValue>> {
        Ok(vec![
            ScalarValue::List(ScalarValue::new_list_from_iter(
                self.values
                    .iter()
                    .cloned()
                    .map(|value| ScalarValue::Utf8(Some(value))),
                &DataType::Utf8,
                true,
            )),
            ScalarValue::Utf8(Some(self.delimiter.clone())),
        ])
    }

    fn evaluate(&mut self) -> Result<ScalarValue> {
        Ok(ScalarValue::Utf8(Some(self.evaluate_to_string())))
    }

    fn size(&self) -> usize {
        size_of_val(self)
            + self
                .values
                .iter()
                .map(std::string::String::len)
                .sum::<usize>()
            + self.delimiter.len()
    }
}

#[derive(Debug)]
struct OrderSensitiveListAggAccumulator {
    values: Vec<String>,
    ordering_values: Vec<ScalarValue>,
    delimiter: String,
    delimiter_set: bool,
    input_arg_count: usize,
    ordering_dtype: DataType,
    descending: bool,
    nulls_first: bool,
    is_input_pre_ordered: bool,
    reverse: bool,
    is_distinct: bool,
}

impl OrderSensitiveListAggAccumulator {
    fn new(
        input_arg_count: usize,
        ordering_dtype: DataType,
        descending: bool,
        nulls_first: bool,
        is_input_pre_ordered: bool,
        reverse: bool,
        is_distinct: bool,
        delimiter: Option<String>,
    ) -> Self {
        Self {
            values: vec![],
            ordering_values: vec![],
            delimiter: delimiter.clone().unwrap_or_default(),
            delimiter_set: delimiter.is_some(),
            input_arg_count,
            ordering_dtype,
            descending,
            nulls_first,
            is_input_pre_ordered,
            reverse,
            is_distinct,
        }
    }

    fn update_delimiter(&mut self, values: &[ArrayRef]) -> Result<()> {
        if self.input_arg_count > 1 && values.len() > 1 && !self.delimiter_set {
            if values[1].data_type() != &DataType::Null {
                let delim_string_arr = compute::cast(&values[1], &DataType::Utf8)?;
                let delim_arr = as_string_array(&delim_string_arr);
                for i in 0..delim_arr.len() {
                    if !delim_arr.is_null(i) {
                        self.delimiter = delim_arr.value(i).to_string();
                        self.delimiter_set = true;
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn sort(&mut self) {
        let mut combined = std::mem::take(&mut self.values)
            .into_iter()
            .zip(std::mem::take(&mut self.ordering_values))
            .collect::<Vec<_>>();
        combined.sort_by(|(_, left), (_, right)| {
            compare_order_values(left, right, self.descending, self.nulls_first)
        });
        (self.values, self.ordering_values) = combined.into_iter().unzip();
    }

    fn evaluate_to_string(&self) -> String {
        let iter: Box<dyn Iterator<Item = String>> = if self.reverse {
            Box::new(self.values.iter().rev().cloned())
        } else {
            Box::new(self.values.iter().cloned())
        };

        let mut seen = HashSet::new();
        let mut parts = vec![];
        for value in iter {
            if self.is_distinct && !seen.insert(value.clone()) {
                continue;
            }
            parts.push(value);
        }
        combine_parts(parts, &self.delimiter)
    }
}

impl Accumulator for OrderSensitiveListAggAccumulator {
    fn update_batch(&mut self, values: &[ArrayRef]) -> Result<()> {
        if values.is_empty() {
            return Ok(());
        }

        let value_arr = values
            .iter()
            .find(|array| array.data_type() != &self.ordering_dtype)
            .unwrap_or(&values[0]);
        let string_arr = compute::cast(value_arr, &DataType::Utf8)?;
        let string_arr = as_string_array(&string_arr);
        let ordering_arr = values[1..]
            .iter()
            .find(|array| array.data_type() == &self.ordering_dtype)
            .unwrap_or(&values[1]);

        for i in 0..string_arr.len() {
            if string_arr.is_null(i) {
                continue;
            }
            self.values.push(string_arr.value(i).to_string());
            self.ordering_values
                .push(ScalarValue::try_from_array(ordering_arr, i)?.compacted());
        }

        Ok(())
    }

    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        if states.is_empty() {
            return Ok(());
        }

        let [agg_values, delim_values, ordering_values] =
            take_function_args("OrderSensitiveListAggAccumulator::merge_batch", states)?;

        self.update_delimiter(&[agg_values.clone(), delim_values.clone()])?;

        let value_partitions =
            ScalarValue::convert_array_to_scalar_vec(agg_values.as_list::<i32>())?;
        let ordering_partitions =
            ScalarValue::convert_array_to_scalar_vec(ordering_values.as_list::<i32>())?;

        for (partition_values, partition_orderings) in value_partitions
            .into_iter()
            .zip(ordering_partitions.into_iter())
        {
            let Some(partition_values) = partition_values else {
                continue;
            };
            let Some(partition_orderings) = partition_orderings else {
                continue;
            };

            for (value, ordering_value) in partition_values
                .into_iter()
                .zip(partition_orderings.into_iter())
            {
                if let ScalarValue::Utf8(Some(value)) = value {
                    self.values.push(value);
                    self.ordering_values.push(ordering_value.compacted());
                }
            }
        }

        Ok(())
    }

    fn state(&mut self) -> Result<Vec<ScalarValue>> {
        if !self.is_input_pre_ordered {
            self.sort();
        }

        Ok(vec![
            ScalarValue::List(ScalarValue::new_list_from_iter(
                self.values
                    .iter()
                    .cloned()
                    .map(|value| ScalarValue::Utf8(Some(value))),
                &DataType::Utf8,
                true,
            )),
            ScalarValue::Utf8(Some(self.delimiter.clone())),
            ScalarValue::List(ScalarValue::new_list_from_iter(
                self.ordering_values.clone().into_iter(),
                &self.ordering_dtype,
                true,
            )),
        ])
    }

    fn evaluate(&mut self) -> Result<ScalarValue> {
        if !self.is_input_pre_ordered {
            self.sort();
        }
        Ok(ScalarValue::Utf8(Some(self.evaluate_to_string())))
    }

    fn size(&self) -> usize {
        size_of_val(self)
            + self
                .values
                .iter()
                .map(std::string::String::len)
                .sum::<usize>()
            + self.delimiter.len()
            + ScalarValue::size_of_vec(&self.ordering_values)
    }
}

fn compare_order_values(
    left: &ScalarValue,
    right: &ScalarValue,
    descending: bool,
    nulls_first: bool,
) -> Ordering {
    match (left.is_null(), right.is_null()) {
        (true, true) => Ordering::Equal,
        (true, false) => {
            if nulls_first {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
        (false, true) => {
            if nulls_first {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
        (false, false) => {
            let ordering = left.partial_cmp(right).unwrap_or(Ordering::Equal);
            if descending {
                ordering.reverse()
            } else {
                ordering
            }
        }
    }
}

fn extract_delimiter(exprs: &[Arc<dyn PhysicalExpr>]) -> Option<String> {
    exprs.iter().find_map(|expr| {
        expr.as_any()
            .downcast_ref::<Literal>()
            .and_then(|literal| match literal.value() {
                ScalarValue::Utf8(Some(value)) => Some(value.clone()),
                ScalarValue::Utf8View(Some(value)) => Some(value.to_string()),
                _ => None,
            })
    })
}

fn combine_parts(parts: Vec<String>, delimiter: &str) -> String {
    let mut result = String::new();
    for part in parts {
        if !result.is_empty() {
            result.push_str(delimiter);
        }
        result.push_str(&part);
    }
    result
}

make_udaf_function!(ListAggUDAF);
