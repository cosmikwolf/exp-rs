//! Batch expression evaluation builder for efficient real-time evaluation
//!
//! This module provides a builder pattern for evaluating multiple expressions
//! with a shared set of parameters, optimized for real-time use cases.

use crate::engine::parse_expression;
use crate::error::ExprError;
use crate::eval::iterative::{EvalEngine, eval_with_engine};
use crate::types::{TryIntoHeaplessString, BatchParamMap};
use crate::{AstExpr, EvalContext, Real};
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use bumpalo::Bump;
use core::cell::RefCell;

/// A parameter with its name and current value
#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub value: Real,
}

/// Builder for efficient batch expression evaluation
///
/// This structure allows you to:
/// 1. Pre-parse expressions once
/// 2. Reuse a single evaluation engine
/// 3. Update parameters efficiently
/// 4. Evaluate all expressions in one call
pub struct Expression<'arena> {
    /// The arena for all allocations
    arena: &'arena Bump,

    /// Pre-parsed expressions with their original strings
    expressions: Vec<(String, &'arena AstExpr<'arena>)>,

    /// Parameters with names and values together
    params: Vec<Param>,

    /// Results for each expression
    results: Vec<Real>,

    /// Reusable evaluation engine
    engine: EvalEngine<'arena>,

    /// Optional arena-allocated expression functions (lazy-initialized)
    local_functions: Option<&'arena RefCell<crate::types::ExpressionFunctionMap>>,
}

impl<'arena> Expression<'arena> {
    /// Create a new empty batch builder with arena
    pub fn new(arena: &'arena Bump) -> Self {
        Expression {
            arena,
            expressions: Vec::new(),
            params: Vec::new(),
            results: Vec::new(),
            engine: EvalEngine::new(arena),
            local_functions: None,
        }
    }

    /// Add an expression to be evaluated
    ///
    /// The expression is parsed immediately and cached.
    /// Returns the index of the added expression.
    pub fn add_expression(&mut self, expr: &str) -> Result<usize, ExprError> {
        let ast = parse_expression(expr, self.arena)?;
        let ast_ref = self.arena.alloc(ast);
        let idx = self.expressions.len();
        self.expressions.push((expr.to_string(), ast_ref));
        self.results.push(0.0); // Pre-allocate result slot
        Ok(idx)
    }

    /// Add a parameter with an initial value
    ///
    /// Returns an error if a parameter with the same name already exists.
    /// Returns the index of the added parameter.
    pub fn add_parameter(&mut self, name: &str, initial_value: Real) -> Result<usize, ExprError> {
        // Check for duplicates
        if self.params.iter().any(|p| p.name == name) {
            return Err(ExprError::DuplicateParameter(name.to_string()));
        }
        let idx = self.params.len();
        self.params.push(Param {
            name: name.to_string(),
            value: initial_value,
        });
        Ok(idx)
    }

    /// Update a parameter value by index (fastest method)
    pub fn set_param(&mut self, idx: usize, value: Real) -> Result<(), ExprError> {
        self.params
            .get_mut(idx)
            .ok_or(ExprError::InvalidParameterIndex(idx))?
            .value = value;
        Ok(())
    }

    /// Update a parameter value by name (convenient but slower)
    pub fn set_param_by_name(&mut self, name: &str, value: Real) -> Result<(), ExprError> {
        self.params
            .iter_mut()
            .find(|p| p.name == name)
            .ok_or_else(|| ExprError::UnknownVariable {
                name: name.to_string(),
            })?
            .value = value;
        Ok(())
    }

    /// Evaluate all expressions with current parameter values
    ///
    /// This uses parameter overrides instead of modifying the context,
    /// providing better performance and avoiding parameter accumulation.
    pub fn eval(&mut self, base_ctx: &Rc<EvalContext>) -> Result<(), ExprError> {
        // Build parameter override map
        let mut param_map = BatchParamMap::new();
        for param in &self.params {
            let hname = param.name.as_str().try_into_heapless()?;
            param_map
                .insert(hname, param.value)
                .map_err(|_| ExprError::CapacityExceeded("parameter overrides"))?;
        }

        // Set parameter overrides in engine
        self.engine.set_param_overrides(param_map);

        // Set local functions in engine
        self.engine.set_local_functions(self.local_functions);

        // Evaluate each expression with the original context
        for (i, (_, ast)) in self.expressions.iter().enumerate() {
            match eval_with_engine(ast, Some(base_ctx.clone()), &mut self.engine) {
                Ok(value) => self.results[i] = value,
                Err(e) => {
                    // Clear overrides on error
                    self.engine.clear_param_overrides();
                    return Err(e);
                }
            }
        }

        // Clear parameter overrides when done
        self.engine.clear_param_overrides();

        Ok(())
    }

