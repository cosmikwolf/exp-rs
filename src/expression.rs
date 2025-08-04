//! Batch expression evaluation builder for efficient real-time evaluation
//!
//! This module provides a builder pattern for evaluating multiple expressions
//! with a shared set of parameters, optimized for real-time use cases.

use crate::engine::parse_expression;
use crate::error::ExprError;
use crate::eval::iterative::{EvalEngine, eval_with_engine};
use crate::types::{HString, TryIntoHeaplessString};
use crate::{AstExpr, EvalContext, Real};
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use bumpalo::Bump;
use heapless::FnvIndexMap;

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
}

impl<'arena> Expression<'arena> {
    /// Create a new empty batch builder with arena
    pub fn new(arena: &'arena Bump) -> Self {
        Expression {
            arena,
            expressions: Vec::new(),
            params: Vec::new(),
            results: Vec::new(),
            engine: EvalEngine::new_with_arena(arena),
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
        let mut param_map = FnvIndexMap::<HString, Real, 16>::new();
        for param in &self.params {
            let hname = param.name.as_str().try_into_heapless()?;
            param_map
                .insert(hname, param.value)
                .map_err(|_| ExprError::CapacityExceeded("parameter overrides"))?;
        }

        // Set parameter overrides in engine
        self.engine.set_param_overrides(param_map);

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
    /// expr.set("x", 2.0).unwrap();
    /// expr.set("y", 3.0).unwrap();
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
    pub fn eval_with_context(expr: &str, ctx: &Rc<EvalContext>, arena: &'arena Bump) -> Result<Real, ExprError> {
        let mut expression = Self::new(arena);
        expression.add_expression(expr)?;
        expression.eval(ctx)?;
        expression.get_result(0).ok_or(ExprError::Other("No result".to_string()))
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
        arena: &'arena Bump
    ) -> Result<Real, ExprError> {
        let mut expression = Self::new(arena);
        
        // Add all parameters
        for (name, value) in params {
            expression.add_parameter(name, *value)?;
        }
        
        expression.add_expression(expr)?;
        expression.eval(ctx)?;
        expression.get_result(0).ok_or(ExprError::Other("No result".to_string()))
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
    /// expr.set("x", 5.0).unwrap();
    /// 
    /// let result = expr.eval_single(&Rc::new(EvalContext::new())).unwrap();
    /// assert_eq!(result, 6.0);
    /// ```
    pub fn eval_single(&mut self, ctx: &Rc<EvalContext>) -> Result<Real, ExprError> {
        if self.expressions.len() != 1 {
            return Err(ExprError::Other("eval_single requires exactly one expression".to_string()));
        }
        
        self.eval(ctx)?;
        self.get_result(0).ok_or(ExprError::Other("No result".to_string()))
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
}

impl<'arena> ArenaBatchBuilder<'arena> {
    /// Create a new empty batch builder with arena
    pub fn new(arena: &'arena Bump) -> Self {
        ArenaBatchBuilder {
            arena,
            expressions: Vec::new(),
            params: Vec::new(),
            results: Vec::new(),
            engine: EvalEngine::new_with_arena(arena),
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
        let mut param_map = FnvIndexMap::<HString, Real, 16>::new();
        for param in &self.params {
            let hname = param.name.as_str().try_into_heapless()?;
            param_map
                .insert(hname, param.value)
                .map_err(|_| ExprError::CapacityExceeded("parameter overrides"))?;
        }

        // Set parameter overrides in engine
        self.engine.set_param_overrides(param_map);

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;

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
}

