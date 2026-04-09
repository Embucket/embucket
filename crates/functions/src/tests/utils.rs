use crate::expr_planner::CustomExprPlanner;
use crate::session::register_session_context_udfs;
use crate::session_params::SessionParams;
use crate::table::register_udtfs;
use crate::visitors::{table_functions, table_functions_cte_relation};
use crate::{register_udafs, register_udfs};
use datafusion::config::Dialect;
use datafusion::execution::SessionStateBuilder;
use datafusion::prelude::{SessionConfig, SessionContext};
use datafusion::sql::parser::Statement as DFStatement;
use std::sync::Arc;

#[allow(clippy::unwrap_used, clippy::expect_used)]
pub fn create_session() -> Arc<SessionContext> {
    let state = SessionStateBuilder::new()
        .with_config(
            SessionConfig::new()
                .with_create_default_catalog_and_schema(true)
                .set_bool(
                    "datafusion.execution.skip_physical_aggregate_schema_check",
                    true,
                ),
        )
        .with_default_features()
        .with_expr_planners(vec![Arc::new(CustomExprPlanner)])
        .build();
    let mut ctx = SessionContext::new_with_state(state);
    register_session_context_udfs(&mut ctx).unwrap();
    register_udfs(&mut ctx, &Arc::new(SessionParams::default())).expect("Cannot register UDFs");
    register_udafs(&mut ctx).expect("Cannot register UDAFs");
    register_udtfs(&ctx);
    Arc::new(ctx)
}

pub async fn run_query(
    ctx: &SessionContext,
    query: &str,
) -> datafusion_common::Result<Vec<datafusion::arrow::record_batch::RecordBatch>> {
    let upper = query.to_ascii_uppercase();
    let sql = if upper.contains("FLATTEN") || upper.contains("TABLE(") {
        let mut statement = ctx.state().sql_to_statement(query, &Dialect::Snowflake)?;
        if let DFStatement::Statement(ref mut stmt) = statement {
            table_functions::visit(stmt);
            table_functions_cte_relation::visit(stmt);
        }
        statement.to_string()
    } else {
        query.to_string()
    };

    ctx.sql(&sql).await?.collect().await
}

#[macro_export]
macro_rules! test_query {
    (
        $test_fn_name:ident,
        $query:expr
        $(, setup_queries =[$($setup_queries:expr),* $(,)?])?
        $(, snapshot_path = $user_snapshot_path:expr)?
    ) => {
        paste::paste! {
            #[tokio::test]
            async fn [< query_ $test_fn_name >]() {
                let ctx = $crate::tests::utils::create_session();

                // Execute all setup queries (if provided) to set up the session context
                $(
                    $(
                        {
                            ctx.sql($setup_queries).await.unwrap().collect().await.unwrap();
                        }
                    )*
                )?
                let mut settings = insta::Settings::new();
                settings.set_description(stringify!($query));
                settings.set_omit_expression(true);
                settings.set_prepend_module_to_snapshot(false);
                settings.set_snapshot_path(concat!("snapshots", "/") $(.to_owned() + $user_snapshot_path)?);

                let setup: Vec<&str> = vec![$($($setup_queries),*)?];
                if !setup.is_empty() {
                    settings.set_info(&format!("Setup queries: {}", setup.join("; ")));
                }

                // Some queries may fail during Dataframe preparing, so we need to check for errors
                let res = $crate::tests::utils::run_query(&ctx, $query).await;
                if let Err(ref e) = res {
                    let err = format!("Error: {}", e);
                    settings.bind(|| {
                        insta::assert_debug_snapshot!(err);
                    });
                    return
                }

                settings.bind(|| {
                    let df = res.map(|record_batches| {
                        datafusion::arrow::util::pretty::pretty_format_batches(&record_batches).unwrap().to_string()
                    }).map_err(|e| format!("Error: {e}"));

                    let df = df.map(|df| df.split('\n').map(|s| s.to_string()).collect::<Vec<String>>());
                    insta::assert_debug_snapshot!((df));
                });
            }
        }
    };
}
