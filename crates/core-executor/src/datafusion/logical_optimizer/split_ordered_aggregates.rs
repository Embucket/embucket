use std::collections::HashMap;

use datafusion::optimizer::{ApplyOrder, OptimizerConfig, OptimizerRule};
use datafusion_common::Column;
use datafusion_common::Result;
use datafusion_common::tree_node::Transformed;
use datafusion_expr::expr::Sort as ExprSort;
use datafusion_expr::{Expr, LogicalPlan, LogicalPlanBuilder, col};

// --- type aliases to keep function signatures simple ---
type AggrIndex = usize;
type OrderedClasses = HashMap<String, Vec<(AggrIndex, Expr)>>;
type UnorderedList = Vec<(AggrIndex, Expr)>;
type NameMap = Vec<(usize, String)>;
type GroupKeys = Vec<String>;
type Branch = (LogicalPlan, NameMap);
type MergeResult = (LogicalPlan, NameMap, GroupKeys);

#[derive(Debug, Default)]
pub struct SplitOrderedAggregates {}

impl SplitOrderedAggregates {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl OptimizerRule for SplitOrderedAggregates {
    fn name(&self) -> &'static str {
        "split_ordered_aggregates"
    }

    fn apply_order(&self) -> Option<ApplyOrder> {
        Some(ApplyOrder::BottomUp)
    }

    fn rewrite(
        &self,
        plan: LogicalPlan,
        _cfg: &dyn OptimizerConfig,
    ) -> Result<Transformed<LogicalPlan>> {
        let LogicalPlan::Aggregate(agg) = plan.clone() else {
            return Ok(Transformed::no(plan));
        };

        // Partition aggregate expressions by ORDER BY signature
        let (ordered_classes, unordered) = partition_aggregates(&agg.aggr_expr);
        if ordered_classes.len() <= 1 {
            return Ok(Transformed::no(plan));
        }

        // Build per-signature aggregate branches
        let mut class_iters = ordered_classes.into_iter();
        let mut branches: Vec<(LogicalPlan, Vec<(usize, String)>)> = Vec::new();
        if let Some((_sig, mut aggs0)) = class_iters.next() {
            aggs0.extend(unordered);
            branches.push(build_branch((*agg.input).clone(), &agg.group_expr, &aggs0)?);
        }
        for (_sig, aggs_i) in class_iters {
            branches.push(build_branch(
                (*agg.input).clone(),
                &agg.group_expr,
                &aggs_i,
            )?);
        }

        // Merge branches (join on synthesized group keys)
        let (acc_plan, name_map, group_keys) = merge_branches(branches)?;

        // Final projection with original column names
        let original_field_names: Vec<String> = agg
            .schema
            .fields()
            .iter()
            .map(|f| f.name().clone())
            .collect();
        let original_group_count = original_field_names.len() - agg.aggr_expr.len();
        let rewritten = build_final_projection(
            acc_plan,
            &original_field_names,
            original_group_count,
            &group_keys,
            name_map,
        )?;

        Ok(Transformed::yes(rewritten))
    }
}

fn partition_aggregates(aggr_exprs: &[Expr]) -> (OrderedClasses, UnorderedList) {
    let mut ordered_classes: OrderedClasses = HashMap::new();
    let mut unordered: UnorderedList = Vec::new();
    for (i, e) in aggr_exprs.iter().cloned().enumerate() {
        if let Some(order_by) = find_ordering_in_aggregate(&e) {
            let sig = ordering_signature(order_by);
            ordered_classes.entry(sig).or_default().push((i, e));
        } else {
            unordered.push((i, e));
        }
    }
    (ordered_classes, unordered)
}

