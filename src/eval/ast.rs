extern crate alloc;
use super::*;
use crate::Real;
use crate::context::EvalContext;
use crate::error::ExprError;

// Only needed if builtins are enabled
use crate::types::AstExpr;
#[cfg(not(test))]
use alloc::rc::Rc;
#[cfg(test)]
use std::rc::Rc;

use alloc::collections::BTreeMap;
use alloc::string::String;

#[cfg(feature = "libm")]
#[allow(dead_code)]
type MathFunc = fn(Real, Real) -> Real;

pub fn eval_ast<'a>(ast: &AstExpr, ctx: Option<Rc<EvalContext>>) -> Result<Real, ExprError> {
    // Use the iterative evaluator - this eliminates stack overflow issues
    // and provides better performance than the recursive approach
    crate::eval::iterative::eval_iterative(ast, ctx)
}

pub fn eval_ast_inner<'a>(
    ast: &AstExpr,
    ctx: Option<Rc<EvalContext>>,
    func_cache: &mut BTreeMap<String, Option<FunctionCacheEntry>>,
    var_cache: &mut BTreeMap<String, Real>,
) -> Result<Real, ExprError> {
    // We'll be more selective about when to increment the recursion counter
    // Only increment for function calls, logical operations, and conditionals, as these are the ones that can cause stack overflow.
    //
    // LogicalOp and Conditional nodes are tracked because:
    // 1. They may contain recursive function calls in their operands/branches
    // 2. Short-circuit evaluation can bypass potential infinite recursion in the non-executed branches
    // 3. Complex logical or conditional expressions might cause stack overflows even with short-circuit behavior
    let should_track = matches!(ast, AstExpr::Function { .. } | AstExpr::LogicalOp { .. } | AstExpr::Conditional { .. });

    // Check and increment recursion depth only for function calls and logical operations.
    // This prevents stack overflows from infinite recursion while still allowing
    // legitimate use of recursive functions and complex logical expressions.
    if should_track {
        check_and_increment_recursion_depth()?;
    }

    // Store result to ensure we always decrement the counter if needed
    let result = match ast {
        AstExpr::Constant(val) => Ok(*val),
        AstExpr::Variable(name) => eval_variable(name, ctx.clone(), var_cache),
        AstExpr::Function { name, args } => {
            eval_function(name, args, ctx.clone(), func_cache, var_cache)
        }
        AstExpr::Array { name, index } => {
            eval_array(name, index, ctx.clone(), func_cache, var_cache)
        }
        AstExpr::Attribute { base, attr } => eval_attribute(base, attr, ctx),
        AstExpr::LogicalOp { op, left, right } => {
            // Implement short-circuit evaluation for logical operators
            // Short-circuit evaluation provides two key benefits:
            // 1. Performance - avoid unnecessary computation
            // 2. Error prevention - potentially skip evaluating expressions that would cause errors
            match op {
                crate::types::LogicalOperator::And => {
                    // Evaluate left side first
                    let left_val = eval_ast_inner(left, ctx.clone(), func_cache, var_cache)?;

                    // Short-circuit if left is false (0.0)
                    // This is important for both performance and error prevention:
                    // - If left is false, the overall result must be false regardless of right
                    // - Any potential errors in the right operand are completely avoided
                    if left_val == 0.0 {
                        Ok(0.0)
                    } else {
                        // Only evaluate right side if left is true (non-zero)
                        // Note: Errors in the right side are still propagated upward
                        let right_val = eval_ast_inner(right, ctx, func_cache, var_cache)?;
                        // Result is true (1.0) only if both are true (non-zero)
                        // We normalize the result to 1.0 for consistency
                        Ok(if right_val != 0.0 { 1.0 } else { 0.0 })
                    }
                }
                crate::types::LogicalOperator::Or => {
                    // Evaluate left side first
                    let left_val = eval_ast_inner(left, ctx.clone(), func_cache, var_cache)?;

                    // Short-circuit if left is true (non-zero)
                    // This is important for both performance and error prevention:
                    // - If left is true, the overall result must be true regardless of right
                    // - Any potential errors in the right operand are completely avoided
                    if left_val != 0.0 {
                        Ok(1.0)
                    } else {
                        // Only evaluate right side if left is false (zero)
                        // Note: Errors in the right side are still propagated upward
                        let right_val = eval_ast_inner(right, ctx, func_cache, var_cache)?;
                        // Result is true (1.0) if either is true (non-zero)
                        // We normalize the result to 1.0 for consistency
                        Ok(if right_val != 0.0 { 1.0 } else { 0.0 })
                    }
                }
            }
        },
        AstExpr::Conditional { condition, true_branch, false_branch } => {
            // Evaluate the condition first
            let condition_val = eval_ast_inner(condition, ctx.clone(), func_cache, var_cache)?;
            
            // Short-circuit to the appropriate branch based on the condition
            // This provides both performance optimization and error prevention:
            // - Only one branch is evaluated, avoiding unnecessary computation
            // - Potential errors in the non-executed branch are completely avoided
            if condition_val != 0.0 {
                // Condition is true (non-zero), evaluate the true branch
                eval_ast_inner(true_branch, ctx, func_cache, var_cache)
            } else {
                // Condition is false (zero), evaluate the false branch
                eval_ast_inner(false_branch, ctx, func_cache, var_cache)
            }
        }
    };

    // Only decrement if we incremented
    if should_track {
        decrement_recursion_depth();
    }

    result
}

