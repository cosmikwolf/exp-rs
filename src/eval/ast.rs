extern crate alloc;
use crate::Real;
use crate::context::EvalContext;
use crate::error::ExprError;
use crate::types::AstExpr;
#[cfg(not(test))]
use alloc::rc::Rc;
#[cfg(test)]
use std::rc::Rc;

pub fn eval_ast<'arena>(ast: &'arena AstExpr<'arena>, ctx: Option<Rc<EvalContext>>, arena: &'arena bumpalo::Bump) -> Result<Real, ExprError> {
    // Use the iterative evaluator - this eliminates stack overflow issues
    // and provides better performance than the recursive approach
    crate::eval::iterative::eval_iterative(ast, ctx, arena)
}

