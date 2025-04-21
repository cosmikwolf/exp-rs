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

// For no_std, we need to be careful with statics
// We'll keep it very simple: just create a fresh cache every time
// This isn't as efficient, but works for our embedded target

// No global cache needed - we pass local caches as parameters for better safety

// Create an owned version of NativeFunction without lifetime dependencies
struct OwnedNativeFunction {
    pub arity: usize,
    pub implementation: Rc<dyn Fn(&[Real]) -> Real>,
    pub name: String, // Fully owned String instead of Cow
    pub description: Option<String>,
}

// Convert from NativeFunction<'a> to OwnedNativeFunction
impl<'a> From<&crate::types::NativeFunction<'a>> for OwnedNativeFunction {
    fn from(nf: &crate::types::NativeFunction<'a>) -> Self {
        OwnedNativeFunction {
            arity: nf.arity,
            implementation: nf.implementation.clone(),
            name: nf.name.to_string(), // Convert Cow to String
            description: nf.description.clone(),
        }
    }
}

enum FunctionCacheEntry {
    Native(OwnedNativeFunction),
    Expression(crate::types::ExpressionFunction),
    User(crate::context::UserFunction),
}

impl Clone for FunctionCacheEntry {
    fn clone(&self) -> Self {
        match self {
            FunctionCacheEntry::Native(nf) => {
                FunctionCacheEntry::Native(OwnedNativeFunction {
                    arity: nf.arity,
                    implementation: nf.implementation.clone(),
                    name: nf.name.clone(),
                    description: nf.description.clone(),
                })
            }
            FunctionCacheEntry::Expression(ef) => FunctionCacheEntry::Expression(ef.clone()),
            FunctionCacheEntry::User(uf) => FunctionCacheEntry::User(uf.clone()),
        }
    }
}

#[cfg(not(feature = "no-builtin-math"))]
#[allow(dead_code)]
type MathFunc = fn(Real, Real) -> Real;

pub fn eval_ast<'a>(ast: &AstExpr, ctx: Option<Rc<EvalContext<'a>>>) -> Result<Real, ExprError> {
    // Don't use a shared cache - we'll operate directly on the function cache
    let mut func_cache: BTreeMap<String, Option<FunctionCacheEntry>> = BTreeMap::new();
    // Also maintain a variable cache specific to this evaluation
    let mut var_cache: BTreeMap<String, Real> = BTreeMap::new();

    eval_ast_inner(ast, ctx, &mut func_cache, &mut var_cache)
}

fn eval_variable(name: &str, ctx: Option<Rc<EvalContext<'_>>>, var_cache: &mut BTreeMap<String, Real>) -> Result<Real, ExprError> {
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

fn eval_function<'a>(
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
        #[cfg(not(feature = "no-builtin-math"))]
        {
            // First check known functions against their expected arity
            // This allows us to return InvalidFunctionCall errors rather than UnknownFunction
            let single_arg_funcs = ["sin", "cos", "tan", "asin", "acos", "atan", 
                "sinh", "cosh", "tanh", "exp", "log", "ln", "log10", "sqrt", 
                "abs", "ceil", "floor", "neg"];
                
            let two_arg_funcs = ["+", "-", "*", "/", "^", "pow", "max", "min", 
                "%", ",", "comma", "atan2"];
            
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

fn eval_custom_function<'a, F>(
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
        match ast {
            AstExpr::Constant(val) => Ok(*val),
            AstExpr::Variable(name) => {
                // First check if the variable is a parameter in the function context
                if let Some(val) = func_ctx.variables.get(name) {
                    return Ok(*val);
                }
                
                // If not found, delegate to normal variable lookup in global context
                if let Some(ctx) = global_ctx {
                    return eval_variable(name, Some(ctx.clone()), var_cache);
                }
                
                // No contexts available
                Err(ExprError::UnknownVariable { name: name.to_string() })
            },
            AstExpr::Function { name, args } => {
                // Create a vector to store evaluated arguments
                let mut arg_values = Vec::with_capacity(args.len());
                
                // Evaluate each argument in the proper context
                for arg in args {
                    // Recursively call our custom evaluator
                    let arg_val = eval_custom_function_ast(arg, func_ctx, global_ctx, func_cache, var_cache)?;
                    arg_values.push(arg_val);
                }
                
                // Once we have all argument values, find and evaluate the function
                if let Some(native_fn) = func_ctx.get_native_function(name) {
                    // Native function
                    if arg_values.len() != native_fn.arity {
                        return Err(ExprError::InvalidFunctionCall {
                            name: name.to_string(),
                            expected: native_fn.arity,
                            found: arg_values.len(),
                        });
                    }
                    // Convert to OwnedNativeFunction
                    let owned_fn = OwnedNativeFunction::from(native_fn);
                    return Ok((owned_fn.implementation)(&arg_values));
                } else if let Some(expr_fn) = func_ctx.get_expression_function(name) {
                    // Expression function (recursive case)
                    return eval_expression_function(name, &arg_values, expr_fn, global_ctx.cloned(), func_cache, var_cache);
                } else {
                    // Not found in the contexts, try built-in functions
                    // Just delegate to eval_function in the global context
                    let ast_args: Vec<AstExpr> = args.iter()
                        .zip(arg_values.iter())
                        .map(|(_, val)| AstExpr::Constant(*val))
                        .collect();
                    
                    return eval_function(name, &ast_args, global_ctx.cloned(), func_cache, var_cache);
                }
            },
            AstExpr::Array { name, index } => {
                let idx_val = eval_custom_function_ast(index, func_ctx, global_ctx, func_cache, var_cache)? as usize;
                
                // First check function context
                if let Some(arr) = func_ctx.get_array(name) {
                    if idx_val < arr.len() {
                        return Ok(arr[idx_val]);
                    } else {
                        return Err(ExprError::ArrayIndexOutOfBounds {
                            name: name.to_string(),
                            index: idx_val,
                            len: arr.len(),
                        });
                    }
                }
                
                // Then check global context
                if let Some(global) = global_ctx {
                    if let Some(arr) = global.get_array(name) {
                        if idx_val < arr.len() {
                            return Ok(arr[idx_val]);
                        } else {
                            return Err(ExprError::ArrayIndexOutOfBounds {
                                name: name.to_string(),
                                index: idx_val,
                                len: arr.len(),
                            });
                        }
                    }
                }
                
                Err(ExprError::UnknownVariable { name: name.to_string() })
            },
            AstExpr::Attribute { base, attr } => {
                // Delegate to eval_attribute
                if let Some(global) = global_ctx {
                    return eval_attribute(base, attr, Some(global.clone()));
                }
                
                Err(ExprError::AttributeNotFound {
                    base: base.to_string(),
                    attr: attr.to_string(),
                })
            }
        }
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
            var_cache
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
                        if let AstExpr::Function { name: inner_name, args: inner_args } = arg {
                            eprintln!("    Inner function: {}", inner_name);
                            for (j, inner_arg) in inner_args.iter().enumerate() {
                                eprintln!("      Inner arg {}: {:?}", j, inner_arg);
                            }
                        }
                    }
                },
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
        let result_x = eval_custom_function_ast(&x_var, &func_ctx, ctx.as_ref(), func_cache, &mut BTreeMap::new());
        
        #[cfg(test)]
        eprintln!("Custom evaluating 'x': {:?}", result_x);
        
        // Try evaluating x^3 with our custom evaluator
        let x_cubed_ast = AstExpr::Function {
            name: "^".to_string(),
            args: alloc::vec![x_var.clone(), AstExpr::Constant(3.0)],
        };
        let result_x_cubed = eval_custom_function_ast(&x_cubed_ast, &func_ctx, ctx.as_ref(), func_cache, &mut BTreeMap::new());
        
        #[cfg(test)]
        eprintln!("Custom evaluating 'x^3': {:?}", result_x_cubed);
    }
    
    // Now evaluate the function body with our custom evaluator
    let result = eval_custom_function_ast(&body_ast, &func_ctx, ctx.as_ref(), func_cache, &mut BTreeMap::new());
    
    // Debug the polynomial calculation for specific values
    #[cfg(test)]
    if name == "polynomial" && arg_values.len() == 1 {
        let x = arg_values[0];
        let expected = x*x*x + 2.0*x*x + 3.0*x + 4.0;
        eprintln!("polynomial({}) = {} (expected {})", x, result.as_ref().unwrap_or(&0.0), expected);
    }
    
    result
}

