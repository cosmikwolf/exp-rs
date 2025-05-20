extern crate alloc;
use crate::Real;
#[cfg(not(test))]
use crate::Vec;
use crate::context::EvalContext;
use crate::error::ExprError;

// Import these for the tests
#[cfg(test)]
use crate::abs;
#[cfg(test)]
use crate::cos;
#[cfg(test)]
use crate::max;
#[cfg(test)]
use crate::min;
#[cfg(test)]
use crate::neg;
#[cfg(test)]
use crate::pow;
#[cfg(test)]
use crate::sin;
// Only needed if builtins are enabled
use crate::types::AstExpr;
#[cfg(not(test))]
use alloc::format;
#[cfg(not(test))]
use alloc::rc::Rc;
#[cfg(test)]
use std::rc::Rc;
#[cfg(test)]
use std::vec::Vec;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};

use super::*;

pub fn eval_variable(
    name: &str,
    ctx: Option<Rc<EvalContext<'_>>>,
    var_cache: &mut BTreeMap<String, Real>,
) -> Result<Real, ExprError> {
    // Handle built-in constants first (pi, e) - these don't need context
    if name == "pi" {
        #[cfg(feature = "f32")]
        return Ok(core::f32::consts::PI);
        #[cfg(not(feature = "f32"))]
        return Ok(core::f64::consts::PI);
    } else if name == "e" {
        #[cfg(feature = "f32")]
        return Ok(core::f32::consts::E);
        #[cfg(not(feature = "f32"))]
        return Ok(core::f64::consts::E);
    }

    // Check local cache first
    if let Some(val) = var_cache.get(name).copied() {
        return Ok(val);
    }

    // Use context helper methods for variable/constant lookup
    if let Some(ctx_ref) = ctx.as_deref() {
        // For expression functions, we need to check the immediate variables first
        // before checking parent contexts to avoid shadowing issues
        if let Some(val) = ctx_ref.variables.get(name) {
            // Found in immediate context, return it
            var_cache.insert(name.to_string(), *val);
            return Ok(*val);
        }

        // Then check constants in the immediate context
        if let Some(val) = ctx_ref.constants.get(name) {
            var_cache.insert(name.to_string(), *val);
            return Ok(*val);
        }

        // Now check the parent chain using the helper methods
        // which will properly handle the parent chain
        if let Some(val) = ctx_ref.get_variable(name) {
            var_cache.insert(name.to_string(), val);
            return Ok(val);
        } else if let Some(val) = ctx_ref.get_constant(name) {
            var_cache.insert(name.to_string(), val);
            return Ok(val);
        }
    }

    // If not a constant and not found in context, check if it looks like a function name
    let is_potential_function_name = match name {
        "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "atan2" | "sinh" | "cosh" | "tanh"
        | "exp" | "log" | "log10" | "ln" | "sqrt" | "abs" | "ceil" | "floor" | "pow" | "neg"
        | "," | "comma" | "+" | "-" | "*" | "/" | "%" | "^" | "max" | "min" => true,
        _ => false,
    };

    if is_potential_function_name && name.len() > 1 {
        return Err(ExprError::Syntax(format!(
            "Function '{}' used without arguments",
            name
        )));
    }

    Err(ExprError::UnknownVariable {
        name: name.to_string(),
    })
}

