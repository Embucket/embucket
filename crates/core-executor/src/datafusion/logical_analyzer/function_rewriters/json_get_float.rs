use arrow_schema::DataType;
use datafusion::common::config::ConfigOptions;
use datafusion::common::tree_node::Transformed;
use datafusion::common::DFSchema;
use datafusion::common::Result;
use datafusion::logical_expr::expr::{Expr, ScalarFunction};
use datafusion::logical_expr::expr_rewriter::FunctionRewrite;
use datafusion_expr::{Cast, ScalarUDF};
use std::sync::Arc;

/// Rewrites calls to `json_get_float` by wrapping them in a `to_decimal` function.
///
/// # Context
///
/// When querying JSON fields, expressions like `jsontext['id']::NUMBER` are parsed into calls
/// to `json_get` functions by the visitor. During query analysis, a `FunctionRewriter` replaces
/// `json_get` with `json_get_float` if the inferred type of the JSON field is one of:
/// - `DataType::Float64`
/// - `DataType::Float32`
/// - `DataType::Decimal128(_, _)`
/// - `DataType::Decimal256(_, _)`
///
/// However, `json_get_float` always returns a `Float64` by default, whereas the system expects
/// a decimal type for these fields. To correctly handle this, this rewrite rule wraps
/// the `json_get_float` call inside a `to_decimal(...)` function call to ensure proper casting
/// of the returned value to a decimal type.
///
/// This rule ensures type correctness and prevents runtime casting errors when working with
/// decimal-typed JSON fields.
///
/// # Example
///
/// ```sql
/// SELECT to_decimal(json_get_float(obj, 'id')) FROM ...
/// ```
#[derive(Debug)]
pub struct JsonGetFloatRewriter {
    json_get_float_func: Arc<ScalarUDF>,
}

impl JsonGetFloatRewriter {
    #[must_use]
    pub const fn new(json_get_float_func: Arc<ScalarUDF>) -> Self {
        Self {
            json_get_float_func,
        }
    }
}

impl FunctionRewrite for JsonGetFloatRewriter {
    fn name(&self) -> &'static str {
        "JsonGetFloatRewriter"
    }

    fn rewrite(
        &self,
        expr: Expr,
        _schema: &DFSchema,
        _config: &ConfigOptions,
    ) -> Result<Transformed<Expr>> {
        let transform = match &expr {
            Expr::Cast(cast) => optimise_json_get_cast(self.json_get_float_func.clone(), cast),
            _ => None,
        };
        Ok(transform.unwrap_or_else(|| Transformed::no(expr)))
    }
}

fn optimise_json_get_cast(func_to_call: Arc<ScalarUDF>, cast: &Cast) -> Option<Transformed<Expr>> {
    let scalar_func = extract_scalar_func(&cast.expr)?;
    if scalar_func.func.name() != "json_get" {
        return None;
    }
    let func = match &cast.data_type {
        DataType::Float64
        | DataType::Float32
        | DataType::Decimal128(_, _)
        | DataType::Decimal256(_, _) => func_to_call,
        _ => return None,
    };
    let new_func_expr = Expr::ScalarFunction(ScalarFunction {
        func,
        args: scalar_func.args.clone(),
    });
    let new_expr = Expr::Cast(Cast {
        expr: Box::new(new_func_expr),
        data_type: cast.data_type.clone(),
    });

    Some(Transformed::yes(new_expr))
}

fn extract_scalar_func(expr: &Expr) -> Option<&ScalarFunction> {
    match expr {
        Expr::ScalarFunction(func) => Some(func),
        Expr::Alias(alias) => extract_scalar_func(&alias.expr),
        _ => None,
    }
}