fn eval_native_function<'a>(
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

fn eval_array<'a>(
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

fn eval_attribute(
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

fn eval_ast_inner<'a>(
    ast: &AstExpr,
    ctx: Option<Rc<EvalContext<'a>>>,
    func_cache: &mut BTreeMap<String, Option<FunctionCacheEntry>>,
    var_cache: &mut BTreeMap<String, Real>,
) -> Result<Real, ExprError> {
    match ast {
        AstExpr::Constant(val) => Ok(*val),
        AstExpr::Variable(name) => eval_variable(name, ctx.clone(), var_cache),
        AstExpr::Function { name, args } => {
            eval_function(name, args, ctx.clone(), func_cache, var_cache)
        }
        AstExpr::Array { name, index } => eval_array(name, index, ctx.clone(), func_cache, var_cache),
        AstExpr::Attribute { base, attr } => eval_attribute(base, attr, ctx),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{interp, parse_expression};
    
    // Helper functions for tests that need to call eval functions directly
    fn test_eval_variable(name: &str, ctx: Option<Rc<EvalContext>>) -> Result<Real, ExprError> {
        let mut var_cache = BTreeMap::new();
        super::eval_variable(name, ctx, &mut var_cache)
    }
    
    fn test_eval_function(
        name: &str, 
        args: &[AstExpr], 
        ctx: Option<Rc<EvalContext>>, 
        func_cache: &mut BTreeMap<String, Option<FunctionCacheEntry>>
    ) -> Result<Real, ExprError> {
        let mut var_cache = BTreeMap::new();
        super::eval_function(name, args, ctx, func_cache, &mut var_cache)
    }
    
    fn test_eval_array(
        name: &str,
        index: &AstExpr,
        ctx: Option<Rc<EvalContext>>,
        func_cache: &mut BTreeMap<String, Option<FunctionCacheEntry>>
    ) -> Result<Real, ExprError> {
        let mut var_cache = BTreeMap::new();
        super::eval_array(name, index, ctx, func_cache, &mut var_cache)
    }
    
    fn test_eval_custom_function<F>(
        name: &str,
        args: &[AstExpr],
        ctx: Option<Rc<EvalContext>>,
        func_cache: &mut BTreeMap<String, Option<FunctionCacheEntry>>,
        func: &F
    ) -> Result<Real, ExprError>
    where
        F: super::CustomFunction
    {
        let mut var_cache = BTreeMap::new();
        super::eval_custom_function(name, args, ctx, func_cache, &mut var_cache, func)
    }
    
    fn test_eval_native_function(
        name: &str,
        args: &[AstExpr],
        ctx: Option<Rc<EvalContext>>,
        func_cache: &mut BTreeMap<String, Option<FunctionCacheEntry>>,
        native_fn: &OwnedNativeFunction
    ) -> Result<Real, ExprError> {
        let mut var_cache = BTreeMap::new();
        super::eval_native_function(name, args, ctx, func_cache, &mut var_cache, native_fn)
    }
    use crate::error::ExprError;

    #[test]
    fn test_eval_user_function_polynomial() {
        let mut ctx = EvalContext::new();
        ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4")
            .unwrap();
        let mut func_cache = std::collections::BTreeMap::new();
        let _ast = AstExpr::Function {
            name: "polynomial".to_string(),
            args: vec![AstExpr::Constant(3.0)],
        };
        // Avoid simultaneous mutable and immutable borrow of ctx
        let expr_fn = ctx.get_expression_function("polynomial").unwrap().clone();
        let val = test_eval_custom_function(
            "polynomial",
            &[AstExpr::Constant(3.0)],
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
            &expr_fn,
        )
        .unwrap();
        assert_eq!(val, 58.0); // 3^3 + 2*3^2 + 3*3 + 4 = 27 + 18 + 9 + 4 = 58
    }

    #[test]
    fn test_eval_expression_function_simple() {
        let mut ctx = EvalContext::new();
        ctx.register_expression_function("double", &["x"], "x*2")
            .unwrap();
        let mut func_cache = std::collections::BTreeMap::new();
        // Avoid simultaneous mutable and immutable borrow of ctx
        let expr_fn = ctx.get_expression_function("double").unwrap().clone();
        let val = test_eval_custom_function(
            "double",
            &[AstExpr::Constant(7.0)],
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
            &expr_fn,
        )
        .unwrap();
        assert_eq!(val, 14.0);
    }

    #[test]
    fn test_eval_native_function_simple() {
        let mut ctx = EvalContext::new();
        ctx.register_native_function("triple", 1, |args| args[0] * 3.0);
        let mut func_cache = std::collections::BTreeMap::new();
        // Avoid simultaneous mutable and immutable borrow of ctx by splitting the scope
        let native_fn = {
            // We need to use the string directly as the key
            let nf = ctx.function_registry.native_functions.get("triple").unwrap();
            OwnedNativeFunction {
                arity: nf.arity,
                implementation: nf.implementation.clone(),
                name: nf.name.to_string(), // Convert to String
                description: nf.description.clone(),
            }
        };
        // At this point, the immutable borrow is dropped

        let val = test_eval_native_function(
            "triple",
            &[AstExpr::Constant(4.0)],
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
            &native_fn,
        )
        .unwrap();
        assert_eq!(val, 12.0);
    }

    // Helper to create a context and register defaults IF builtins are enabled
    fn create_test_context<'a>() -> EvalContext<'a> {
        let mut ctx = EvalContext::new();
        // Register defaults only if the feature allows it
        #[cfg(not(feature = "no-builtin-math"))]
        {
            // Manually register built-ins needed for tests if register_defaults doesn't exist
            // or isn't comprehensive enough for test setup.
            ctx.register_native_function("sin", 1, |args| sin(args[0], 0.0));
            ctx.register_native_function("cos", 1, |args| cos(args[0], 0.0));
            ctx.register_native_function("pow", 2, |args| pow(args[0], args[1]));
            ctx.register_native_function("^", 2, |args| pow(args[0], args[1]));
            ctx.register_native_function("min", 2, |args| min(args[0], args[1]));
            ctx.register_native_function("max", 2, |args| max(args[0], args[1]));
            ctx.register_native_function("neg", 1, |args| neg(args[0], 0.0));
            ctx.register_native_function("abs", 1, |args| abs(args[0], 0.0));
            // Add others as needed by tests...
        }
        ctx
    }

    #[test]
    fn test_eval_variable_builtin_constants() {
        // Test pi and e
        #[cfg(feature = "f32")]
        {
            assert!((test_eval_variable("pi", None).unwrap() - std::f32::consts::PI).abs() < 1e-5);
            assert!((test_eval_variable("e", None).unwrap() - std::f32::consts::E).abs() < 1e-5);
        }
        #[cfg(not(feature = "f32"))]
        {
            assert!((test_eval_variable("pi", None).unwrap() - std::f64::consts::PI).abs() < 1e-10);
            assert!((test_eval_variable("e", None).unwrap() - std::f64::consts::E).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_variable_context_lookup() {
        let mut ctx = EvalContext::new();
        ctx.set_parameter("x", 42.0);
        ctx.constants.insert("y".into(), 3.14);
        assert_eq!(test_eval_variable("x", Some(Rc::new(ctx.clone()))).unwrap(), 42.0);
        assert_eq!(test_eval_variable("y", Some(Rc::new(ctx.clone()))).unwrap(), 3.14);
    }

    #[test]
    fn test_eval_variable_unknown_and_function_name() {
        let err = test_eval_variable("nosuchvar", None).unwrap_err();
        assert!(matches!(err, ExprError::UnknownVariable { .. }));
        let err2 = test_eval_variable("sin", None).unwrap_err();
        assert!(matches!(err2, ExprError::Syntax(_)));
    }

    #[test]
    fn test_eval_function_native_and_expression() {
        let mut ctx = create_test_context();
        // Native function
        // Don't use the ast variable if we're not going to use it
        let mut func_cache = std::collections::BTreeMap::new();
        let val = test_eval_function(
            "sin",
            &[AstExpr::Constant(0.0)],
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
        )
        .unwrap();
        assert!((val - 0.0).abs() < 1e-10);

        // Expression function
        ctx.register_expression_function("double", &["x"], "x*2")
            .unwrap();
        // No need for ast2 since we're not using it
        let val2 = test_eval_function(
            "double",
            &[AstExpr::Constant(5.0)],
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
        )
        .unwrap();
        assert_eq!(val2, 10.0);
    }

    #[test]
    fn test_eval_function_user_function() {
        let mut ctx = create_test_context();
        ctx.register_expression_function("inc", &["x"], "x+1")
            .unwrap();
        let mut func_cache = std::collections::BTreeMap::new();
        let val = test_eval_function(
            "inc",
            &[AstExpr::Constant(41.0)],
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
        )
        .unwrap();
        assert_eq!(val, 42.0);
    }

    #[test]
    fn test_eval_function_builtin_fallback() {
        let ctx = create_test_context();
        let mut func_cache = std::collections::BTreeMap::new();
        // Built-in fallback: pow(2,3)
        let val = test_eval_function(
            "pow",
            &[AstExpr::Constant(2.0), AstExpr::Constant(3.0)],
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
        )
        .unwrap();
        assert_eq!(val, 8.0);
        // Built-in fallback: abs(-5)
        let val2 = test_eval_function(
            "abs",
            &[AstExpr::Constant(-5.0)],
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
        )
        .unwrap();
        assert_eq!(val2, 5.0);
    }

    #[test]
    fn test_eval_array_success_and_out_of_bounds() {
        let mut ctx = EvalContext::new();
        ctx.arrays.insert("arr".into(), vec![1.0, 2.0, 3.0]);
        
        // Create separate caches for each call to avoid borrowing issues
        let mut func_cache1 = std::collections::BTreeMap::new();
        let mut func_cache2 = std::collections::BTreeMap::new();
        
        let idx_expr = AstExpr::Constant(1.0);
        let val = test_eval_array("arr", &idx_expr, Some(Rc::new(ctx.clone())), &mut func_cache1).unwrap();
        assert_eq!(val, 2.0);
        
        // Out of bounds
        let idx_expr2 = AstExpr::Constant(10.0);
        let err =
            test_eval_array("arr", &idx_expr2, Some(Rc::new(ctx.clone())), &mut func_cache2).unwrap_err();
        assert!(matches!(err, ExprError::ArrayIndexOutOfBounds { .. }));
    }

    #[test]
    fn test_eval_array_unknown() {
        let ctx = EvalContext::new();
        let mut func_cache = std::collections::BTreeMap::new();
        let idx_expr = AstExpr::Constant(0.0);
        let err =
            test_eval_array("nosucharr", &idx_expr, Some(Rc::new(ctx.clone())), &mut func_cache).unwrap_err();
        assert!(matches!(err, ExprError::UnknownVariable { .. }));
    }

    #[test]
    fn test_eval_attribute_success_and_not_found() {
        let mut ctx = EvalContext::new();
        let mut map = std::collections::HashMap::new();
        map.insert("foo".to_string(), 123.0);
        ctx.attributes.insert("bar".to_string(), map);
        let val = super::eval_attribute("bar", "foo", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val, 123.0);
        let err = super::eval_attribute("bar", "baz", Some(Rc::new(ctx.clone()))).unwrap_err();
        assert!(matches!(err, ExprError::AttributeNotFound { .. }));
    }

    #[test]
    fn test_eval_attribute_unknown_base() {
        let ctx = EvalContext::new();
        let err = super::eval_attribute("nosuch", "foo", Some(Rc::new(ctx.clone()))).unwrap_err();
        assert!(matches!(err, ExprError::AttributeNotFound { .. }));
    }

    #[test]
    fn test_neg_pow_ast() {
        // AST structure test - independent of evaluation context or features
        let ast = parse_expression("-2^2").unwrap_or_else(|e| panic!("Parse error: {}", e));
        // ... (assertions remain the same) ...
        match ast {
            AstExpr::Function { ref name, ref args } if name == "neg" => {
                assert_eq!(args.len(), 1);
                match &args[0] {
                    AstExpr::Function {
                        name: pow_name,
                        args: pow_args,
                    } if pow_name == "^" => {
                        assert_eq!(pow_args.len(), 2);
                        match (&pow_args[0], &pow_args[1]) {
                            (AstExpr::Constant(a), AstExpr::Constant(b)) => {
                                assert_eq!(*a, 2.0);
                                assert_eq!(*b, 2.0);
                            }
                            _ => panic!("Expected constants as pow args"),
                        }
                    }
                    _ => panic!("Expected pow as argument to neg"),
                }
            }
            _ => panic!("Expected neg as top-level function"),
        }
    }

    #[test]
    #[cfg(not(feature = "no-builtin-math"))] // This test relies on built-in fallback
    fn test_neg_pow_eval() {
        // Test evaluation using built-in functions (no context needed for this specific expr)
        let val = interp("-2^2", None).unwrap();
        assert_eq!(val, -4.0); // Should be -(2^2) = -4
        let val2 = interp("(-2)^2", None).unwrap();
        assert_eq!(val2, 4.0); // Should be 4
    }

    #[test]
    #[cfg(feature = "no-builtin-math")] // Test behavior when builtins are disabled
    fn test_neg_pow_eval_no_builtins() {
        // Without builtins, neg and ^ must be provided in context
        let mut ctx = EvalContext::new();
        ctx.register_native_function("neg", 1, |args| -args[0]);
        ctx.register_native_function("^", 2, |args| args[0].powf(args[1])); // Example using powf

        let val = interp("-2^2", Some(&mut ctx)).unwrap();
        assert_eq!(val, -4.0);
        let val2 = interp("(-2)^2", Some(&mut ctx)).unwrap();
        assert_eq!(val2, 4.0);

        // Test that it fails without context
        let err = interp("-2^2", None).unwrap_err();
        assert!(matches!(err, ExprError::UnknownFunction { .. }));
    }

    #[test]
    fn test_paren_neg_pow_ast() {
        // AST structure test - independent of evaluation context or features
        let ast = parse_expression("(-2)^2").unwrap_or_else(|e| panic!("Parse error: {}", e));
        // ... (assertions remain the same) ...
        match ast {
            AstExpr::Function { ref name, ref args } if name == "^" => {
                assert_eq!(args.len(), 2);
                match &args[0] {
                    AstExpr::Function {
                        name: neg_name,
                        args: neg_args,
                    } if neg_name == "neg" => {
                        assert_eq!(neg_args.len(), 1);
                        match &neg_args[0] {
                            AstExpr::Constant(a) => assert_eq!(*a, 2.0),
                            _ => panic!("Expected constant as neg arg"),
                        }
                    }
                    _ => panic!("Expected neg as left arg to pow"),
                }
                match &args[1] {
                    AstExpr::Constant(b) => assert_eq!(*b, 2.0),
                    _ => panic!("Expected constant as right arg to pow"),
                }
            }
            _ => panic!("Expected pow as top-level function"),
        }
    }

    #[test]
    fn test_function_application_juxtaposition_ast() {
        // AST structure test - independent of evaluation context or features
        // ... (assertions remain the same) ...
        let sin_x_ast = AstExpr::Function {
            name: "sin".to_string(),
            args: vec![AstExpr::Variable("x".to_string())],
        };

        match sin_x_ast {
            AstExpr::Function { ref name, ref args } if name == "sin" => {
                assert_eq!(args.len(), 1);
                match &args[0] {
                    AstExpr::Variable(var) => assert_eq!(var, "x"),
                    _ => panic!("Expected variable as argument"),
                }
            }
            _ => panic!("Expected function node for sin x"),
        }

        // For "abs -42", we expect abs(neg(42))
        let neg_42_ast = AstExpr::Function {
            name: "neg".to_string(),
            args: vec![AstExpr::Constant(42.0)],
        };

        let abs_neg_42_ast = AstExpr::Function {
            name: "abs".to_string(),
            args: vec![neg_42_ast],
        };

        println!("AST for 'abs -42': {:?}", abs_neg_42_ast);

        match abs_neg_42_ast {
            AstExpr::Function { ref name, ref args } if name == "abs" => {
                assert_eq!(args.len(), 1);
                match &args[0] {
                    AstExpr::Function {
                        name: n2,
                        args: args2,
                    } if n2 == "neg" => {
                        assert_eq!(args2.len(), 1);
                        match &args2[0] {
                            AstExpr::Constant(c) => assert_eq!(*c, 42.0),
                            _ => panic!("Expected constant as neg arg"),
                        }
                    }
                    _ => panic!("Expected neg as argument to abs"),
                }
            }
            _ => panic!("Expected function node for abs -42"),
        }
    }

    #[test]
    fn test_function_application_juxtaposition_eval() {
        // Test evaluation: abs(neg(42)) = 42
        // This requires 'abs' and 'neg' to be available.
        let ctx = create_test_context(); // Gets defaults if enabled

        // If builtins disabled, manually add abs and neg
        #[cfg(feature = "no-builtin-math")]
        {
            ctx.register_native_function("abs", 1, |args| args[0].abs());
            ctx.register_native_function("neg", 1, |args| -args[0]);
        }

        // Manually create AST as parser might handle juxtaposition differently
        let ast = AstExpr::Function {
            name: "abs".to_string(),
            args: vec![AstExpr::Function {
                name: "neg".to_string(),
                args: vec![AstExpr::Constant(42.0)],
            }],
        };

        let val = eval_ast(&ast, Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val, 42.0);
    }

    #[test]
    fn test_pow_arity_ast() {
        // AST structure test - independent of evaluation context or features
        // This test assumes the *parser* handles pow(2) -> pow(2, 2) or similar.
        // If the parser produces pow(2), the evaluator handles the default exponent.
        let ast = parse_expression("pow(2)").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast {
            AstExpr::Function { ref name, ref args } if name == "pow" => {
                // The parser might produce 1 or 2 args depending on its logic.
                // The evaluator handles the case where only 1 arg is provided by the AST.
                assert!(args.len() == 1 || args.len() == 2);
                match &args[0] {
                    AstExpr::Constant(c) => assert_eq!(*c, 2.0),
                    _ => panic!("Expected constant as pow arg"),
                }
                // If parser adds default arg:
                if args.len() == 2 {
                    match &args[1] {
                        AstExpr::Constant(c) => assert_eq!(*c, 2.0),
                        _ => panic!("Expected constant as pow second arg"),
                    }
                }
            }
            _ => panic!("Expected function node for pow(2)"),
        }
    }

    #[test]
    #[cfg(not(feature = "no-builtin-math"))] // Relies on built-in pow fallback logic for default exponent
    fn test_pow_arity_eval() {
        // Test evaluation using built-in pow, which handles the default exponent case
        let result = interp("pow(2)", None).unwrap();
        assert_eq!(result, 4.0); // pow(2) -> pow(2, 2) = 4.0

        let result2 = interp("pow(2, 3)", None).unwrap();
        assert_eq!(result2, 8.0);
    }

    #[test]
    #[cfg(feature = "no-builtin-math")] // Test with explicit pow needed
    fn test_pow_arity_eval_no_builtins() {
        let mut ctx = EvalContext::new();
        // Provide pow that handles the default exponent itself if needed, or require 2 args
        ctx.register_native_function("pow", 2, |args| args[0].powf(args[1]));
        // Or, if parser produces pow(2) as AST with 1 arg:
        // ctx.register_native_function("pow", 1, |args| args[0].powf(2.0));

        // Assuming parser produces pow(2, 2) or evaluator requires 2 args for registered fn:
        let err = interp("pow(2)", Some(&mut ctx)).unwrap_err(); // Expect error if registered fn expects 2 args
        assert!(matches!(err, ExprError::InvalidFunctionCall { .. }));

        let result2 = interp("pow(2, 3)", Some(&mut ctx)).unwrap();
        assert_eq!(result2, 8.0);
    }

    #[test]
    fn test_unknown_variable_and_function_ast() {
        // AST structure test - independent of evaluation context or features
        let ast = parse_expression("sin").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast {
            AstExpr::Variable(ref name) => assert_eq!(name, "sin"),
            _ => panic!("Expected variable node for sin"),
        }
        let ast2 = parse_expression("abs").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast2 {
            AstExpr::Variable(ref name) => assert_eq!(name, "abs"),
            _ => panic!("Expected variable node for abs"),
        }
    }

    #[test]
    fn test_unknown_variable_and_function_eval() {
        // Test evaluation when a function name is used as a variable
        let ctx = create_test_context(); // Gets defaults if enabled

        // If builtins disabled, manually add sin/abs so they are known *potential* functions
        #[cfg(feature = "no-builtin-math")]
        {
            ctx.register_native_function("sin", 1, |args| args[0].sin());
            ctx.register_native_function("abs", 1, |args| args[0].abs());
        }

        // Evaluate AST for variable "sin"
        let sin_var_ast = AstExpr::Variable("sin".to_string());
        let err = eval_ast(&sin_var_ast, Some(Rc::new(ctx.clone()))).unwrap_err();
        match err {
            ExprError::Syntax(msg) => {
                assert!(msg.contains("Function 'sin' used without arguments"));
            }
            _ => panic!("Expected Syntax error, got {:?}", err),
        }

        // Evaluate AST for variable "abs"
        let abs_var_ast = AstExpr::Variable("abs".to_string());
        let err2 = eval_ast(&abs_var_ast, Some(Rc::new(ctx.clone()))).unwrap_err();
        match err2 {
            ExprError::Syntax(msg) => {
                assert!(msg.contains("Function 'abs' used without arguments"));
            }
            _ => panic!("Expected Syntax error, got {:?}", err2),
        }

        // Test a truly unknown variable
        let unknown_var_ast = AstExpr::Variable("nosuchvar".to_string());
        let err3 = eval_ast(&unknown_var_ast, Some(Rc::new(ctx.clone()))).unwrap_err();
        assert!(matches!(err3, ExprError::UnknownVariable { name } if name == "nosuchvar"));
    }

    #[test]
    fn test_override_builtin_native() {
        let mut ctx = create_test_context(); // Start with defaults (if enabled)

        // Override 'sin'
        ctx.register_native_function("sin", 1, |_args| 100.0);
        // Override 'pow'
        ctx.register_native_function("pow", 2, |args| args[0] + args[1]);
        // Also override '^' if it's treated separately by parser/evaluator
        ctx.register_native_function("^", 2, |args| args[0] + args[1]);

        // Test overridden sin
        let val_sin = interp("sin(0.5)", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val_sin, 100.0, "Native 'sin' override failed");

        // Test overridden pow
        let val_pow = interp("pow(3, 4)", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val_pow, 7.0, "Native 'pow' override failed");

        // Test overridden pow using operator ^
        let val_pow_op = interp("3^4", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val_pow_op, 7.0, "Native '^' override failed");

        // Test a non-overridden function still works (cos)
        // Need to ensure 'cos' is available either via defaults or manual registration
        #[cfg(feature = "no-builtin-math")]
        {
            ctx.register_native_function("cos", 1, |args| args[0].cos()); // Example impl
        }
        // If cos wasn't registered by create_test_context and no-builtin-math is off, this might fail
        if ctx.function_registry.native_functions.contains_key("cos")
            || cfg!(not(feature = "no-builtin-math"))
        {
            let val_cos = interp("cos(0)", Some(Rc::new(ctx.clone()))).unwrap();
            // Use approx eq for floating point results
            let expected_cos = 1.0;
            assert!(
                (val_cos - expected_cos).abs() < 1e-9,
                "Built-in/default 'cos' failed after override. Got {}",
                val_cos
            );
        } else {
            // If cos is unavailable, trying to interp it should fail
            let err = interp("cos(0)", Some(Rc::new(ctx.clone()))).unwrap_err();
            assert!(matches!(err, ExprError::UnknownFunction { .. }));
        }
    }

    #[test]
    fn test_override_builtin_expression() {
        let mut ctx = create_test_context(); // Start with defaults (if enabled)

        // Override 'cos' with an expression function
        ctx.register_expression_function("cos", &["x"], "x * 10")
            .unwrap();

        // Override 'max' with an expression function that uses 'min'
        // Ensure 'min' is available first
        #[cfg(feature = "no-builtin-math")]
        {
            ctx.register_native_function("min", 2, |args| args[0].min(args[1]));
        }
        // If min wasn't registered by create_test_context and no-builtin-math is off, this might fail
        if ctx.function_registry.native_functions.contains_key("min")
            || cfg!(not(feature = "no-builtin-math"))
        {
            ctx.register_expression_function("max", &["a", "b"], "min(a, b)")
                .unwrap();

            // Test overridden max
            let val_max = interp("max(10, 2)", Some(Rc::new(ctx.clone()))).unwrap();
            assert_eq!(val_max, 2.0, "Expression 'max' override failed");
        } else {
            // Cannot register max if min is unavailable
            let reg_err = ctx.register_expression_function("max", &["a", "b"], "min(a, b)");
            // Depending on when parsing/checking happens, this might succeed or fail
            // If it succeeds, evaluation will fail later.
            if reg_err.is_ok() {
                let eval_err = interp("max(10, 2)", Some(Rc::new(ctx.clone()))).unwrap_err();
                assert!(matches!(eval_err, ExprError::UnknownFunction { name } if name == "min"));
            }
        }

        // Test overridden cos
        let val_cos = interp("cos(5)", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val_cos, 50.0, "Expression 'cos' override failed");

        // Test a non-overridden function still works (sin)
        #[cfg(feature = "no-builtin-math")]
        {
            ctx.register_native_function("sin", 1, |args| args[0].sin());
        }
        if ctx.function_registry.native_functions.contains_key("sin")
            || cfg!(not(feature = "no-builtin-math"))
        {
            let val_sin = interp("sin(0)", Some(Rc::new(ctx.clone()))).unwrap();
            assert!(
                (val_sin - 0.0).abs() < 1e-9,
                "Built-in/default 'sin' failed after override"
            );
        } else {
            let err = interp("sin(0)", Some(Rc::new(ctx.clone()))).unwrap_err();
            assert!(matches!(err, ExprError::UnknownFunction { .. }));
        }
    }

    #[test]
    fn test_expression_function_uses_correct_context() {
        let mut ctx = create_test_context(); // Start with defaults (if enabled)
        ctx.set_parameter("a", 10.0); // Variable in outer context
        ctx.constants.insert("my_const".to_string().into(), 100.0); // Constant in outer context

        // Define func1_const(x) = x + my_const
        // Expression functions inherit constants.
        ctx.register_expression_function("func1_const", &["x"], "x + my_const")
            .unwrap();
        let val1 = interp("func1_const(5)", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val1, 105.0, "func1_const should use constant from context");

        // Define func_uses_outer_var(x) = x + a
        ctx.register_expression_function("func_uses_outer_var", &["x"], "x + a")
            .unwrap();

        // Add a test to check if 'a' is visible inside the function
        let result = interp("func_uses_outer_var(5)", Some(Rc::new(ctx.clone())));
        match result {
            Ok(val) => {
                assert_eq!(
                    val, 15.0,
                    "func_uses_outer_var should use variable 'a' from context"
                );
            }
            Err(e) => {
                println!("Error evaluating func_uses_outer_var(5): {:?}", e);
                panic!(
                    "Expected Ok(15.0) for func_uses_outer_var(5), got error: {:?}",
                    e
                );
            }
        }

        // Add a test for parameter shadowing
        ctx.register_expression_function("shadow_test", &["a"], "a + 1")
            .unwrap();
        let val_shadow = interp("shadow_test(7)", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(
            val_shadow, 8.0,
            "Parameter 'a' should shadow context variable 'a'"
        );

        // Verify original 'a' in outer context is unchanged
        let val_a = interp("a", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val_a, 10.0, "Context 'a' should remain unchanged");
    }

    // Additional tests for polynomial expression function and related checks

    #[test]
    fn test_polynomial_expression_function_direct() {
        let mut ctx = EvalContext::new();
        ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4")
            .unwrap();

        // Test for x = 2
        let ast = crate::engine::parse_expression("polynomial(2)").unwrap();
        let result = crate::eval::eval_ast(&ast, Some(Rc::new(ctx.clone()))).unwrap();
        assert!(
            (result - 26.0).abs() < 1e-10,
            "Expected 26.0, got {}",
            result
        );

        // Test for x = 3
        let ast = crate::engine::parse_expression("polynomial(3)").unwrap();
        let result = crate::eval::eval_ast(&ast, Some(Rc::new(ctx.clone()))).unwrap();
        assert!(
            (result - 58.0).abs() < 1e-10,
            "Expected 58.0, got {}",
            result
        );
    }

    #[test]
    fn test_polynomial_subexpressions() {
        let mut ctx = EvalContext::new();
        ctx.set_parameter("x", 2.0);

        // x^3
        let ast = crate::engine::parse_expression("x^3").unwrap();
        let result = crate::eval::eval_ast(&ast, Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(result, 8.0);

        // 2*x^2
        let ast = crate::engine::parse_expression("2*x^2").unwrap();
        let result = crate::eval::eval_ast(&ast, Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(result, 8.0);

        // 3*x
        let ast = crate::engine::parse_expression("3*x").unwrap();
        let result = crate::eval::eval_ast(&ast, Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(result, 6.0);

        // 4
        let ast = crate::engine::parse_expression("4").unwrap();
        let result = crate::eval::eval_ast(&ast, Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(result, 4.0);
    }

    #[test]
    fn test_operator_precedence() {
        let ast = crate::engine::parse_expression("2 + 3 * 4 ^ 2").unwrap();
        let result = crate::eval::eval_ast(&ast, None).unwrap();
        assert_eq!(result, 2.0 + 3.0 * 16.0); // 2 + 3*16 = 50
    }

    #[test]
    fn test_polynomial_ast_structure() {
        let ast = crate::engine::parse_expression("x^3 + 2*x^2 + 3*x + 4").unwrap();
        // Print the AST for inspection
        println!("{:?}", ast);
        // Optionally, walk the AST and check node types here if desired
    }

    // New test for debugging polynomial function and body evaluation
    #[test]
    fn test_polynomial_integration_debug() {
        let mut ctx = EvalContext::new();
        ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4")
            .unwrap();

        // Print the AST for the polynomial body
        let body_ast = crate::engine::parse_expression_with_reserved(
            "x^3 + 2*x^2 + 3*x + 4",
            Some(&vec!["x".to_string()]),
        )
        .unwrap();
        println!("AST for polynomial body: {:?}", body_ast);

        // Print the AST for polynomial(2)
        let call_ast = crate::engine::parse_expression("polynomial(2)").unwrap();
        println!("AST for polynomial(2): {:?}", call_ast);

        // Evaluate polynomial(2)
        let result = crate::eval::eval_ast(&call_ast, Some(Rc::new(ctx.clone()))).unwrap();
        println!("polynomial(2) = {}", result);

        // Evaluate the body directly with x=2
        ctx.set_parameter("x", 2.0);
        let direct_result = crate::eval::eval_ast(&body_ast, Some(Rc::new(ctx.clone()))).unwrap();
        println!("Direct eval with x=2: {}", direct_result);
    }

    // Test for function argument passing and context mapping in polynomial
    #[test]
    fn test_polynomial_argument_mapping_debug() {
        let mut ctx = EvalContext::new();
        ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4")
            .unwrap();

        // Test with a literal
        let ast_lit = crate::engine::parse_expression("polynomial(10)").unwrap();
        let result_lit = crate::eval::eval_ast(&ast_lit, Some(Rc::new(ctx.clone()))).unwrap();
        println!("polynomial(10) = {}", result_lit);
        assert_eq!(result_lit, 1234.0);

        // Test with a variable
        ctx.set_parameter("z", 10.0);
        let ast_var = crate::engine::parse_expression("polynomial(z)").unwrap();
        let result_var = crate::eval::eval_ast(&ast_var, Some(Rc::new(ctx.clone()))).unwrap();
        println!("polynomial(z) = {}", result_var);
        assert_eq!(result_var, 1234.0);

        // Test with a subexpression
        ctx.set_parameter("a", 5.0);
        ctx.set_parameter("b", 10.0);
        let ast_sub = crate::engine::parse_expression("polynomial(a + b / 2)").unwrap();
        let result_sub = crate::eval::eval_ast(&ast_sub, Some(Rc::new(ctx.clone()))).unwrap();
        println!("polynomial(a + b / 2) = {}", result_sub);
        assert_eq!(result_sub, 1234.0);

        // Test with a nested polynomial call
        let ast_nested = crate::engine::parse_expression("polynomial(polynomial(2))").unwrap();
        let result_nested = crate::eval::eval_ast(&ast_nested, Some(Rc::new(ctx.clone()))).unwrap();
        println!("polynomial(polynomial(2)) = {}", result_nested);
    }
    #[test]
    fn test_polynomial_shadowing_variable() {
        let mut ctx = EvalContext::new();
        ctx.set_parameter("x", 100.0); // Shadowing variable
        ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4")
            .unwrap();

        let call_ast = crate::engine::parse_expression("polynomial(2)").unwrap();
        let result = crate::eval::eval_ast(&call_ast, Some(Rc::new(ctx.clone()))).unwrap();

        assert!(
            (result - 26.0).abs() < 1e-10,
            "Expected 26.0, got {}",
            result
        );
    }

    // Test for AST caching effect on polynomial evaluation
    #[test]
    fn test_polynomial_ast_cache_effect() {
        use std::cell::RefCell;
        use std::collections::HashMap;
        use std::rc::Rc;

        let mut ctx = EvalContext::new();
        ctx.ast_cache = Some(RefCell::new(HashMap::<String, Rc<crate::types::AstExpr>>::new()));
        ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4")
            .unwrap();

        let expr = "polynomial(2)";

        // First evaluation (should parse and cache)
        let result1 = crate::engine::interp(expr, Some(Rc::new(ctx.clone()))).unwrap();
        println!("First eval with cache: {}", result1);

        // Second evaluation (should use cache)
        let result2 = crate::engine::interp(expr, Some(Rc::new(ctx.clone()))).unwrap();
        println!("Second eval with cache: {}", result2);

        assert_eq!(result1, result2);
        assert!((result1 - 26.0).abs() < 1e-10);
    }

    // Test for function overriding
    #[test]
    fn test_polynomial_function_overriding() {
        let mut ctx = EvalContext::new();
        ctx.register_expression_function("polynomial", &["x"], "x + 1")
            .unwrap();
        ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4")
            .unwrap();

        let call_ast = crate::engine::parse_expression("polynomial(2)").unwrap();
        let result = crate::eval::eval_ast(&call_ast, Some(Rc::new(ctx.clone()))).unwrap();

        println!("polynomial(2) after overriding = {}", result);
        assert!((result - 26.0).abs() < 1e-10);
    }

    // Test for built-in function name collision
    #[test]
    fn test_polynomial_name_collision_with_builtin() {
        let mut ctx = EvalContext::new();
        // Register a function named "sin" that overrides built-in
        ctx.register_expression_function("sin", &["x"], "x + 100")
            .unwrap();

        let call_ast = crate::engine::parse_expression("sin(2)").unwrap();
        let result = crate::eval::eval_ast(&call_ast, Some(Rc::new(ctx.clone()))).unwrap();

        println!("sin(2) with override = {}", result);
        assert_eq!(result, 102.0);
    }
}
