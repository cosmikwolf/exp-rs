//! Iterative AST evaluator
//!
//! This module implements an iterative (non-recursive) AST evaluator using
//! an explicit stack. This approach eliminates stack overflow issues and
//! provides better performance for deeply nested expressions.

use crate::Real;
use crate::context::EvalContext;
use crate::error::ExprError;
use crate::eval::context_stack::ContextStack;
use crate::eval::stack_ops::EvalOp;
use crate::eval::types::{FunctionCacheEntry, OwnedNativeFunction};
use crate::types::{AstExpr, FunctionName, HString};
use crate::types::{TryIntoFunctionName, TryIntoHeaplessString};

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::rc::Rc;
use alloc::string::ToString;
use alloc::vec::Vec;
use heapless::FnvIndexMap;

/// Maximum depth of the operation stack (prevents runaway evaluation)
const MAX_STACK_DEPTH: usize = 1000;

/// Initial capacity for stacks (tuned for typical expressions)
const INITIAL_OP_CAPACITY: usize = 32;
const INITIAL_VALUE_CAPACITY: usize = 16;

/// Main iterative evaluation function
pub fn eval_iterative<'arena>(
    ast: &'arena AstExpr<'arena>,
    ctx: Option<Rc<EvalContext>>,
) -> Result<Real, ExprError> {
    let mut engine = EvalEngine::new();
    engine.eval(ast, ctx)
}

/// Reusable evaluation engine to avoid allocations
pub struct EvalEngine<'arena> {
    /// Operation stack
    op_stack: Vec<EvalOp<'arena>>,
    /// Value stack for intermediate results
    value_stack: Vec<Real>,
    /// Context management
    ctx_stack: ContextStack,
    /// Function cache
    func_cache: BTreeMap<HString, Option<FunctionCacheEntry>>,
    /// Parameter overrides for batch evaluation (avoids context modification)
    param_overrides: Option<FnvIndexMap<HString, Real, 16>>,
    /// Optional reference to local expression functions
    local_functions: Option<&'arena core::cell::RefCell<crate::types::ExpressionFunctionMap>>,
    /// Optional arena for parsing expression functions on-demand
    arena: Option<&'arena bumpalo::Bump>,
    /// Cache for parsed expression functions
    expr_func_cache: BTreeMap<HString, &'arena AstExpr<'arena>>,
}

impl<'arena> Default for EvalEngine<'arena> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'arena> EvalEngine<'arena> {
    /// Create a new evaluation engine
    pub fn new() -> Self {
        Self {
            op_stack: Vec::with_capacity(INITIAL_OP_CAPACITY),
            value_stack: Vec::with_capacity(INITIAL_VALUE_CAPACITY),
            ctx_stack: ContextStack::new(),
            func_cache: BTreeMap::new(),
            param_overrides: None,
            local_functions: None,
            arena: None,
            expr_func_cache: BTreeMap::new(),
        }
    }

    /// Create a new evaluation engine with arena for expression functions
    pub fn new_with_arena(arena: &'arena bumpalo::Bump) -> Self {
        Self {
            op_stack: Vec::with_capacity(INITIAL_OP_CAPACITY),
            value_stack: Vec::with_capacity(INITIAL_VALUE_CAPACITY),
            ctx_stack: ContextStack::new(),
            func_cache: BTreeMap::new(),
            param_overrides: None,
            local_functions: None,
            arena: Some(arena),
            expr_func_cache: BTreeMap::new(),
        }
    }
    
    /// Set the local expression functions for this evaluation
    pub fn set_local_functions(&mut self, functions: Option<&'arena core::cell::RefCell<crate::types::ExpressionFunctionMap>>) {
        self.local_functions = functions;
    }

    /// Evaluate an expression
    pub fn eval(
        &mut self,
        ast: &'arena AstExpr<'arena>,
        ctx: Option<Rc<EvalContext>>,
    ) -> Result<Real, ExprError> {
        // Clear stacks but keep capacity
        self.op_stack.clear();
        self.value_stack.clear();
        self.ctx_stack.clear();
        self.func_cache.clear();

        // Initialize with root context
        let root_ctx_id = self.ctx_stack.push_context(ctx)?;

        // Push initial operation (no clone needed - just use reference!)
        self.op_stack.push(EvalOp::Eval {
            expr: ast,
            ctx_id: root_ctx_id,
        });

        // Main evaluation loop
        while let Some(op) = self.op_stack.pop() {
            // Check depth limit
            if self.op_stack.len() > MAX_STACK_DEPTH {
                return Err(ExprError::RecursionLimit(format!(
                    "Maximum evaluation depth {} exceeded",
                    MAX_STACK_DEPTH
                )));
            }

            self.process_operation(op)?;
        }

        // Result should be on top of value stack
        self.value_stack
            .pop()
            .ok_or_else(|| ExprError::Other("No result on value stack".to_string()))
    }

