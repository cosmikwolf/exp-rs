//! Batch expression evaluation builder for efficient real-time evaluation
//!
//! This module provides a builder pattern for evaluating multiple expressions
//! with a shared set of parameters, optimized for real-time use cases.

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

        // Pre-allocate parameter buffer in arena for zero-allocation evaluation
        let param_buffer = if params.is_empty() {
            None
        } else {
            // Pre-allocate parameter slice in arena
            let slice: &mut [(crate::types::HString, crate::Real)] = 
                self.arena.alloc_slice_fill_default(params.len());
            
            // Pre-fill parameter names (they never change)
            for (i, param_name) in params.iter().enumerate() {
                slice[i].0 = param_name.try_into_heapless()?;
                slice[i].1 = 0.0; // Default value
            }
            
            Some(slice as *mut _)
        };

        // Create the function
        let func_name = name.try_into_function_name()?;
        let expr_func = ExpressionFunction {
            name: func_name.clone(),
            params: params.iter().map(|s| s.to_string()).collect(),
            expression: body.to_string(),
            description: None,
            param_buffer,
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

    // === Convenience Methods ===

    /// Evaluate a single expression without parameters
    ///
    /// This is the simplest way to evaluate an expression that doesn't need variables.
    ///
    /// # Example
    /// ```
    /// use bumpalo::Bump;
    /// use exp_rs::expression::ArenaBatchBuilder;
    ///
    /// let arena = Bump::new();
    /// let result = ArenaBatchBuilder::eval_simple("2 + 3 * 4", &arena).unwrap();
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
    /// use exp_rs::{expression::ArenaBatchBuilder, EvalContext};
    /// use std::rc::Rc;
    ///
    /// let arena = Bump::new();
    /// let mut ctx = EvalContext::new();
    /// ctx.set_parameter("x", 5.0);
    ///
    /// let result = ArenaBatchBuilder::eval_with_context("x * 2", &Rc::new(ctx), &arena).unwrap();
    /// assert_eq!(result, 10.0);
    /// ```
    pub fn eval_with_context(
        expr: &str,
        ctx: &Rc<EvalContext>,
        arena: &'arena Bump,
    ) -> Result<Real, ExprError> {
        let mut builder = Self::new(arena);
        builder.add_expression(expr)?;
        builder.eval(ctx)?;
        builder
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
    /// use exp_rs::{expression::ArenaBatchBuilder, EvalContext};
    /// use std::rc::Rc;
    ///
    /// let arena = Bump::new();
    /// let params = [("x", 3.0), ("y", 4.0)];
    /// let ctx = Rc::new(EvalContext::new());
    ///
    /// let result = ArenaBatchBuilder::eval_with_params("x^2 + y^2", &params, &ctx, &arena).unwrap();
    /// assert_eq!(result, 25.0); // 3^2 + 4^2 = 25
    /// ```
    pub fn eval_with_params(
        expr: &str,
        params: &[(&str, Real)],
        ctx: &Rc<EvalContext>,
        arena: &'arena Bump,
    ) -> Result<Real, ExprError> {
        let mut builder = Self::new(arena);

        // Add all parameters
        for (name, value) in params {
            builder.add_parameter(name, *value)?;
        }

        builder.add_expression(expr)?;
        builder.eval(ctx)?;
        builder
            .get_result(0)
            .ok_or(ExprError::Other("No result".to_string()))
    }

    /// Convenience setter using string slices
    ///
    /// This is an alias for set_param_by_name with a shorter name for convenience.
    ///
    /// # Example
    /// ```
    /// use bumpalo::Bump;
    /// use exp_rs::expression::ArenaBatchBuilder;
    ///
    /// let arena = Bump::new();
    /// let mut builder = ArenaBatchBuilder::new(&arena);
    /// builder.add_parameter("x", 0.0).unwrap();
    /// builder.set("x", 5.0).unwrap();
    /// ```
    pub fn set(&mut self, name: &str, value: Real) -> Result<(), ExprError> {
        self.set_param_by_name(name, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;

    // === Tests for ArenaBatchBuilder Convenience Methods ===

    #[test]
    fn test_arena_batch_eval_simple() {
        let arena = Bump::new();

        // Test basic arithmetic
        assert_eq!(ArenaBatchBuilder::eval_simple("2 + 3 * 4", &arena).unwrap(), 14.0);
        assert_eq!(
            ArenaBatchBuilder::eval_simple("(2 + 3) * 4", &arena).unwrap(),
            20.0
        );
        assert_eq!(ArenaBatchBuilder::eval_simple("10 / 2 - 3", &arena).unwrap(), 2.0);

        // Test with constants
        #[cfg(feature = "libm")]
        {
            assert!(ArenaBatchBuilder::eval_simple("pi", &arena).unwrap() - std::f64::consts::PI < 0.0001);
            assert!(ArenaBatchBuilder::eval_simple("e", &arena).unwrap() - std::f64::consts::E < 0.0001);
        }
    }

    #[test]
    fn test_arena_batch_eval_with_context() {
        let arena = Bump::new();
        let mut ctx = EvalContext::new();

        // Add some variables to context
        let _ = ctx.set_parameter("x", 10.0);
        let _ = ctx.set_parameter("y", 20.0);

        let ctx_rc = Rc::new(ctx);

        // Test evaluation with context variables
        assert_eq!(
            ArenaBatchBuilder::eval_with_context("x + y", &ctx_rc, &arena).unwrap(),
            30.0
        );
        assert_eq!(
            ArenaBatchBuilder::eval_with_context("x * 2 + y / 2", &ctx_rc, &arena).unwrap(),
            30.0
        );

        // Test with functions if available
        #[cfg(feature = "libm")]
        {
            assert_eq!(
                ArenaBatchBuilder::eval_with_context("sin(0)", &ctx_rc, &arena).unwrap(),
                0.0
            );
            assert_eq!(
                ArenaBatchBuilder::eval_with_context("cos(0)", &ctx_rc, &arena).unwrap(),
                1.0
            );
        }
    }

    #[test]
    fn test_arena_batch_eval_with_params() {
        let arena = Bump::new();
        let ctx = Rc::new(EvalContext::new());

        // Test with simple parameters
        let params = [("x", 3.0), ("y", 4.0)];
        assert_eq!(
            ArenaBatchBuilder::eval_with_params("x + y", &params, &ctx, &arena).unwrap(),
            7.0
        );

        // Test with complex expression
        assert_eq!(
            ArenaBatchBuilder::eval_with_params("x^2 + y^2", &params, &ctx, &arena).unwrap(),
            25.0
        );

        // Test with multiple parameters
        let params3 = [("a", 2.0), ("b", 3.0), ("c", 5.0)];
        assert_eq!(
            ArenaBatchBuilder::eval_with_params("a * b + c", &params3, &ctx, &arena).unwrap(),
            11.0
        );
    }

    #[test]
    fn test_arena_batch_set_convenience_method() {
        let arena = Bump::new();
        let ctx = Rc::new(EvalContext::new());

        let mut builder = ArenaBatchBuilder::new(&arena);
        builder.add_parameter("a", 1.0).unwrap();
        builder.add_parameter("b", 2.0).unwrap();
        builder.add_expression("a + b").unwrap();

        // Test initial evaluation
        builder.eval(&ctx).unwrap();
        assert_eq!(builder.get_result(0), Some(3.0));

        // Test using set method
        builder.set("a", 5.0).unwrap();
        builder.eval(&ctx).unwrap();
        assert_eq!(builder.get_result(0), Some(7.0));

        builder.set("b", 10.0).unwrap();
        builder.eval(&ctx).unwrap();
        assert_eq!(builder.get_result(0), Some(15.0));

        // Test error on unknown parameter
        assert!(builder.set("c", 100.0).is_err());
    }

    #[test]
    fn test_arena_batch_local_expression_functions() {
        let arena = Bump::new();
        let mut builder = ArenaBatchBuilder::new(&arena);

        // Register a local function
        builder.register_expression_function("double", &["x"], "x * 2")
            .unwrap();
        builder.register_expression_function("add_one", &["x"], "x + 1")
            .unwrap();

        // Use the functions in expressions
        builder.add_expression("double(5)").unwrap();
        builder.add_expression("add_one(10)").unwrap();
        builder.add_expression("double(add_one(3))").unwrap(); // Nested

        // Evaluate
        let ctx = Rc::new(EvalContext::new());
        builder.eval(&ctx).unwrap();

        // Check results
        assert_eq!(builder.get_result(0), Some(10.0)); // double(5) = 10
        assert_eq!(builder.get_result(1), Some(11.0)); // add_one(10) = 11
        assert_eq!(builder.get_result(2), Some(8.0)); // double(add_one(3)) = double(4) = 8

        // Test removing a function
        assert!(builder.unregister_expression_function("double").unwrap());
        assert!(!builder.unregister_expression_function("double").unwrap()); // Already removed
    }

    #[test]
    fn test_arena_batch_local_functions() {
        let arena = Bump::new();

        // Create basic context
        let ctx = Rc::new(EvalContext::new());

        // Test: Local arena function
        {
            let mut builder = ArenaBatchBuilder::new(&arena);
            // Register local function
            builder.register_expression_function("calc", &["x"], "x * 3")
                .unwrap();
            builder.add_expression("calc(5)").unwrap();
            builder.eval(&ctx).unwrap();
            assert_eq!(builder.get_result(0), Some(15.0)); // x * 3 = 15
        }
    }
}

// Implement Drop to manually free heap-allocated strings in ExpressionFunction objects
// This prevents memory leaks when the batch contains expression functions
impl<'arena> Drop for ArenaBatchBuilder<'arena> {
    fn drop(&mut self) {
        // Manually clear local functions to ensure String objects are dropped
        // This is important because if the arena is dropped before this builder,
        // the String objects won't get their destructors called
        if let Some(funcs) = self.local_functions {
            funcs.borrow_mut().clear();
        }
    }
}