    /// Get the result of a specific expression by index
    pub fn get_result(&self, expr_idx: usize) -> Option<Real> {
        self.results.get(expr_idx).copied()
    }

    /// Get all results as a slice
    pub fn get_all_results(&self) -> &[Real] {
        &self.results
    }

    /// Get the number of parameters
    pub fn param_count(&self) -> usize {
        self.params.len()
    }

    /// Get the number of expressions
    pub fn expression_count(&self) -> usize {
        self.expressions.len()
    }

    /// Get a parameter by index
    pub fn get_param(&self, idx: usize) -> Option<&Param> {
        self.params.get(idx)
    }

    /// Get a parameter by name
    pub fn get_param_by_name(&self, name: &str) -> Option<&Param> {
        self.params.iter().find(|p| p.name == name)
    }

    // === Convenience Methods for Single Expression Use ===

    /// Parse and create an Expression for a single expression
    ///
    /// This is a convenience constructor for single expression use cases.
    ///
    /// # Example
    /// ```
    /// use bumpalo::Bump;
    /// use exp_rs::expression::Expression;
    /// use exp_rs::EvalContext;
    /// use std::rc::Rc;
    ///
    /// let arena = Bump::new();
    /// let mut expr = Expression::parse("x^2 + y", &arena).unwrap();
    /// expr.add_parameter("x", 2.0).unwrap();
    /// expr.add_parameter("y", 3.0).unwrap();
    /// let result = expr.eval_single(&Rc::new(EvalContext::new())).unwrap();
    /// assert_eq!(result, 7.0); // 2^2 + 3 = 7
    /// ```
    pub fn parse(expr: &str, arena: &'arena Bump) -> Result<Self, ExprError> {
        let mut expression = Self::new(arena);
        expression.add_expression(expr)?;
        Ok(expression)
    }

    /// Evaluate a single expression without parameters
    ///
    /// This is the simplest way to evaluate an expression that doesn't need variables.
    ///
    /// # Example
    /// ```
    /// use bumpalo::Bump;
    /// use exp_rs::expression::Expression;
    ///
    /// let arena = Bump::new();
    /// let result = Expression::eval_simple("2 + 3 * 4", &arena).unwrap();
    /// assert_eq!(result, 14.0);
    /// ```
    pub fn eval_simple(expr: &str, arena: &'arena Bump) -> Result<Real, ExprError> {
        let ctx = Rc::new(EvalContext::new());
        Self::eval_with_context(expr, &ctx, arena)
    }

    /// Evaluate a single expression with context
    ///
    /// Use this when you have a context with pre-defined variables, constants, or functions.
    ///
    /// # Example
    /// ```
    /// use bumpalo::Bump;
    /// use exp_rs::{expression::Expression, EvalContext};
    /// use std::rc::Rc;
    ///
    /// let arena = Bump::new();
    /// let mut ctx = EvalContext::new();
    /// ctx.set_parameter("x", 5.0);
    ///
    /// let result = Expression::eval_with_context("x * 2", &Rc::new(ctx), &arena).unwrap();
    /// assert_eq!(result, 10.0);
    /// ```
    pub fn eval_with_context(
        expr: &str,
        ctx: &Rc<EvalContext>,
        arena: &'arena Bump,
    ) -> Result<Real, ExprError> {
        let mut expression = Self::new(arena);
        expression.add_expression(expr)?;
        expression.eval(ctx)?;
        expression
            .get_result(0)
            .ok_or(ExprError::Other("No result".to_string()))
    }

    /// Evaluate a single expression with parameters
    ///
    /// This is convenient when you want to provide parameters inline without creating a context.
    ///
    /// # Example
    /// ```
    /// use bumpalo::Bump;
    /// use exp_rs::{expression::Expression, EvalContext};
    /// use std::rc::Rc;
    ///
    /// let arena = Bump::new();
    /// let params = [("x", 3.0), ("y", 4.0)];
    /// let ctx = Rc::new(EvalContext::new());
    ///
    /// let result = Expression::eval_with_params("x^2 + y^2", &params, &ctx, &arena).unwrap();
    /// assert_eq!(result, 25.0); // 3^2 + 4^2 = 25
    /// ```
    pub fn eval_with_params(
        expr: &str,
        params: &[(&str, Real)],
        ctx: &Rc<EvalContext>,
        arena: &'arena Bump,
    ) -> Result<Real, ExprError> {
        let mut expression = Self::new(arena);

        // Add all parameters
        for (name, value) in params {
            expression.add_parameter(name, *value)?;
        }

        expression.add_expression(expr)?;
        expression.eval(ctx)?;
        expression
            .get_result(0)
            .ok_or(ExprError::Other("No result".to_string()))
    }