    /// Process a single operation
    fn process_operation(&mut self, op: EvalOp<'arena>) -> Result<(), ExprError> {
        match op {
            EvalOp::Eval { expr, ctx_id } => {
                self.process_eval(expr, ctx_id)?;
            }

            EvalOp::ApplyUnary { op } => {
                let operand = self.pop_value()?;
                self.value_stack.push(op.apply(operand));
            }

            EvalOp::CompleteBinary { op } => {
                let right = self.pop_value()?;
                let left = self.pop_value()?;
                self.value_stack.push(op.apply(left, right));
            }

            EvalOp::ShortCircuitAnd { right_expr, ctx_id } => {
                let left_val = self.pop_value()?;
                if left_val == 0.0 {
                    // Short circuit - left is false, result is false
                    self.value_stack.push(0.0);
                } else {
                    // Need to evaluate right side
                    self.op_stack.push(EvalOp::CompleteAnd);
                    self.op_stack.push(EvalOp::Eval {
                        expr: right_expr,
                        ctx_id,
                    });
                }
            }

            EvalOp::ShortCircuitOr { right_expr, ctx_id } => {
                let left_val = self.pop_value()?;
                if left_val != 0.0 {
                    // Short circuit - left is true, result is true
                    self.value_stack.push(1.0);
                } else {
                    // Need to evaluate right side
                    self.op_stack.push(EvalOp::CompleteOr);
                    self.op_stack.push(EvalOp::Eval {
                        expr: right_expr,
                        ctx_id,
                    });
                }
            }

            EvalOp::CompleteAnd => {
                let right = self.pop_value()?;
                let left = 1.0; // We know left was true or we wouldn't be here
                self.value_stack.push(if left != 0.0 && right != 0.0 {
                    1.0
                } else {
                    0.0
                });
            }

            EvalOp::CompleteOr => {
                let right = self.pop_value()?;
                let left = 0.0; // We know left was false or we wouldn't be here
                self.value_stack.push(if left != 0.0 || right != 0.0 {
                    1.0
                } else {
                    0.0
                });
            }

            EvalOp::LookupVariable { name, ctx_id } => {
                self.process_variable_lookup(name, ctx_id)?;
            }

            EvalOp::TernaryCondition {
                true_branch,
                false_branch,
                ctx_id,
            } => {
                let condition = self.pop_value()?;
                if condition != 0.0 {
                    self.op_stack.push(EvalOp::Eval {
                        expr: true_branch,
                        ctx_id,
                    });
                } else {
                    self.op_stack.push(EvalOp::Eval {
                        expr: false_branch,
                        ctx_id,
                    });
                }
            }

            EvalOp::AccessArray { array_name, ctx_id } => {
                let index = self.pop_value()?;
                self.process_array_access(array_name, index, ctx_id)?;
            }

            EvalOp::AccessAttribute {
                object_name,
                attr_name,
                ctx_id,
            } => {
                self.process_attribute_access(object_name, attr_name, ctx_id)?;
            }

            EvalOp::CollectFunctionArgs {
                name,
                total_args,
                mut args_so_far,
                ctx_id,
            } => {
                // Pop one argument from value stack and insert at the beginning
                // to preserve the original order (since we evaluate args in reverse)
                let arg = self.pop_value()?;
                args_so_far.insert(0, arg);

                if args_so_far.len() == total_args {
                    // All arguments collected, apply function
                    self.op_stack.push(EvalOp::ApplyFunction {
                        name,
                        args_needed: total_args,
                        args_collected: args_so_far,
                        ctx_id,
                    });
                } else {
                    // Still need more arguments
                    self.op_stack.push(EvalOp::CollectFunctionArgs {
                        name,
                        total_args,
                        args_so_far,
                        ctx_id,
                    });
                }
            }

            EvalOp::ApplyFunction {
                name,
                args_needed,
                args_collected,
                ctx_id,
            } => {
                self.process_function_call(name, args_needed, args_collected, ctx_id)?;
            }
        }

        Ok(())
    }

