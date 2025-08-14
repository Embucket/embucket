use arrow_schema::DataType;
use datafusion::common::DFSchema;
use datafusion::common::Result;
use datafusion::common::config::ConfigOptions;
use datafusion::common::tree_node::Transformed;
use datafusion::logical_expr::expr::{Expr, ScalarFunction};
use datafusion::logical_expr::expr_rewriter::FunctionRewrite;
use datafusion_expr::{Cast, ScalarUDF};
use embucket_functions::semi_structured::json::json_get_float::JsonGetFloat;
use std::sync::Arc;

/// Rewrites `json_get` call to `json_get_float` for specific datatypes.
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
#[derive(Debug)]
pub struct JsonGetFloatRewriter {}

impl Default for JsonGetFloatRewriter {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonGetFloatRewriter {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
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
            Expr::Cast(cast) => optimise_json_get_cast(cast),
            _ => None,
        };
        Ok(transform.unwrap_or_else(|| Transformed::no(expr)))
    }
}

fn optimise_json_get_cast(cast: &Cast) -> Option<Transformed<Expr>> {
    let scalar_func = extract_scalar_func(&cast.expr)?;
    if scalar_func.func.name() != "json_get" {
        return None;
    }
    match &cast.data_type {
        DataType::Float64
        | DataType::Float32
        | DataType::Decimal128(_, _)
        | DataType::Decimal256(_, _) => {
            let new_expr = Expr::ScalarFunction(ScalarFunction {
                func: Arc::new(ScalarUDF::from(JsonGetFloat::default())),
                args: scalar_func.args.clone(),
            });
            Some(Transformed::yes(new_expr))
        }
        _ => None,
    }
}

fn extract_scalar_func(expr: &Expr) -> Option<&ScalarFunction> {
    match expr {
        Expr::ScalarFunction(func) => Some(func),
        Expr::Alias(alias) => extract_scalar_func(&alias.expr),
        _ => None,
    }
}