pub fn eval_function<'a>(
    name: &str,
    args: &[AstExpr],
    ctx: Option<Rc<EvalContext<'a>>>,
    func_cache: &mut BTreeMap<String, Option<FunctionCacheEntry>>,
    var_cache: &mut BTreeMap<String, Real>,
) -> Result<Real, ExprError> {
    // Try to get cached function info first
    if let Some(entry) = func_cache.get(name) {
        if let Some(cached_fn) = entry {
            return match cached_fn.clone() {
                FunctionCacheEntry::User(user_fn) => {
                    eval_custom_function(name, args, ctx.clone(), func_cache, var_cache, &user_fn)
                }
                FunctionCacheEntry::Expression(expr_fn) => {
                    eval_custom_function(name, args, ctx.clone(), func_cache, var_cache, &expr_fn)
                }
                FunctionCacheEntry::Native(native_fn) => {
                    eval_native_function(name, args, ctx.clone(), func_cache, var_cache, &native_fn)
                }
            };
        }
    }

    // If not in cache, look up in context and create cache entry
    let entry = if let Some(ctx_ref) = ctx.as_ref() {
        // Create fully owned entries from the context's data
        if let Some(expr_fn) = ctx_ref.get_expression_function(name) {
            // ExpressionFunction is already Clone so we can use it directly
            Some(FunctionCacheEntry::Expression(expr_fn.clone()))
        } else if let Some(native_fn) = ctx_ref.get_native_function(name) {
            // Convert to our owned version
            let owned_fn = OwnedNativeFunction::from(native_fn);
            Some(FunctionCacheEntry::Native(owned_fn))
        } else if let Some(user_fn) = ctx_ref.get_user_function(name) {
            // UserFunction is already Clone so we can use it directly
            Some(FunctionCacheEntry::User(user_fn.clone()))
        } else {
            None
        }
    } else {
        None
    };

    // Cache the entry without borrowing ctx
    func_cache.insert(name.to_string(), entry.clone());

    // Process the entry
    if let Some(func_entry) = entry {
        match func_entry {
            FunctionCacheEntry::User(user_fn) => {
                eval_custom_function(name, args, ctx.clone(), func_cache, var_cache, &user_fn)
            }
            FunctionCacheEntry::Expression(expr_fn) => {
                eval_custom_function(name, args, ctx.clone(), func_cache, var_cache, &expr_fn)
            }
            FunctionCacheEntry::Native(native_fn) => {
                eval_native_function(name, args, ctx.clone(), func_cache, var_cache, &native_fn)
            }
        }
    } else {
        // Rest of the function remains the same...
        // Fallback to built-in functions...
        #[cfg(feature = "libm")]
        {
            // First check known functions against their expected arity
            // This allows us to return InvalidFunctionCall errors rather than UnknownFunction
            let single_arg_funcs = [
                "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "exp", "log",
                "ln", "log10", "sqrt", "abs", "ceil", "floor", "neg",
            ];

            let two_arg_funcs = [
                "+", "-", "*", "/", "^", "pow", "max", "min", "%", ",", "comma", "atan2",
            ];

            // Check single-arg functions with wrong arity
            if single_arg_funcs.contains(&name) && args.len() != 1 {
                return Err(ExprError::InvalidFunctionCall {
                    name: name.to_string(),
                    expected: 1,
                    found: args.len(),
                });
            }

            // Check two-arg functions with wrong arity
            if two_arg_funcs.contains(&name) && args.len() != 2 {
                return Err(ExprError::InvalidFunctionCall {
                    name: name.to_string(),
                    expected: 2,
                    found: args.len(),
                });
            }

            // Now evaluate with correct arity
            if args.len() == 1 {
                // Single-arg built-in functions
                let arg_val = eval_ast_inner(&args[0], ctx.clone(), func_cache, var_cache)?;
                match name {
                    "sin" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::sinf(arg_val));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::sin(arg_val));
                        }
                    }
                    "cos" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::cosf(arg_val));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::cos(arg_val));
                        }
                    }
                    "tan" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::tanf(arg_val));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::tan(arg_val));
                        }
                    }
                    "asin" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::asinf(arg_val));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::asin(arg_val));
                        }
                    }
                    "acos" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::acosf(arg_val));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::acos(arg_val));
                        }
                    }
                    "atan" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::atanf(arg_val));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::atan(arg_val));
                        }
                    }
                    "sinh" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::sinhf(arg_val));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::sinh(arg_val));
                        }
                    }
                    "cosh" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::coshf(arg_val));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::cosh(arg_val));
                        }
                    }
                    "tanh" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::tanhf(arg_val));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::tanh(arg_val));
                        }
                    }
                    "exp" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::expf(arg_val));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::exp(arg_val));
                        }
                    }
                    "log" | "ln" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::logf(arg_val));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::log(arg_val));
                        }
                    }
                    "log10" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::log10f(arg_val));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::log10(arg_val));
                        }
                    }
                    "sqrt" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::sqrtf(arg_val));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::sqrt(arg_val));
                        }
                    }
                    "abs" => return Ok(arg_val.abs()),
                    "ceil" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::ceilf(arg_val));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::ceil(arg_val));
                        }
                    }
                    "floor" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::floorf(arg_val));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::floor(arg_val));
                        }
                    }
                    "neg" => return Ok(-arg_val),
                    _ => {}
                }
            } else if args.len() == 2 {
                // Two-arg built-in functions
                let mut arg_vals = [0.0; 2];
                arg_vals[0] = eval_ast_inner(&args[0], ctx.clone(), func_cache, var_cache)?;
                arg_vals[1] = eval_ast_inner(&args[1], ctx.clone(), func_cache, var_cache)?;
                match name {
                    // Basic arithmetic operators
                    "+" => return Ok(arg_vals[0] + arg_vals[1]),
                    "-" => return Ok(arg_vals[0] - arg_vals[1]),
                    "*" => return Ok(arg_vals[0] * arg_vals[1]),
                    "/" => return Ok(arg_vals[0] / arg_vals[1]),
                    "^" | "pow" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::powf(arg_vals[0], arg_vals[1]));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::pow(arg_vals[0], arg_vals[1]));
                        }
                    }
                    "max" => return Ok(arg_vals[0].max(arg_vals[1])),
                    "min" => return Ok(arg_vals[0].min(arg_vals[1])),
                    "%" => return Ok(arg_vals[0] % arg_vals[1]),
                    "," | "comma" => return Ok(arg_vals[1]),

                    // Comparison operators - return 1.0 for true and 0.0 for false
                    "<" => return Ok(if arg_vals[0] < arg_vals[1] { 1.0 } else { 0.0 }),
                    ">" => return Ok(if arg_vals[0] > arg_vals[1] { 1.0 } else { 0.0 }),
                    "<=" => return Ok(if arg_vals[0] <= arg_vals[1] { 1.0 } else { 0.0 }),
                    ">=" => return Ok(if arg_vals[0] >= arg_vals[1] { 1.0 } else { 0.0 }),
                    "==" => return Ok(if arg_vals[0] == arg_vals[1] { 1.0 } else { 0.0 }),
                    "!=" | "<>" => return Ok(if arg_vals[0] != arg_vals[1] { 1.0 } else { 0.0 }),

                    "atan2" => {
                        #[cfg(feature = "f32")]
                        {
                            return Ok(libm::atan2f(arg_vals[0], arg_vals[1]));
                        }
                        #[cfg(not(feature = "f32"))]
                        {
                            return Ok(libm::atan2(arg_vals[0], arg_vals[1]));
                        }
                    }
                    _ => {}
                }
            }
        }

        // If we get here, the function is unknown
        return Err(ExprError::UnknownFunction {
            name: name.to_string(),
        });
    }
}