    /// Process an Eval operation by converting AST to stack operations
    fn process_eval(
        &mut self,
        expr: &'arena AstExpr<'arena>,
        ctx_id: usize,
    ) -> Result<(), ExprError> {
        match expr {
            AstExpr::Constant(val) => {
                self.value_stack.push(*val);
            }

            AstExpr::Variable(name) => {
                let hname = name.try_into_heapless()?;
                self.op_stack.push(EvalOp::LookupVariable {
                    name: hname,
                    ctx_id,
                });
            }

            AstExpr::LogicalOp { op, left, right } => {
                // Handle short-circuit operators
                use crate::types::LogicalOperator;
                match op {
                    LogicalOperator::And => {
                        self.op_stack.push(EvalOp::ShortCircuitAnd {
                            right_expr: right,
                            ctx_id,
                        });
                        self.op_stack.push(EvalOp::Eval { expr: left, ctx_id });
                    }
                    LogicalOperator::Or => {
                        self.op_stack.push(EvalOp::ShortCircuitOr {
                            right_expr: right,
                            ctx_id,
                        });
                        self.op_stack.push(EvalOp::Eval { expr: left, ctx_id });
                    }
                }
            }

            AstExpr::Conditional {
                condition,
                true_branch,
                false_branch,
            } => {
                self.op_stack.push(EvalOp::TernaryCondition {
                    true_branch,
                    false_branch,
                    ctx_id,
                });
                self.op_stack.push(EvalOp::Eval {
                    expr: condition,
                    ctx_id,
                });
            }

            AstExpr::Function { name, args } => {
                // Special handling for short-circuit operators
                match (*name, args.len()) {
                    ("&&", 2) => {
                        // Short-circuit AND: evaluate left first, then right only if left is true
                        self.op_stack.push(EvalOp::ShortCircuitAnd {
                            right_expr: &args[1],
                            ctx_id,
                        });
                        self.op_stack.push(EvalOp::Eval {
                            expr: &args[0],
                            ctx_id,
                        });
                    }
                    ("||", 2) => {
                        // Short-circuit OR: evaluate left first, then right only if left is false
                        self.op_stack.push(EvalOp::ShortCircuitOr {
                            right_expr: &args[1],
                            ctx_id,
                        });
                        self.op_stack.push(EvalOp::Eval {
                            expr: &args[0],
                            ctx_id,
                        });
                    }
                    _ => {
                        // All other function calls go through the same path to support overrides
                        // The parser represents operators like ^, +, -, etc. as function calls
                        // So we treat them all uniformly to allow user overrides
                        let fname = name.try_into_function_name()?;

                        if args.is_empty() {
                            // No arguments to evaluate
                            self.op_stack.push(EvalOp::ApplyFunction {
                                name: fname,
                                args_needed: 0,
                                args_collected: Vec::new(),
                                ctx_id,
                            });
                        } else {
                            // Push collection operation
                            self.op_stack.push(EvalOp::CollectFunctionArgs {
                                name: fname,
                                total_args: args.len(),
                                args_so_far: Vec::new(),
                                ctx_id,
                            });

                            // Push argument evaluations in reverse order
                            for arg in args.iter().rev() {
                                self.op_stack.push(EvalOp::Eval { expr: arg, ctx_id });
                            }
                        }
                    }
                }
            }

            AstExpr::Array { name, index } => {
                let array_name = name.try_into_heapless()?;

                self.op_stack
                    .push(EvalOp::AccessArray { array_name, ctx_id });
                self.op_stack.push(EvalOp::Eval {
                    expr: index,
                    ctx_id,
                });
            }

            AstExpr::Attribute { base, attr } => {
                let obj_name = base.try_into_heapless()?;
                let attr_name = attr.try_into_heapless()?;

                self.op_stack.push(EvalOp::AccessAttribute {
                    object_name: obj_name,
                    attr_name,
                    ctx_id,
                });
            }
        }

        Ok(())
    }

