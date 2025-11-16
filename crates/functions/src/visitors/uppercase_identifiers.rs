use datafusion_expr::sqlparser::ast::{self as ast, VisitMut, VisitorMut};
use std::ops::ControlFlow;

pub struct UppercaseIdentifiers;

impl VisitorMut for UppercaseIdentifiers {
    type Break = ();

    fn pre_visit_query(&mut self, query: &mut ast::Query) -> ControlFlow<Self::Break> {
        if let ast::SetExpr::Select(select) = &mut *query.body {
            for item in &mut select.projection {
                uppercase_select_alias(item);
            }
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_expr(&mut self, expr: &mut ast::Expr) -> ControlFlow<Self::Break> {
        match expr {
            ast::Expr::Identifier(ident) => {
                uppercase_ident(ident);
                quote_ident(ident);
            }
            ast::Expr::CompoundIdentifier(idents) => {
                idents.iter_mut().for_each(uppercase_ident);
                if let Some(last) = idents.last_mut() {
                    quote_ident(last);
                }
            }
            ast::Expr::QualifiedWildcard(prefix, _) => uppercase_object_name(prefix),
            ast::Expr::Function(func) => uppercase_object_name(&mut func.name),
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_table_factor(
        &mut self,
        factor: &mut ast::TableFactor,
    ) -> ControlFlow<Self::Break> {
        match factor {
            ast::TableFactor::Table {
                alias: Some(alias), ..
            }
            | ast::TableFactor::Derived {
                alias: Some(alias), ..
            }
            | ast::TableFactor::TableFunction {
                alias: Some(alias), ..
            }
            | ast::TableFactor::NestedJoin {
                alias: Some(alias), ..
            } => {
                uppercase_table_alias(alias);
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }
}

fn uppercase_ident(ident: &mut ast::Ident) {
    if ident.quote_style.is_none() {
        ident.value.make_ascii_uppercase();
    }
}

fn uppercase_object_name(name: &mut ast::ObjectName) {
    for part in &mut name.0 {
        if let ast::ObjectNamePart::Identifier(ident) = part {
            uppercase_ident(ident);
        }
    }
}

const fn quote_ident(ident: &mut ast::Ident) {
    if ident.quote_style.is_none() {
        ident.quote_style = Some('"');
    }
}

fn uppercase_select_alias(item: &mut ast::SelectItem) {
    if let ast::SelectItem::ExprWithAlias { alias, .. } = item {
        uppercase_alias_ident(alias);
    }
}

fn uppercase_table_alias(alias: &mut ast::TableAlias) {
    uppercase_alias_ident(&mut alias.name);
    for column in &mut alias.columns {
        uppercase_alias_ident(&mut column.name);
    }
}

fn uppercase_alias_ident(ident: &mut ast::Ident) {
    uppercase_ident(ident);
    quote_ident(ident);
}

pub fn visit(statement: &mut ast::Statement) {
    let mut visitor = UppercaseIdentifiers;
    let _ = statement.visit(&mut visitor);
}
