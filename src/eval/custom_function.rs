extern crate alloc;
use crate::Real;
#[cfg(not(test))]
use crate::Vec;
use crate::context::EvalContext;
use crate::error::ExprError;
use crate::eval::*;

// Import these for the tests
// Only needed if builtins are enabled
use crate::types::{AstExpr, TryIntoHeaplessString};
#[cfg(not(test))]
use alloc::rc::Rc;
#[cfg(test)]
use std::rc::Rc;
#[cfg(test)]
use std::vec::Vec;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};

// --- Helper functions for each FunctionCacheEntry arm ---

pub trait CustomFunction {
    fn params(&self) -> Vec<String>;
    fn body_str(&self) -> String;
}

impl CustomFunction for crate::context::UserFunction {
    fn params(&self) -> Vec<String> {
        self.params.clone()
    }
    fn body_str(&self) -> String {
        self.body.clone()
    }
}

impl CustomFunction for crate::types::ExpressionFunction {
    fn params(&self) -> Vec<String> {
        self.params.clone()
    }
    fn body_str(&self) -> String {
        self.expression.clone()
    }
}

pub fn eval_custom_function<'a, F>(
    name: &str,
    args: &[AstExpr],
    ctx: Option<Rc<EvalContext>>,
    func_cache: &mut BTreeMap<String, Option<FunctionCacheEntry>>,
    var_cache: &mut BTreeMap<String, Real>,
    func: &F,
) -> Result<Real, ExprError>
where
    F: CustomFunction,
{
    if args.len() != func.params().len() {
        return Err(ExprError::InvalidFunctionCall {
            name: name.to_string(),
            expected: func.params().len(),
            found: args.len(),
        });
    }
    // First evaluate all arguments in the parent context
    let mut arg_values = Vec::with_capacity(args.len());
    for arg in args {
        let arg_val = eval_ast_inner(arg, ctx.clone(), func_cache, var_cache)?;
        arg_values.push(arg_val);
    }

    // Create a fresh context with the function arguments
    let mut func_ctx = EvalContext::new();

    // Add all parameters with their values
    for (i, param_name) in func.params().iter().enumerate() {
        func_ctx.set_parameter(param_name, arg_values[i]);
    }

    // Copy function registry from parent to access all functions
    if let Some(parent) = &ctx {
        func_ctx.function_registry = parent.function_registry.clone();
    }

    // We'll use the passed-in function cache directly

    // This function is only called by the recursive evaluator (eval_ast_inner)
    // which is being phased out in favor of the iterative evaluator.
    // The iterative evaluator handles expression functions directly and correctly.
    // 
    // For now, return an error to make it clear this path shouldn't be used.
    Err(ExprError::Other(
        "Expression functions should be evaluated through the iterative evaluator, not the recursive path".to_string()
    ))
}