    /// Process variable lookup
    fn process_variable_lookup(&mut self, name: HString, ctx_id: usize) -> Result<(), ExprError> {
        // Check parameter overrides first (highest priority for batch evaluation)
        if let Some(ref overrides) = self.param_overrides {
            if let Some(&value) = overrides.get(&name) {
                self.value_stack.push(value);
                return Ok(());
            }
        }

        // Try context stack next
        if let Some(value) = self.ctx_stack.lookup_variable(ctx_id, &name) {
            self.value_stack.push(value);
            return Ok(());
        }

        // Check if it's a built-in constant
        let value = match name.as_str() {
            "pi" | "PI" => core::f64::consts::PI as Real,
            "e" | "E" => core::f64::consts::E as Real,
            "tau" | "TAU" => 2.0 * core::f64::consts::PI as Real,
            _ => {
                // Check if this looks like a function name
                let is_potential_function_name = match name.as_str() {
                    "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "atan2" | "sinh"
                    | "cosh" | "tanh" | "exp" | "log" | "log10" | "ln" | "sqrt" | "abs"
                    | "ceil" | "floor" | "pow" | "neg" | "," | "comma" | "+" | "-" | "*" | "/"
                    | "%" | "^" | "max" | "min" | "<" | ">" | "<=" | ">=" | "==" | "!=" => true,
                    _ => false,
                };

                if is_potential_function_name && name.len() > 1 {
                    return Err(ExprError::Syntax(format!(
                        "Function '{}' used without arguments",
                        name
                    )));
                }

                return Err(ExprError::UnknownVariable {
                    name: name.to_string(),
                });
            }
        };

        self.value_stack.push(value);
        Ok(())
    }

    /// Process array access
    fn process_array_access(
        &mut self,
        array_name: HString,
        index: Real,
        ctx_id: usize,
    ) -> Result<(), ExprError> {
        let idx = index as usize;

        if let Some(ctx) = self.ctx_stack.get_context(ctx_id) {
            if let Some(array) = ctx.arrays.get(&array_name) {
                if idx < array.len() {
                    self.value_stack.push(array[idx]);
                    return Ok(());
                } else {
                    return Err(ExprError::ArrayIndexOutOfBounds {
                        name: array_name.to_string(),
                        index: idx,
                        len: array.len(),
                    });
                }
            }
        }

        Err(ExprError::UnknownVariable {
            name: array_name.to_string(),
        })
    }

    /// Process attribute access
    fn process_attribute_access(
        &mut self,
        object_name: HString,
        attr_name: HString,
        ctx_id: usize,
    ) -> Result<(), ExprError> {
        if let Some(ctx) = self.ctx_stack.get_context(ctx_id) {
            if let Some(obj_attrs) = ctx.attributes.get(&object_name) {
                if let Some(&value) = obj_attrs.get(&attr_name) {
                    self.value_stack.push(value);
                    return Ok(());
                }
            }
        }

        Err(ExprError::AttributeNotFound {
            base: object_name.to_string(),
            attr: attr_name.to_string(),
        })
    }

    /// Process function call
    fn process_function_call(
        &mut self,
        name: FunctionName,
        args_needed: usize,
        args: Vec<Real>,
        ctx_id: usize,
    ) -> Result<(), ExprError> {
        // Get context
        let ctx = self
            .ctx_stack
            .get_context(ctx_id)
            .ok_or_else(|| ExprError::Other("Invalid context ID".to_string()))?;

        // Check local functions first (highest priority)
        if let Some(local_funcs) = self.local_functions {
            if let Some(func) = local_funcs.borrow().get(&name).cloned() {
                return self.process_expression_function(&func, args, ctx_id);
            }
        }

        // Try expression function from context (second priority) - clone it to avoid borrowing issues
        let expr_func = ctx.get_expression_function(&name).cloned();
        if let Some(func) = expr_func {
            return self.process_expression_function(&func, args, ctx_id);
        }

        // Try native function second
        if let Some(func) = ctx.get_native_function(&name) {
            if args.len() != func.arity {
                return Err(ExprError::InvalidFunctionCall {
                    name: name.to_string(),
                    expected: func.arity,
                    found: args.len(),
                });
            }

            let owned_fn = OwnedNativeFunction::from(func);
            let result = (owned_fn.implementation)(&args);
            self.value_stack.push(result);
            return Ok(());
        }

        Err(ExprError::UnknownFunction {
            name: name.to_string(),
        })
    }


    /// Pop a value from the value stack
    fn pop_value(&mut self) -> Result<Real, ExprError> {
        self.value_stack
            .pop()
            .ok_or_else(|| ExprError::Other("Value stack underflow".to_string()))
    }

