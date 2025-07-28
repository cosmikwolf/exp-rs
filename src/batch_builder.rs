//! Batch expression evaluation builder for efficient real-time evaluation
//!
//! This module provides a builder pattern for evaluating multiple expressions
//! with a shared set of parameters, optimized for real-time use cases.

use crate::{Real, AstExpr, EvalContext};
use crate::error::ExprError;
use crate::engine::parse_expression;
use crate::eval::iterative::{EvalEngine, eval_with_engine};
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::rc::Rc;
use heapless::FnvIndexMap;
use crate::types::{HString, TryIntoHeaplessString};

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
pub struct BatchBuilder {
    /// Pre-parsed expressions with their original strings
    expressions: Vec<(String, AstExpr)>,
    
    /// Parameters with names and values together
    params: Vec<Param>,
    
    /// Results for each expression
    results: Vec<Real>,
    
    /// Reusable evaluation engine
    engine: EvalEngine,
}

impl BatchBuilder {
    /// Create a new empty batch builder
    pub fn new() -> Self {
        BatchBuilder {
            expressions: Vec::new(),
            params: Vec::new(),
            results: Vec::new(),
            engine: EvalEngine::new(),
        }
    }
    
    /// Add an expression to be evaluated
    ///
    /// The expression is parsed immediately and cached.
    /// Returns the index of the added expression.
    pub fn add_expression(&mut self, expr: &str) -> Result<usize, ExprError> {
        let ast = parse_expression(expr)?;
        let idx = self.expressions.len();
        self.expressions.push((expr.to_string(), ast));
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
            .ok_or_else(|| ExprError::UnknownVariable { name: name.to_string() })?
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
            param_map.insert(hname, param.value)
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
}

impl Default for BatchBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_batch_builder_basic() {
        let mut builder = BatchBuilder::new();
        
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
        let mut builder = BatchBuilder::new();
        
        builder.add_parameter("x", 1.0).unwrap();
        let result = builder.add_parameter("x", 2.0);
        
        assert!(matches!(result, Err(ExprError::DuplicateParameter(_))));
    }
}