    /// For single expression mode - evaluate and return result directly
    ///
    /// This method assumes you've already added exactly one expression and returns
    /// its result directly instead of requiring a call to get_result().
    ///
    /// # Example
    /// ```
    /// use bumpalo::Bump;
    /// use exp_rs::{expression::Expression, EvalContext};
    /// use std::rc::Rc;
    ///
    /// let arena = Bump::new();
    /// let mut expr = Expression::parse("x + 1", &arena).unwrap();
    /// expr.add_parameter("x", 5.0).unwrap();
    ///
    /// let result = expr.eval_single(&Rc::new(EvalContext::new())).unwrap();
    /// assert_eq!(result, 6.0);
    /// ```
    pub fn eval_single(&mut self, ctx: &Rc<EvalContext>) -> Result<Real, ExprError> {
        if self.expressions.len() != 1 {
            return Err(ExprError::Other(
                "eval_single requires exactly one expression".to_string(),
            ));
        }

        self.eval(ctx)?;
        self.get_result(0)
            .ok_or(ExprError::Other("No result".to_string()))
    }

    /// Convenience setter using string slices
    ///
    /// This is an alias for set_param_by_name with a shorter name for convenience.
    ///
    /// # Example
    /// ```
    /// use bumpalo::Bump;
    /// use exp_rs::expression::Expression;
    ///
    /// let arena = Bump::new();
    /// let mut expr = Expression::new(&arena);
    /// expr.add_parameter("x", 0.0).unwrap();
    /// expr.set("x", 5.0).unwrap();
    /// ```
    pub fn set(&mut self, name: &str, value: Real) -> Result<(), ExprError> {
        self.set_param_by_name(name, value)
    }

    /// Register a local expression function for this batch
    ///
    /// Expression functions are mathematical expressions that can call other functions.
    /// They are specific to this batch and take precedence over context functions.
    ///
    /// # Arguments
    /// * `name` - Function name
    /// * `params` - Parameter names
    /// * `body` - Expression string defining the function
    ///
    /// # Example
    /// ```
    /// use bumpalo::Bump;
    /// use exp_rs::expression::Expression;
    ///
    /// let arena = Bump::new();
    /// let mut expr = Expression::new(&arena);
    /// expr.register_expression_function("distance", &["x1", "y1", "x2", "y2"],
    ///                                   "sqrt((x2-x1)^2 + (y2-y1)^2)").unwrap();
    /// ```
    pub fn register_expression_function(
        &mut self,
        name: &str,
        params: &[&str],
        body: &str,
    ) -> Result<(), ExprError> {
        use crate::types::{ExpressionFunction, ExpressionFunctionMap, TryIntoFunctionName};

        // Lazy initialization - only allocate map when first function is added
        if self.local_functions.is_none() {
            let map = self.arena.alloc(RefCell::new(ExpressionFunctionMap::new()));
            self.local_functions = Some(map);
        }

        // Create the function
        let func_name = name.try_into_function_name()?;
        let expr_func = ExpressionFunction {
            name: func_name.clone(),
            params: params.iter().map(|s| s.to_string()).collect(),
            expression: body.to_string(),
            description: None,
        };

        // Add to map through RefCell
        self.local_functions
            .unwrap()
            .borrow_mut()
            .insert(func_name, expr_func)
            .map_err(|_| ExprError::Other("Too many expression functions".to_string()))?;
        Ok(())
    }

    /// Remove a local expression function from this batch
    ///
    /// # Arguments
    /// * `name` - Function name to remove
    ///
    /// # Returns
    /// * `Ok(true)` if the function was removed
    /// * `Ok(false)` if the function didn't exist
    /// * `Err` if the name is invalid
    pub fn unregister_expression_function(&mut self, name: &str) -> Result<bool, ExprError> {
        use crate::types::TryIntoFunctionName;

        if let Some(map) = self.local_functions {
            let func_name = name.try_into_function_name()?;
            Ok(map.borrow_mut().remove(&func_name).is_some())
        } else {
            Ok(false)
        }
    }
}

// Default implementation removed - requires arena parameter

/// Arena-aware batch builder for zero-allocation expression evaluation
///
/// This structure is similar to BatchBuilder but uses an arena for all
/// AST allocations, eliminating dynamic memory allocation during evaluation.
pub struct ArenaBatchBuilder<'arena> {
    /// The arena for all allocations
    arena: &'arena Bump,

    /// Pre-parsed expressions with their original strings
    expressions: Vec<(&'arena str, &'arena AstExpr<'arena>)>,

    /// Parameters with names and values together
    params: Vec<Param>,

    /// Results for each expression
    results: Vec<Real>,

    /// Reusable evaluation engine
    engine: EvalEngine<'arena>,

    /// Optional arena-allocated expression functions (lazy-initialized)
    local_functions: Option<&'arena RefCell<crate::types::ExpressionFunctionMap>>,
}