    /// Set parameter overrides for batch evaluation.
    /// These take precedence over context variables during lookup.
    pub fn set_param_overrides(&mut self, params: FnvIndexMap<HString, Real, 16>) {
        self.param_overrides = Some(params);
    }

    /// Clear parameter overrides.
    pub fn clear_param_overrides(&mut self) {
        self.param_overrides = None;
    }

    /// Execute a function with parameter overrides, ensuring they are cleared afterwards.
    /// This provides RAII-style cleanup for safe batch evaluation.
    pub fn with_param_overrides<F, R>(&mut self, params: FnvIndexMap<HString, Real, 16>, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        let old_overrides = self.param_overrides.take();
        self.param_overrides = Some(params);
        let result = f(self);
        self.param_overrides = old_overrides;
        result
    }
    
    /// Process an expression function call
    fn process_expression_function(
        &mut self,
        func: &crate::types::ExpressionFunction,
        args: Vec<Real>,
        ctx_id: usize,
    ) -> Result<(), ExprError> {
        use crate::types::TryIntoHeaplessString;
        
        if args.len() != func.params.len() {
            return Err(ExprError::InvalidFunctionCall {
                name: func.name.to_string(),
                expected: func.params.len(),
                found: args.len(),
            });
        }

        // Get context
        let ctx = self
            .ctx_stack
            .get_context(ctx_id)
            .ok_or_else(|| ExprError::Other("Invalid context ID".to_string()))?;

        // Create new context for function evaluation
        let mut func_ctx = EvalContext::new();

        // Set parameters
        for (param, value) in func.params.iter().zip(args.iter()) {
            func_ctx.set_parameter(param, *value)?;
        }

        // Copy function registry
        func_ctx.function_registry = ctx.function_registry.clone();

        // Set parent context to inherit variables and constants
        func_ctx.parent = Some(ctx.clone());

        // Parse expression function on-demand if we have an arena
        if let Some(arena) = self.arena {
            // Check if we've already parsed this function
            let func_key = func.name.try_into_heapless()?;

            let ast = if let Some(&cached_ast) = self.expr_func_cache.get(&func_key) {
                cached_ast
            } else {
                // Parse the expression function body into the arena
                let param_names: Vec<crate::String> = func.params.clone();
                let parsed_ast = crate::engine::parse_expression_with_parameters(
                    &func.expression,
                    arena,
                    &param_names,
                )?;

                // Allocate the AST in the arena
                let arena_ast = arena.alloc(parsed_ast);

                // Cache for future use
                self.expr_func_cache.insert(func_key.clone(), &*arena_ast);

                &*arena_ast
            };

            // Push the function's AST for evaluation with the new context
            let func_ctx_id = self.ctx_stack.push_context(Some(Rc::new(func_ctx)))?;
            self.op_stack.push(EvalOp::Eval {
                expr: ast,
                ctx_id: func_ctx_id,
            });
            Ok(())
        } else {
            // No arena available for expression functions
            Err(ExprError::Other(
                "Expression functions require an arena-enabled evaluator".to_string(),
            ))
        }
    }
}

/// Evaluate an expression using a provided engine (avoids engine allocation).
///
/// This function is useful for batch evaluation where the same engine can be
/// reused across multiple evaluations, avoiding the overhead of creating a
/// new engine for each evaluation.
///
/// # Parameters
///
/// * `ast` - The abstract syntax tree to evaluate
/// * `ctx` - Optional evaluation context containing variables and functions
/// * `engine` - Mutable reference to an existing evaluation engine
///
/// # Returns
///
/// The result of evaluating the expression, or an error if evaluation fails
///
/// # Example
///
/// ```
/// use exp_rs::eval::iterative::{eval_with_engine, EvalEngine};
/// use exp_rs::engine::parse_expression;
/// use exp_rs::context::EvalContext;
/// use std::rc::Rc;
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let ast = parse_expression("2 + 3", &arena).unwrap();
/// let mut engine = EvalEngine::new_with_arena(&arena);
/// let result = eval_with_engine(&ast, None, &mut engine).unwrap();
/// assert_eq!(result, 5.0);
/// ```
pub fn eval_with_engine<'arena>(
    ast: &'arena AstExpr<'arena>,
    ctx: Option<Rc<EvalContext>>,
    engine: &mut EvalEngine<'arena>,
) -> Result<Real, ExprError> {
    engine.eval(ast, ctx)
}

