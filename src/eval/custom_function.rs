extern crate alloc;
use crate::Real;
#[cfg(not(test))]
use crate::Vec;
use crate::context::EvalContext;
use crate::error::ExprError;
use crate::eval::*;

// Import these for the tests
// Only needed if builtins are enabled
use crate::types::AstExpr;
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
    fn compiled_ast(&self) -> Option<&AstExpr> {
        None
    }
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
    fn compiled_ast(&self) -> Option<&AstExpr> {
        Some(&self.compiled_ast)
    }
}

pub fn eval_custom_function<'a, F>(
    name: &str,
    args: &[AstExpr],
    ctx: Option<Rc<EvalContext<'a>>>,
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

    // Use precompiled AST if available, else parse body string
    let body_ast = if let Some(ast) = func.compiled_ast() {
        ast.clone()
    } else {
        let param_names_str: Vec<String> = func.params().iter().map(|c| c.to_string()).collect();
        crate::engine::parse_expression_with_reserved(&func.body_str(), Some(&param_names_str))?
    };

    // No global cache needed - we use per-call variable caches

    // We need to create a special version of eval_ast_inner that knows how to handle
    // variables in this custom function scope
    fn eval_custom_function_ast<'b>(
        ast: &AstExpr,
        func_ctx: &EvalContext<'b>,
        global_ctx: Option<&Rc<EvalContext<'b>>>,
        func_cache: &mut BTreeMap<String, Option<FunctionCacheEntry>>,
        var_cache: &mut BTreeMap<String, Real>,
    ) -> Result<Real, ExprError> {
        // Track recursion depth for function calls and logical operations
        // Track recursion depth for function calls, logical operations, and conditionals.
        // We include LogicalOp and Conditional in recursion tracking for two main reasons:
        // 1. They can contain recursive function calls in operands/branches
        // 2. Short-circuit evaluation might involve complex logic that could lead to stack overflows
        let should_track = matches!(ast, AstExpr::Function { .. } | AstExpr::LogicalOp { .. } | AstExpr::Conditional { .. });

        // Check and increment recursion depth only if needed.
        // This protects against infinite recursion in expression functions and
        // complex logical operations that might cause stack overflows.
        if should_track {
            check_and_increment_recursion_depth()?;
        }

        // Store result to ensure we always decrement the counter if needed
        let result = match ast {
            AstExpr::Constant(val) => Ok(*val),
            AstExpr::Conditional { condition, true_branch, false_branch } => {
                // Evaluate the condition first
                let condition_val = eval_custom_function_ast(condition, func_ctx, global_ctx, func_cache, var_cache)?;
                
                // Short-circuit to the appropriate branch based on the condition
                if condition_val != 0.0 {
                    // Condition is true (non-zero), evaluate the true branch
                    eval_custom_function_ast(true_branch, func_ctx, global_ctx, func_cache, var_cache)
                } else {
                    // Condition is false (zero), evaluate the false branch
                    eval_custom_function_ast(false_branch, func_ctx, global_ctx, func_cache, var_cache)
                }
            },
            AstExpr::Variable(name) => {
                // First check if the variable is a parameter in the function context
                if let Some(val) = func_ctx.variables.get(name) {
                    Ok(*val)
                } else {
                    // If not found, delegate to normal variable lookup in global context
                    if let Some(ctx) = global_ctx {
                        eval_variable(name, Some(ctx.clone()), var_cache)
                    } else {
                        // No contexts available
                        Err(ExprError::UnknownVariable {
                            name: name.to_string(),
                        })
                    }
                }
            }
            AstExpr::Function { name, args } => {
                // Create a vector to store evaluated arguments
                let mut arg_values = Vec::with_capacity(args.len());

                // Evaluate each argument in the proper context
                for arg in args {
                    // Recursively call our custom evaluator
                    let arg_val =
                        eval_custom_function_ast(arg, func_ctx, global_ctx, func_cache, var_cache)?;
                    arg_values.push(arg_val);
                }

                // Once we have all argument values, find and evaluate the function
                if let Some(native_fn) = func_ctx.get_native_function(name) {
                    // Native function
                    if arg_values.len() != native_fn.arity {
                        Err(ExprError::InvalidFunctionCall {
                            name: name.to_string(),
                            expected: native_fn.arity,
                            found: arg_values.len(),
                        })
                    } else {
                        // Convert to OwnedNativeFunction
                        let owned_fn = OwnedNativeFunction::from(native_fn);
                        Ok((owned_fn.implementation)(&arg_values))
                    }
                } else if let Some(expr_fn) = func_ctx.get_expression_function(name) {
                    // Expression function (recursive case)
                    // This is the critical case for tracking recursion depth
                    if arg_values.len() != expr_fn.params.len() {
                        Err(ExprError::InvalidFunctionCall {
                            name: name.to_string(),
                            expected: expr_fn.params.len(),
                            found: arg_values.len(),
                        })
                    } else {
                        // Create a fresh context for this function call
                        let mut nested_func_ctx = EvalContext::new();

                        // Add parameters
                        for (i, param_name) in expr_fn.params.iter().enumerate() {
                            nested_func_ctx.set_parameter(param_name, arg_values[i]);
                        }

                        // Copy function registry
                        if let Some(parent) = global_ctx {
                            nested_func_ctx.function_registry = parent.function_registry.clone();
                        }

                        // Evaluate directly using the AST without going through eval_expression_function
                        eval_custom_function_ast(
                            &expr_fn.compiled_ast,
                            &nested_func_ctx,
                            global_ctx,
                            func_cache,
                            var_cache,
                        )
                    }
                } else {
                    // Not found in the contexts, try built-in functions
                    // Just delegate to eval_function in the global context
                    let ast_args: Vec<AstExpr> = args
                        .iter()
                        .zip(arg_values.iter())
                        .map(|(_, val)| AstExpr::Constant(*val))
                        .collect();

                    eval_function(name, &ast_args, global_ctx.cloned(), func_cache, var_cache)
                }
            }
            AstExpr::Array { name, index } => {
                let idx_val =
                    eval_custom_function_ast(index, func_ctx, global_ctx, func_cache, var_cache)?
                        as usize;

                // First check function context
                if let Some(arr) = func_ctx.get_array(name) {
                    if idx_val < arr.len() {
                        Ok(arr[idx_val])
                    } else {
                        Err(ExprError::ArrayIndexOutOfBounds {
                            name: name.to_string(),
                            index: idx_val,
                            len: arr.len(),
                        })
                    }
                } else if let Some(global) = global_ctx {
                    // Then check global context
                    if let Some(arr) = global.get_array(name) {
                        if idx_val < arr.len() {
                            Ok(arr[idx_val])
                        } else {
                            Err(ExprError::ArrayIndexOutOfBounds {
                                name: name.to_string(),
                                index: idx_val,
                                len: arr.len(),
                            })
                        }
                    } else {
                        Err(ExprError::UnknownVariable {
                            name: name.to_string(),
                        })
                    }
                } else {
                    Err(ExprError::UnknownVariable {
                        name: name.to_string(),
                    })
                }
            }
            AstExpr::Attribute { base, attr } => {
                // Delegate to eval_attribute
                if let Some(global) = global_ctx {
                    eval_attribute(base, attr, Some(global.clone()))
                } else {
                    Err(ExprError::AttributeNotFound {
                        base: base.to_string(),
                        attr: attr.to_string(),
                    })
                }
            }
            AstExpr::LogicalOp { op, left, right } => {
                // Implement short-circuit evaluation for logical operators in custom function context
                // This is particularly important when logical operators are used inside user-defined functions,
                // as they may involve recursive calls that need to be properly tracked
                match op {
                    crate::types::LogicalOperator::And => {
                        // Evaluate left side first
                        let left_val = eval_custom_function_ast(
                            left, func_ctx, global_ctx, func_cache, var_cache,
                        )?;

                        // Short-circuit if left is false (0.0)
                        // This helps prevent unnecessary recursion and potential stack overflow
                        if left_val == 0.0 {
                            Ok(0.0)
                        } else {
                            // Only evaluate right side if left is true (non-zero)
                            let right_val = eval_custom_function_ast(
                                right, func_ctx, global_ctx, func_cache, var_cache,
                            )?;
                            // Result is true (1.0) only if both are true (non-zero)
                            // We normalize the result to 1.0 for consistency
                            Ok(if right_val != 0.0 { 1.0 } else { 0.0 })
                        }
                    }
                    crate::types::LogicalOperator::Or => {
                        // Evaluate left side first
                        let left_val = eval_custom_function_ast(
                            left, func_ctx, global_ctx, func_cache, var_cache,
                        )?;

                        // Short-circuit if left is true (non-zero)
                        // This helps prevent unnecessary recursion and potential stack overflow
                        if left_val != 0.0 {
                            Ok(1.0)
                        } else {
                            // Only evaluate right side if left is false (zero)
                            let right_val = eval_custom_function_ast(
                                right, func_ctx, global_ctx, func_cache, var_cache,
                            )?;
                            // Result is true (1.0) if either is true (non-zero)
                            // We normalize the result to 1.0 for consistency
                            Ok(if right_val != 0.0 { 1.0 } else { 0.0 })
                        }
                    }
                }
            }
        };

        // Only decrement if we incremented
        if should_track {
            decrement_recursion_depth();
        }

        result
    }

    // Helper function to evaluate expression functions using our custom evaluator
    fn eval_expression_function<'b>(
        name: &str,
        arg_values: &[Real],
        expr_fn: &crate::types::ExpressionFunction,
        global_ctx: Option<Rc<EvalContext<'b>>>,
        func_cache: &mut BTreeMap<String, Option<FunctionCacheEntry>>,
        var_cache: &mut BTreeMap<String, Real>,
    ) -> Result<Real, ExprError> {
        // We don't need to check recursion depth here anymore since it's checked in eval_ast_inner
        // for every AST node, including the ones in the function body

        if arg_values.len() != expr_fn.params.len() {
            return Err(ExprError::InvalidFunctionCall {
                name: name.to_string(),
                expected: expr_fn.params.len(),
                found: arg_values.len(),
            });
        }

        // Create a fresh context for this function call
        let mut nested_func_ctx = EvalContext::new();

        // Add parameters
        for (i, param_name) in expr_fn.params.iter().enumerate() {
            nested_func_ctx.set_parameter(param_name, arg_values[i]);
        }

        // Copy function registry
        if let Some(parent) = &global_ctx {
            nested_func_ctx.function_registry = parent.function_registry.clone();
        }

        // Evaluate the expression with our custom evaluator
        eval_custom_function_ast(
            &expr_fn.compiled_ast,
            &nested_func_ctx,
            global_ctx.as_ref(),
            func_cache,
            var_cache,
        )
    }

    // Debug output for the polynomial function
    #[cfg(test)]
    if name == "polynomial" && arg_values.len() == 1 {
        let x = arg_values[0];

        // Let's manually calculate the expected result
        let x_cubed = x * x * x;
        let two_x_squared = 2.0 * x * x;
        let three_x = 3.0 * x;
        let expected = x_cubed + two_x_squared + three_x + 4.0;

        // In test mode we can use eprintln
        #[cfg(test)]
        {
            eprintln!("Polynomial calculation breakdown for x={}:", x);
            eprintln!("  x^3 = {}", x_cubed);
            eprintln!("  2*x^2 = {}", two_x_squared);
            eprintln!("  3*x = {}", three_x);
            eprintln!("  4 = 4");
            eprintln!("  Total expected: {}", expected);

            // Print the function body string
            eprintln!("Function body string: {}", func.body_str());

            // Print the AST structure
            eprintln!("AST structure for polynomial body:");
            match &body_ast {
                AstExpr::Function { name, args } => {
                    eprintln!("Top-level function: {}", name);
                    for (i, arg) in args.iter().enumerate() {
                        eprintln!("  Arg {}: {:?}", i, arg);

                        // If this is a function, go deeper
                        if let AstExpr::Function {
                            name: inner_name,
                            args: inner_args,
                        } = arg
                        {
                            eprintln!("    Inner function: {}", inner_name);
                            for (j, inner_arg) in inner_args.iter().enumerate() {
                                eprintln!("      Inner arg {}: {:?}", j, inner_arg);
                            }
                        }
                    }
                }
                _ => eprintln!("Not a function at top level: {:?}", body_ast),
            }

            // Check if x is correctly set in the function context
            if let Some(x_val) = func_ctx.variables.get("x") {
                eprintln!("Value of 'x' in function context: {}", x_val);
            } else {
                eprintln!("ERROR: 'x' not found in function context!");
            }
        }

        // Try directly evaluating parts of the expression with our custom evaluator
        let x_var = AstExpr::Variable("x".to_string());
        let result_x = eval_custom_function_ast(
            &x_var,
            &func_ctx,
            ctx.as_ref(),
            func_cache,
            &mut BTreeMap::new(),
        );

        #[cfg(test)]
        eprintln!("Custom evaluating 'x': {:?}", result_x);

        // Try evaluating x^3 with our custom evaluator
        let x_cubed_ast = AstExpr::Function {
            name: "^".to_string(),
            args: alloc::vec![x_var.clone(), AstExpr::Constant(3.0)],
        };
        let result_x_cubed = eval_custom_function_ast(
            &x_cubed_ast,
            &func_ctx,
            ctx.as_ref(),
            func_cache,
            &mut BTreeMap::new(),
        );

        #[cfg(test)]
        eprintln!("Custom evaluating 'x^3': {:?}", result_x_cubed);
    }

    // Now evaluate the function body with our custom evaluator
    let result = eval_custom_function_ast(
        &body_ast,
        &func_ctx,
        ctx.as_ref(),
        func_cache,
        &mut BTreeMap::new(),
    );

    // Debug the polynomial calculation for specific values
    #[cfg(test)]
    if name == "polynomial" && arg_values.len() == 1 {
        let x = arg_values[0];
        let expected = x * x * x + 2.0 * x * x + 3.0 * x + 4.0;
        eprintln!(
            "polynomial({}) = {} (expected {})",
            x,
            result.as_ref().unwrap_or(&0.0),
            expected
        );
    }

    result
}