impl<'arena> ArenaBatchBuilder<'arena> {
    /// Create a new empty batch builder with arena
    pub fn new(arena: &'arena Bump) -> Self {
        ArenaBatchBuilder {
            arena,
            expressions: Vec::new(),
            params: Vec::new(),
            results: Vec::new(),
            engine: EvalEngine::new(arena),
            local_functions: None,
        }
    }

    /// Add an expression to be evaluated
    ///
    /// The expression is parsed immediately into the arena.
    /// Returns the index of the added expression.
    pub fn add_expression(&mut self, expr: &str) -> Result<usize, ExprError> {
        // Parse the expression into the arena
        let ast = crate::engine::parse_expression(expr, self.arena)?;

        // Allocate expression string in arena
        let expr_str = self.arena.alloc_str(expr);

        // Allocate the AST in the arena
        let arena_ast = self.arena.alloc(ast);

        let idx = self.expressions.len();
        self.expressions.push((expr_str, arena_ast));
        self.results.push(0.0); // Pre-allocate result slot
        Ok(idx)
    }

    /// Add a parameter with an initial value
    ///
    /// Returns an error if a parameter with the same name already exists.
    /// Returns the index of the added parameter.
    pub fn add_parameter(&mut self, name: &str, initial_value: Real) -> Result<usize, ExprError> {
        // Check for duplicates
        if self.params.iter().any(|p| p.name == name) {
            return Err(ExprError::DuplicateParameter(name.to_string()));
        }
        let idx = self.params.len();
        self.params.push(Param {
            name: name.to_string(),
            value: initial_value,
        });
        Ok(idx)
    }

    /// Update a parameter value by index (fastest method)
    pub fn set_param(&mut self, idx: usize, value: Real) -> Result<(), ExprError> {
        self.params
            .get_mut(idx)
            .ok_or(ExprError::InvalidParameterIndex(idx))?
            .value = value;
        Ok(())
    }

    /// Update a parameter value by name (convenient but slower)
    pub fn set_param_by_name(&mut self, name: &str, value: Real) -> Result<(), ExprError> {
        self.params
            .iter_mut()
            .find(|p| p.name == name)
            .ok_or_else(|| ExprError::UnknownVariable {
                name: name.to_string(),
            })?
            .value = value;
        Ok(())
    }

    /// Evaluate all expressions with current parameter values
    pub fn eval(&mut self, base_ctx: &Rc<EvalContext>) -> Result<(), ExprError> {
        // Build parameter override map
        let mut param_map = BatchParamMap::new();
        for param in &self.params {
            let hname = param.name.as_str().try_into_heapless()?;
            param_map
                .insert(hname, param.value)
                .map_err(|_| ExprError::CapacityExceeded("parameter overrides"))?;
        }

        // Set parameter overrides in engine
        self.engine.set_param_overrides(param_map);

        // Set local functions in engine
        self.engine.set_local_functions(self.local_functions);

        // Evaluate each expression with the original context
        for (i, (_, ast)) in self.expressions.iter().enumerate() {
            match eval_with_engine(ast, Some(base_ctx.clone()), &mut self.engine) {
                Ok(value) => self.results[i] = value,
                Err(e) => {
                    // Clear overrides on error
                    self.engine.clear_param_overrides();
                    return Err(e);
                }
            }
        }

        // Clear parameter overrides when done
        self.engine.clear_param_overrides();

        Ok(())
    }

    /// Get the result of a specific expression by index
    pub fn get_result(&self, expr_idx: usize) -> Option<Real> {
        self.results.get(expr_idx).copied()
    }

    /// Get all results as a slice
    pub fn get_all_results(&self) -> &[Real] {
        &self.results
    }

    /// Get the number of parameters
    pub fn param_count(&self) -> usize {
        self.params.len()
    }

    /// Get the number of expressions
    pub fn expression_count(&self) -> usize {
        self.expressions.len()
    }

    /// Register a local expression function for this batch
    ///
    /// Expression functions are mathematical expressions that can call other functions.
    /// They are specific to this batch and take precedence over context functions.
    ///
    /// # Arguments
    /// * `name` - Function name
    /// * `params` - Parameter names
    /// * `body` - Expression string defining the function
    pub fn register_expression_function(
        &mut self,
        name: &str,
        params: &[&str],
        body: &str,
    ) -> Result<(), ExprError> {
        use crate::types::{ExpressionFunction, ExpressionFunctionMap, TryIntoFunctionName};

        // Lazy initialization - only allocate map when first function is added
        if self.local_functions.is_none() {
            let map = self.arena.alloc(RefCell::new(ExpressionFunctionMap::new()));
            self.local_functions = Some(map);
        }

        // Create the function
        let func_name = name.try_into_function_name()?;
        let expr_func = ExpressionFunction {
            name: func_name.clone(),
            params: params.iter().map(|s| s.to_string()).collect(),
            expression: body.to_string(),
            description: None,
        };

        // Add to map through RefCell
        self.local_functions
            .unwrap()
            .borrow_mut()
            .insert(func_name, expr_func)
            .map_err(|_| ExprError::Other("Too many expression functions".to_string()))?;
        Ok(())
    }

