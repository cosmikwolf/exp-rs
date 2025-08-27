//! Expression functions implementation for the exp-rs library.
//!
//! This module provides functionality for defining and evaluating functions
//! that are specified as expression strings rather than native Rust code.

extern crate alloc;
use crate::Real;
use crate::context::EvalContext;
use crate::error::Result;
use crate::eval::eval_ast;
use crate::types::AstExpr;
use crate::types::TryIntoHeaplessString;
use alloc::borrow::Cow;
#[cfg(not(test))]
use alloc::rc::Rc;
use alloc::string::ToString;
#[cfg(test)]
use std::rc::Rc;

/// Evaluates an expression function with the given arguments.
///
/// This is a helper function used internally by the evaluation logic.
pub fn eval_expression_function<'a, 'arena>(
    ast: &'arena AstExpr<'arena>,
    param_names: &[Cow<'a, str>],
    arg_values: &[Real],
    parent_ctx: Option<Rc<EvalContext>>,
    arena: &'arena bumpalo::Bump,
) -> Result<Real> {
    let mut temp_ctx = EvalContext::new();
    if let Some(parent) = parent_ctx {
        temp_ctx.parent = Some(Rc::clone(&parent));
    }
    for (param_name, &arg_val) in param_names.iter().zip(arg_values.iter()) {
        if let Ok(key) = param_name.to_string().try_into_heapless() {
            let _ = temp_ctx.variables.insert(key, arg_val);
        }
    }
    eval_ast(ast, Some(Rc::new(temp_ctx)), arena)
}

