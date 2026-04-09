use datafusion::logical_expr::sqlparser::ast::{
    Expr, FunctionArg, FunctionArgExpr, TableFactor, Value, ValueWithSpan,
    VisitMut,
};
use datafusion::sql::sqlparser::ast::{
    Function, FunctionArguments, Query, SetExpr, Statement, VisitorMut,
};
use datafusion::sql::sqlparser::tokenizer::{Location, Span};
use std::ops::ControlFlow;

fn flatten_default_string(value: &str) -> FunctionArg {
    FunctionArg::Unnamed(FunctionArgExpr::Expr(Expr::Value(ValueWithSpan {
        value: Value::SingleQuotedString(value.to_string()),
        span: Span::new(Location::new(0, 0), Location::new(0, 0)),
    })))
}

fn flatten_default_bool(value: bool) -> FunctionArg {
    FunctionArg::Unnamed(FunctionArgExpr::Expr(Expr::Value(ValueWithSpan {
        value: Value::Boolean(value),
        span: Span::new(Location::new(0, 0), Location::new(0, 0)),
    })))
}

fn rewrite_flatten_args(args: Vec<FunctionArg>) -> Vec<FunctionArg> {
    if !args.iter().any(|arg| matches!(arg, FunctionArg::Named { .. })) {
        return args;
    }

    let mut input = None;
    let mut path = None;
    let mut outer = None;
    let mut recursive = None;
    let mut mode = None;

    for arg in args {
        match arg {
            FunctionArg::Named {
                name,
                arg,
                operator: _,
            } => match name.to_string().to_ascii_lowercase().as_str() {
                "input" => input = Some(FunctionArg::Unnamed(arg)),
                "path" => path = Some(FunctionArg::Unnamed(arg)),
                "outer" | "is_outer" => outer = Some(FunctionArg::Unnamed(arg)),
                "recursive" | "is_recursive" => recursive = Some(FunctionArg::Unnamed(arg)),
                "mode" => mode = Some(FunctionArg::Unnamed(arg)),
                _ => {}
            },
            FunctionArg::Unnamed(arg) => {
                if input.is_none() {
                    input = Some(FunctionArg::Unnamed(arg));
                }
            }
            _ => {}
        }
    }

    input.map_or_else(Vec::new, |input| {
        vec![
            input,
            path.unwrap_or_else(|| flatten_default_string("")),
            outer.unwrap_or_else(|| flatten_default_bool(false)),
            recursive.unwrap_or_else(|| flatten_default_bool(false)),
            mode.unwrap_or_else(|| flatten_default_string("both")),
        ]
    })
}

/// A SQL AST visitor that rewrites `TABLE(<FUNCTION>(...))` table functions
/// into `<FUNCTION>(...)` by removing the unnecessary `TABLE(...)` wrapper.
///
/// This transformation is useful because in many SQL dialects, especially Snowflake-like syntax,
/// queries such as:
///
/// ```sql
/// SELECT * FROM TABLE(<FUNCTION>(LAST_QUERY_ID())) WHERE value > 1;
/// ```
///
/// are semantically equivalent to:
///
/// ```sql
/// SELECT * FROM <FUNCTION>(LAST_QUERY_ID()) WHERE value > 1;
/// ```
///
/// However, the presence of the `TABLE(...)` wrapper can complicate query parsing
/// or downstream analysis in some tools, such as logical planners or optimizers.
/// This visitor simplifies the AST by stripping the redundant `TABLE(...)`
/// call when it wraps a single `<FUNCTION>(...)` function call.
///
/// # How it works:
/// - It traverses SQL `Query` nodes in the AST.
/// - For each `FROM` clause entry that is a `TableFactor::TableFunction`, it checks whether the expression is:
///     - A function call named `TABLE`,
///     - With exactly one argument,
///     - And that argument is a function call named `<FUNCTION>`.
/// - If all conditions are met, it replaces the outer `TABLE(...)` function expression
///   with the inner `<FUNCTION>(...)` function directly.
///
/// This transformation is performed in-place using the `VisitorMut` trait.
#[derive(Debug, Default)]
pub struct TableFunctionVisitor {}

impl VisitorMut for TableFunctionVisitor {
    type Break = ();

    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<Self::Break> {
        if let SetExpr::Select(select) = query.body.as_mut() {
            for item in &mut select.from {
                if let TableFactor::TableFunction {
                    expr:
                        Expr::Function(Function {
                            name,
                            args: FunctionArguments::List(args),
                            ..
                        }),
                    alias,
                } = &mut item.relation
                {
                    let func_name = name.to_string();
                    if matches!(func_name.to_lowercase().as_str(), "result_scan" | "flatten") {
                        item.relation = TableFactor::Function {
                            name: name.clone(),
                            args: if func_name.eq_ignore_ascii_case("flatten") {
                                rewrite_flatten_args(args.args.clone())
                            } else {
                                args.args.clone()
                            },
                            alias: alias.clone(),
                            lateral: false,
                        };
                    }
                } else if let TableFactor::Function { name, args, .. } = &mut item.relation
                    && name.to_string().eq_ignore_ascii_case("flatten")
                {
                    *args = rewrite_flatten_args(std::mem::take(args));
                } else if let TableFactor::Table {
                    name,
                    args: Some(table_args),
                    ..
                } = &mut item.relation
                    && name.to_string().eq_ignore_ascii_case("flatten")
                {
                    table_args.args = rewrite_flatten_args(std::mem::take(&mut table_args.args));
                }
            }
        }
        ControlFlow::Continue(())
    }
}

pub fn visit(stmt: &mut Statement) {
    let _ = stmt.visit(&mut TableFunctionVisitor {});
}