fn build_branch(
    input: LogicalPlan,
    group_expr: &[Expr],
    aggs: &[(AggrIndex, Expr)],
) -> Result<Branch> {
    let branch_lp = LogicalPlanBuilder::from(input)
        .aggregate(
            group_expr.iter().cloned(),
            aggs.iter().map(|(_, e)| e.clone()),
        )?
        .build()?;

    let group_fields_count = branch_lp.schema().fields().len() - aggs.len();
    let out_fields = branch_lp.schema().fields();

    // Project to stable group key names gk{idx} and unique aggregate names agg_{orig_idx}
    let mut proj_exprs: Vec<Expr> = Vec::with_capacity(branch_lp.schema().fields().len());
    for i in 0..group_fields_count {
        let name = out_fields[i].name();
        proj_exprs.push(col(name).alias(format!("gk{i}")));
    }
    let mut out_names: NameMap = Vec::with_capacity(aggs.len());
    for (k, (orig_idx, _)) in aggs.iter().enumerate() {
        let src_name = out_fields[group_fields_count + k].name();
        let alias = format!("agg_{orig_idx}");
        proj_exprs.push(col(src_name).alias(alias.clone()));
        out_names.push((*orig_idx, alias));
    }

    let projected = LogicalPlanBuilder::from(branch_lp)
        .project(proj_exprs)?
        .build()?;
    Ok((projected, out_names))
}

fn compute_group_keys(acc_plan: &LogicalPlan, agg_count: usize) -> GroupKeys {
    let total_fields = acc_plan.schema().fields().len();
    let group_fields_count = total_fields - agg_count;
    (0..group_fields_count).map(|i| format!("gk{i}")).collect()
}

fn merge_branches(mut branches: Vec<Branch>) -> Result<MergeResult> {
    let (mut acc_plan, mut name_map) = branches.remove(0);
    let group_keys = compute_group_keys(&acc_plan, name_map.len());
    for (lp, out) in branches {
        let left_schema = acc_plan.schema();
        let right_schema = lp.schema();
        let left_has_all = group_keys
            .iter()
            .all(|k| left_schema.has_column(&Column::from_name(k.clone())));
        let right_has_all = group_keys
            .iter()
            .all(|k| right_schema.has_column(&Column::from_name(k.clone())));

        acc_plan = if group_keys.is_empty() || !left_has_all || !right_has_all {
            LogicalPlanBuilder::from(acc_plan).cross_join(lp)?.build()?
        } else {
            let right_for_join = lp.clone();
            let right_for_cross = lp;
            match LogicalPlanBuilder::from(acc_plan.clone()).join_using(
                right_for_join,
                datafusion_common::JoinType::Inner,
                group_keys.clone(),
            ) {
                Ok(b) => match b.build() {
                    Ok(p) => p,
                    Err(_) => LogicalPlanBuilder::from(acc_plan)
                        .cross_join(right_for_cross)?
                        .build()?,
                },
                Err(_) => LogicalPlanBuilder::from(acc_plan)
                    .cross_join(right_for_cross)?
                    .build()?,
            }
        };

        name_map.extend(out.into_iter());
    }
    Ok((acc_plan, name_map, group_keys))
}

fn build_final_projection(
    acc_plan: LogicalPlan,
    original_field_names: &[String],
    original_group_count: usize,
    group_keys: &GroupKeys,
    mut name_map: NameMap,
) -> Result<LogicalPlan> {
    let mut proj_exprs: Vec<Expr> = Vec::with_capacity(original_field_names.len());

    for (i, f_name) in original_field_names
        .iter()
        .take(original_group_count)
        .enumerate()
    {
        proj_exprs.push(col(&group_keys[i]).alias(f_name));
    }

    name_map.sort_by_key(|(idx, _)| *idx);
    for (i, (_, tmp_name)) in name_map.into_iter().enumerate() {
        let f_name = &original_field_names[original_group_count + i];
        proj_exprs.push(col(&tmp_name).alias(f_name));
    }

    LogicalPlanBuilder::from(acc_plan)
        .project(proj_exprs)?
        .build()
}

fn ordering_signature(order_by: &[ExprSort]) -> String {
    let mut parts = Vec::with_capacity(order_by.len());
    for s in order_by {
        let expr = s.expr.human_display().to_string();
        let asc = s.asc;
        let nulls_first = s.nulls_first;
        parts.push(format!("{expr}|{asc}|{nulls_first}"));
    }
    parts.join(";")
}

fn find_ordering_in_aggregate(expr: &Expr) -> Option<&[ExprSort]> {
    match expr {
        Expr::AggregateFunction(af) => af.params.order_by.as_deref(),
        Expr::Alias(a) => find_ordering_in_aggregate(&a.expr),
        Expr::Cast(c) => find_ordering_in_aggregate(&c.expr),
        Expr::TryCast(c) => find_ordering_in_aggregate(&c.expr),
        _ => None,
    }
}
