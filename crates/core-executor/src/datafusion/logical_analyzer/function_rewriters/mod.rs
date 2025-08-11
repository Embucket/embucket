use datafusion_expr::registry::FunctionRegistry;
use std::sync::Arc;

pub mod json_get_float;

pub fn register_func_rewriters(
    registry: &mut dyn FunctionRegistry,
) -> datafusion_common::Result<()> {
    registry.register_function_rewrite(Arc::new(
        json_get_float::JsonGetFloatRewriter::new(registry.udf("json_get_float")?),
    ))?;
    
    Ok(())
}