pub fn eval_native_function<'a>(
    name: &str,
    args: &[AstExpr],
    ctx: Option<Rc<EvalContext<'a>>>,
    func_cache: &mut BTreeMap<String, Option<FunctionCacheEntry>>,
    var_cache: &mut BTreeMap<String, Real>,
    native_fn: &OwnedNativeFunction,
) -> Result<Real, ExprError> {
    if args.len() != native_fn.arity {
        return Err(ExprError::InvalidFunctionCall {
            name: name.to_string(),
            expected: native_fn.arity,
            found: args.len(),
        });
    }
    let mut arg_values = Vec::with_capacity(args.len());
    for arg in args.iter() {
        let arg_val = eval_ast_inner(arg, ctx.clone(), func_cache, var_cache)?;
        arg_values.push(arg_val);
    }
    Ok((native_fn.implementation)(&arg_values))
}

pub fn eval_array<'a>(
    name: &str,
    index: &AstExpr,
    ctx: Option<Rc<EvalContext<'a>>>,
    func_cache: &mut BTreeMap<String, Option<FunctionCacheEntry>>,
    var_cache: &mut BTreeMap<String, Real>,
) -> Result<Real, ExprError> {
    let idx = eval_ast_inner(index, ctx.clone(), func_cache, var_cache)? as usize;

    if let Some(ctx_ref) = ctx.as_ref() {
        if let Some(arr) = ctx_ref.get_array(name) {
            if idx < arr.len() {
                return Ok(arr[idx]);
            } else {
                return Err(ExprError::ArrayIndexOutOfBounds {
                    name: name.to_string(),
                    index: idx,
                    len: arr.len(),
                });
            }
        }
    }
    Err(ExprError::UnknownVariable {
        name: name.to_string(),
    })
}

pub fn eval_attribute(
    base: &str,
    attr: &str,
    ctx: Option<Rc<EvalContext<'_>>>,
) -> Result<Real, ExprError> {
    if let Some(ctx_ref) = ctx.as_ref() {
        if let Some(attr_map) = ctx_ref.get_attribute_map(base) {
            if let Some(val) = attr_map.get(attr) {
                return Ok(*val);
            } else {
                return Err(ExprError::AttributeNotFound {
                    base: base.to_string(),
                    attr: attr.to_string(),
                });
            }
        }
    }
    Err(ExprError::AttributeNotFound {
        base: base.to_string(),
        attr: attr.to_string(),
    })
}