    /// Remove a local expression function from this batch
    ///
    /// # Arguments
    /// * `name` - Function name to remove
    ///
    /// # Returns
    /// * `Ok(true)` if the function was removed
    /// * `Ok(false)` if the function didn't exist
    /// * `Err` if the name is invalid
    pub fn unregister_expression_function(&mut self, name: &str) -> Result<bool, ExprError> {
        use crate::types::TryIntoFunctionName;

        if let Some(map) = self.local_functions {
            let func_name = name.try_into_function_name()?;
            Ok(map.borrow_mut().remove(&func_name).is_some())
        } else {
            Ok(false)
        }
    }

    /// Get the current number of bytes allocated in the arena
    pub fn arena_allocated_bytes(&self) -> usize {
        self.arena.allocated_bytes()
    }

    /// Clear all expressions, parameters, results, and local functions from this batch
    ///
    /// This allows the batch to be reused without recreating it. The arena memory
    /// used by previous expressions remains allocated but unused until the arena
    /// is reset. The evaluation engine is retained for reuse.
    ///
    /// # Example
    /// ```
    /// use bumpalo::Bump;
    /// use exp_rs::expression::ArenaBatchBuilder;
    ///
    /// let arena = Bump::new();
    /// let mut batch = ArenaBatchBuilder::new(&arena);
    /// batch.add_expression("x + 1").unwrap();
    /// batch.add_parameter("x", 5.0).unwrap();
    /// 
    /// // Clear and reuse
    /// batch.clear();
    /// assert_eq!(batch.expression_count(), 0);
    /// assert_eq!(batch.param_count(), 0);
    /// ```
    pub fn clear(&mut self) {
        self.expressions.clear();
        self.params.clear();
        self.results.clear();
        
        // Clear local functions if they exist
        if let Some(funcs) = self.local_functions {
            funcs.borrow_mut().clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use proptest::prelude::*;

    #[test]
    fn test_local_expression_functions() {
        let arena = Bump::new();
        let mut expr = Expression::new(&arena);

        // Register a local function
        expr.register_expression_function("double", &["x"], "x * 2")
            .unwrap();
        expr.register_expression_function("add_one", &["x"], "x + 1")
            .unwrap();

        // Use the functions in expressions
        expr.add_expression("double(5)").unwrap();
        expr.add_expression("add_one(10)").unwrap();
        expr.add_expression("double(add_one(3))").unwrap(); // Nested

        // Evaluate
        let ctx = Rc::new(EvalContext::new());
        expr.eval(&ctx).unwrap();

        // Check results
        assert_eq!(expr.get_result(0), Some(10.0)); // double(5) = 10
        assert_eq!(expr.get_result(1), Some(11.0)); // add_one(10) = 11
        assert_eq!(expr.get_result(2), Some(8.0)); // double(add_one(3)) = double(4) = 8

        // Test removing a function
        assert!(expr.unregister_expression_function("double").unwrap());
        assert!(!expr.unregister_expression_function("double").unwrap()); // Already removed
    }

    #[test]
    fn test_local_functions_override_context() {
        let arena = Bump::new();

        // Create context with a function
        let mut ctx = EvalContext::new();
        ctx.register_expression_function("calc", &["x"], "x * 2")
            .unwrap();
        let ctx = Rc::new(ctx);

        // Test 1: Use context function
        {
            let mut expr = Expression::new(&arena);
            expr.add_expression("calc(5)").unwrap();
            expr.eval(&ctx).unwrap();
            assert_eq!(expr.get_result(0), Some(10.0)); // x * 2 = 10
        }

        // Test 2: Local function overrides context function
        {
            let mut expr = Expression::new(&arena);
            // Register local function with same name (should override)
            expr.register_expression_function("calc", &["x"], "x * 3")
                .unwrap();
            expr.add_expression("calc(5)").unwrap();
            expr.eval(&ctx).unwrap();
            assert_eq!(expr.get_result(0), Some(15.0)); // x * 3 = 15
        }
    }

    #[test]
    fn test_batch_builder_basic() {
        let arena = Bump::new();
        let mut builder = Expression::new(&arena);

        // Add parameters
        let a_idx = builder.add_parameter("a", 2.0).unwrap();
        let b_idx = builder.add_parameter("b", 3.0).unwrap();

        // Add expressions
        let expr1_idx = builder.add_expression("a + b").unwrap();
        let expr2_idx = builder.add_expression("a * b").unwrap();

        // Create context
        let ctx = Rc::new(EvalContext::new());

        // Evaluate
        builder.eval(&ctx).unwrap();

        // Check results
        assert_eq!(builder.get_result(expr1_idx), Some(5.0));
        assert_eq!(builder.get_result(expr2_idx), Some(6.0));

        // Update parameters and re-evaluate
        builder.set_param(a_idx, 4.0).unwrap();
        builder.set_param(b_idx, 5.0).unwrap();
        builder.eval(&ctx).unwrap();

        assert_eq!(builder.get_result(expr1_idx), Some(9.0));
        assert_eq!(builder.get_result(expr2_idx), Some(20.0));
    }

    #[test]
    fn test_duplicate_parameter() {
        let arena = Bump::new();
        let mut builder = Expression::new(&arena);

        builder.add_parameter("x", 1.0).unwrap();
        let result = builder.add_parameter("x", 2.0);

        assert!(matches!(result, Err(ExprError::DuplicateParameter(_))));
    }

    // === Tests for Convenience Methods ===

    #[test]
    fn test_eval_simple() {
        let arena = Bump::new();

        // Test basic arithmetic
        assert_eq!(Expression::eval_simple("2 + 3 * 4", &arena).unwrap(), 14.0);
        assert_eq!(
            Expression::eval_simple("(2 + 3) * 4", &arena).unwrap(),
            20.0
        );
        assert_eq!(Expression::eval_simple("10 / 2 - 3", &arena).unwrap(), 2.0);

        // Test with constants
        #[cfg(feature = "libm")]
        {
            assert!(Expression::eval_simple("pi", &arena).unwrap() - std::f64::consts::PI < 0.0001);
            assert!(Expression::eval_simple("e", &arena).unwrap() - std::f64::consts::E < 0.0001);
        }
    }

    #[test]
    fn test_parse() {
        let arena = Bump::new();
        let ctx = Rc::new(EvalContext::new());

        // Test parsing various expressions
        let mut expr = Expression::parse("2 + 3", &arena).unwrap();
        assert_eq!(expr.eval_single(&ctx).unwrap(), 5.0);

        // Test with variables (should fail without parameters)
        let mut expr_with_var = Expression::parse("x + 1", &arena).unwrap();
        assert!(expr_with_var.eval_single(&ctx).is_err());

        // Add parameter and try again
        expr_with_var.add_parameter("x", 5.0).unwrap();
        assert_eq!(expr_with_var.eval_single(&ctx).unwrap(), 6.0);
    }

    #[test]
    fn test_eval_with_context() {
        let arena = Bump::new();
        let mut ctx = EvalContext::new();

        // Add some variables to context
        let _ = ctx.set_parameter("x", 10.0);
        let _ = ctx.set_parameter("y", 20.0);

        let ctx_rc = Rc::new(ctx);

        // Test evaluation with context variables
        assert_eq!(
            Expression::eval_with_context("x + y", &ctx_rc, &arena).unwrap(),
            30.0
        );
        assert_eq!(
            Expression::eval_with_context("x * 2 + y / 2", &ctx_rc, &arena).unwrap(),
            30.0
        );

        // Test with functions if available
        #[cfg(feature = "libm")]
        {
            assert_eq!(
                Expression::eval_with_context("sin(0)", &ctx_rc, &arena).unwrap(),
                0.0
            );
            assert_eq!(
                Expression::eval_with_context("cos(0)", &ctx_rc, &arena).unwrap(),
                1.0
            );
        }
    }

    #[test]
    fn test_eval_with_params() {
        let arena = Bump::new();
        let ctx = Rc::new(EvalContext::new());

        // Test with simple parameters
        let params = [("x", 3.0), ("y", 4.0)];
        assert_eq!(
            Expression::eval_with_params("x + y", &params, &ctx, &arena).unwrap(),
            7.0
        );

        // Test with complex expression
        assert_eq!(
            Expression::eval_with_params("x^2 + y^2", &params, &ctx, &arena).unwrap(),
            25.0
        );

        // Test with multiple parameters
        let params3 = [("a", 2.0), ("b", 3.0), ("c", 5.0)];
        assert_eq!(
            Expression::eval_with_params("a * b + c", &params3, &ctx, &arena).unwrap(),
            11.0
        );
    }

    #[test]
    fn test_eval_single() {
        let arena = Bump::new();
        let ctx = Rc::new(EvalContext::new());

        // Test basic usage
        let mut expr = Expression::parse("x^2 + 1", &arena).unwrap();
        expr.add_parameter("x", 3.0).unwrap();
        assert_eq!(expr.eval_single(&ctx).unwrap(), 10.0);

        // Test updating parameter
        expr.set("x", 4.0).unwrap();
        assert_eq!(expr.eval_single(&ctx).unwrap(), 17.0);

        // Test error when multiple expressions
        let mut multi_expr = Expression::new(&arena);
        multi_expr.add_expression("x + 1").unwrap();
        multi_expr.add_expression("x * 2").unwrap();
        assert!(multi_expr.eval_single(&ctx).is_err());
    }

    #[test]
    fn test_set_convenience_method() {
        let arena = Bump::new();
        let ctx = Rc::new(EvalContext::new());

        let mut expr = Expression::parse("a + b", &arena).unwrap();
        expr.add_parameter("a", 1.0).unwrap();
        expr.add_parameter("b", 2.0).unwrap();

        // Test initial evaluation
        assert_eq!(expr.eval_single(&ctx).unwrap(), 3.0);

        // Test using set method
        expr.set("a", 5.0).unwrap();
        assert_eq!(expr.eval_single(&ctx).unwrap(), 7.0);

        expr.set("b", 10.0).unwrap();
        assert_eq!(expr.eval_single(&ctx).unwrap(), 15.0);

        // Test error on unknown parameter
        assert!(expr.set("c", 100.0).is_err());
    }

    #[test]
    fn test_parameter_management() {
        let arena = Bump::new();

        let mut expr = Expression::new(&arena);

        // Test adding parameters
        let idx0 = expr.add_parameter("x", 1.0).unwrap();
        let idx1 = expr.add_parameter("y", 2.0).unwrap();
        let idx2 = expr.add_parameter("z", 3.0).unwrap();

        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);
        assert_eq!(idx2, 2);
        assert_eq!(expr.param_count(), 3);

        // Test getting parameters
        assert_eq!(expr.get_param(0).unwrap().name, "x");
        assert_eq!(expr.get_param(0).unwrap().value, 1.0);

        assert_eq!(expr.get_param_by_name("y").unwrap().name, "y");
        assert_eq!(expr.get_param_by_name("y").unwrap().value, 2.0);

        // Test updating by index
        expr.set_param(0, 10.0).unwrap();
        assert_eq!(expr.get_param(0).unwrap().value, 10.0);

        // Test updating by name
        expr.set_param_by_name("z", 30.0).unwrap();
        assert_eq!(expr.get_param_by_name("z").unwrap().value, 30.0);

        // Test invalid operations
        assert!(expr.set_param(10, 100.0).is_err());
        assert!(expr.set_param_by_name("nonexistent", 100.0).is_err());
        assert!(expr.get_param(10).is_none());
        assert!(expr.get_param_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_expression_management() {
        let arena = Bump::new();
        let ctx = Rc::new(EvalContext::new());

        let mut expr = Expression::new(&arena);

        // Add multiple expressions
        let idx0 = expr.add_expression("1 + 2").unwrap();
        let idx1 = expr.add_expression("3 * 4").unwrap();
        let idx2 = expr.add_expression("5 - 6").unwrap();

        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);
        assert_eq!(idx2, 2);
        assert_eq!(expr.expression_count(), 3);

        // Evaluate all
        expr.eval(&ctx).unwrap();

        // Check results
        assert_eq!(expr.get_result(0), Some(3.0));
        assert_eq!(expr.get_result(1), Some(12.0));
        assert_eq!(expr.get_result(2), Some(-1.0));

        // Test get_all_results
        let all_results = expr.get_all_results();
        assert_eq!(all_results.len(), 3);
        assert_eq!(all_results[0], 3.0);
        assert_eq!(all_results[1], 12.0);
        assert_eq!(all_results[2], -1.0);

        // Test invalid index
        assert_eq!(expr.get_result(10), None);
    }

    #[test]
    fn test_complex_expressions_with_params() {
        let arena = Bump::new();
        let mut ctx = EvalContext::new();

        // Register a custom function
        ctx.register_expression_function("quad", &["a", "b", "c", "x"], "a*x^2 + b*x + c")
            .unwrap();
        let ctx_rc = Rc::new(ctx);

        let mut expr = Expression::parse("quad(1, -3, 2, t)", &arena).unwrap();
        expr.add_parameter("t", 0.0).unwrap();

        // Test quadratic at different points
        let test_points = [(0.0, 2.0), (1.0, 0.0), (2.0, 0.0), (3.0, 2.0)];

        for (t_val, expected) in test_points {
            expr.set("t", t_val).unwrap();
            assert_eq!(expr.eval_single(&ctx_rc).unwrap(), expected);
        }
    }

    // === Property-based tests ===

    proptest! {
        #[test]
        fn prop_eval_simple_arithmetic_properties(a in -1000.0..1000.0f64, b in -1000.0..1000.0f64) {
            let arena = Bump::new();

            // Test commutativity of addition
            let add1 = Expression::eval_simple(&format!("{} + {}", a, b), &arena).unwrap();
            let add2 = Expression::eval_simple(&format!("{} + {}", b, a), &arena).unwrap();
            prop_assert!((add1 - add2).abs() < 1e-10);

            // Test commutativity of multiplication
            let mul1 = Expression::eval_simple(&format!("{} * {}", a, b), &arena).unwrap();
            let mul2 = Expression::eval_simple(&format!("{} * {}", b, a), &arena).unwrap();
            prop_assert!((mul1 - mul2).abs() < 1e-10);

            // Test identity properties
            let id_add = Expression::eval_simple(&format!("{} + 0", a), &arena).unwrap();
            prop_assert!((id_add - a).abs() < 1e-10);

            let id_mul = Expression::eval_simple(&format!("{} * 1", a), &arena).unwrap();
            prop_assert!((id_mul - a).abs() < 1e-10);
        }

        #[test]
        fn prop_parameter_names(name in "[a-zA-Z][a-zA-Z0-9_]{0,15}") {
            let arena = Bump::new();
            let mut expr = Expression::new(&arena);

            // Valid parameter names should work
            let result = expr.add_parameter(&name, 42.0);
            prop_assert!(result.is_ok());

            // Duplicate names should fail
            let duplicate = expr.add_parameter(&name, 100.0);
            prop_assert!(matches!(duplicate, Err(ExprError::DuplicateParameter(_))));

            // Should be able to get and set by name
            prop_assert_eq!(expr.get_param_by_name(&name).unwrap().value, 42.0);

            expr.set(&name, 123.0).unwrap();
            prop_assert_eq!(expr.get_param_by_name(&name).unwrap().value, 123.0);
        }

        #[test]
        fn prop_eval_with_random_params(
            num_params in 1..5usize,
            values in prop::collection::vec(-100.0..100.0f64, 1..5)
        ) {
            let arena = Bump::new();
            let ctx = Rc::new(EvalContext::new());

            // Generate unique parameter names
            let params: Vec<(String, f64)> = values.iter()
                .take(num_params)
                .enumerate()
                .map(|(i, val)| (format!("p{}", i), *val))
                .collect();

            // Build expression that sums all parameters
            let param_names: Vec<String> = params.iter().map(|(name, _)| name.clone()).collect();
            let expr_str = param_names.join(" + ");

            // Calculate expected sum
            let expected: f64 = params.iter().map(|(_, val)| val).sum();

            // Evaluate using eval_with_params
            let param_refs: Vec<(&str, f64)> = params.iter()
                .map(|(name, val)| (name.as_str(), *val))
                .collect();

            if !params.is_empty() {
                let result = Expression::eval_with_params(&expr_str, &param_refs, &ctx, &arena).unwrap();
                prop_assert!((result - expected).abs() < 1e-10);
            }
        }

        #[test]
        fn prop_expression_evaluation_consistency(
            a in -100.0..100.0f64,
            b in -100.0..100.0f64,
            c in -100.0..100.0f64
        ) {
            let arena = Bump::new();
            let ctx = Rc::new(EvalContext::new());

            // Test that different evaluation methods give same results
            let expr_str = "a * b + c";

            // Method 1: eval_with_params
            let params = [("a", a), ("b", b), ("c", c)];
            let result1 = Expression::eval_with_params(expr_str, &params, &ctx, &arena).unwrap();

            // Method 2: parse + add_parameter + eval_single
            let mut expr = Expression::parse(expr_str, &arena).unwrap();
            expr.add_parameter("a", a).unwrap();
            expr.add_parameter("b", b).unwrap();
            expr.add_parameter("c", c).unwrap();
            let result2 = expr.eval_single(&ctx).unwrap();

            // Method 3: eval_with_context (after setting up context)
            let mut ctx2 = EvalContext::new();
            let _ = ctx2.set_parameter("a", a);
            let _ = ctx2.set_parameter("b", b);
            let _ = ctx2.set_parameter("c", c);
            let result3 = Expression::eval_with_context(expr_str, &Rc::new(ctx2), &arena).unwrap();

            // All methods should give the same result
            prop_assert!((result1 - result2).abs() < 1e-10);
            prop_assert!((result2 - result3).abs() < 1e-10);
        }

        #[test]
        fn prop_parameter_updates(
            initial_vals in prop::collection::vec(-100.0..100.0f64, 3..10),
            update_indices in prop::collection::vec(0..10usize, 5..20),
            update_vals in prop::collection::vec(-100.0..100.0f64, 5..20)
        ) {
            let arena = Bump::new();
            let mut expr = Expression::new(&arena);

            // Add parameters with initial values
            for (i, val) in initial_vals.iter().enumerate() {
                expr.add_parameter(&format!("p{}", i), *val).unwrap();
            }

            // Apply updates and verify
            for (idx, val) in update_indices.iter().zip(update_vals.iter()) {
                if *idx < initial_vals.len() {
                    expr.set_param(*idx, *val).unwrap();
                    prop_assert_eq!(expr.get_param(*idx).unwrap().value, *val);
                } else {
                    // Out of bounds should fail
                    prop_assert!(expr.set_param(*idx, *val).is_err());
                }
            }
        }
    }
}
