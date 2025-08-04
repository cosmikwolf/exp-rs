//! Arena-managed expression evaluator
//!
//! This module provides a high-level interface for evaluating expressions
//! with automatic arena lifecycle management.

extern crate alloc;
use crate::{EvalContext, Real};
use crate::engine::parse_expression;
use crate::eval::eval_ast;
use crate::error::Result;
use alloc::rc::Rc;
use bumpalo::Bump;

/// An expression evaluator that manages its own memory arena.
/// 
/// This provides a simple interface for evaluating expressions without
/// needing to manually manage arena lifetimes.
/// 
/// # Examples
/// 
/// ```
/// use exp_rs::evaluator::Evaluator;
/// use exp_rs::EvalContext;
/// use alloc::rc::Rc;
/// 
/// // Simple evaluation
/// let mut evaluator = Evaluator::new();
/// let result = evaluator.eval("2 + 3 * 4").unwrap();
/// assert_eq!(result, 14.0);
/// 
/// // With context
/// let mut ctx = EvalContext::new();
/// ctx.set_parameter("x", 5.0);
/// let result = evaluator.eval_with_context("x * 2", Rc::new(ctx)).unwrap();
/// assert_eq!(result, 10.0);
/// ```
pub struct Evaluator {
    arena: Bump,
}

impl Evaluator {
    /// Creates a new evaluator with a fresh arena.
    pub fn new() -> Self {
        Self {
            arena: Bump::new(),
        }
    }

    /// Creates a new evaluator with a pre-allocated arena capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: Bump::with_capacity(capacity),
        }
    }

    /// Evaluates an expression using the default context.
    pub fn eval(&self, expression: &str) -> Result<Real> {
        let ctx = EvalContext::new();
        self.eval_with_context(expression, Rc::new(ctx))
    }

    /// Evaluates an expression with a custom context.
    pub fn eval_with_context(&self, expression: &str, ctx: Rc<EvalContext>) -> Result<Real> {
        // Parse with arena
        let ast = parse_expression(expression, &self.arena)?;
        
        // Evaluate
        eval_ast(&ast, Some(ctx))
    }

    /// Resets the arena, freeing all allocated memory.
    /// 
    /// This is useful when evaluating many expressions in sequence
    /// to prevent unbounded memory growth.
    pub fn reset(&mut self) {
        self.arena.reset();
    }

    /// Returns the current memory usage of the arena in bytes.
    pub fn allocated_bytes(&self) -> usize {
        self.arena.allocated_bytes()
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TryIntoHeaplessString;

    #[test]
    fn test_simple_evaluation() {
        let evaluator = Evaluator::new();
        let result = evaluator.eval("2 + 3 * 4").unwrap();
        assert_eq!(result, 14.0);
    }

    #[test]
    fn test_evaluation_with_variables() {
        let evaluator = Evaluator::new();
        let mut ctx = EvalContext::new();
        ctx.set_parameter("x", 5.0);
        ctx.set_parameter("y", 3.0);
        
        let result = evaluator.eval_with_context("x * y + 2", Rc::new(ctx)).unwrap();
        assert_eq!(result, 17.0);
    }

    #[test]
    fn test_memory_reset() {
        let mut evaluator = Evaluator::with_capacity(1024);
        
        // Evaluate some expressions
        for i in 0..10 {
            let expr = format!("{} + {}", i, i);
            let _ = evaluator.eval(&expr).unwrap();
        }
        
        let bytes_before = evaluator.allocated_bytes();
        assert!(bytes_before > 0);
        
        evaluator.reset();
        
        // After reset, we can still evaluate
        let result = evaluator.eval("42").unwrap();
        assert_eq!(result, 42.0);
    }

    #[test]
    fn test_with_constants() {
        let evaluator = Evaluator::new();
        let mut ctx = EvalContext::new();
        ctx.constants.insert("ANSWER".try_into_heapless().unwrap(), 42.0).unwrap();
        
        let result = evaluator.eval_with_context("ANSWER * 2", Rc::new(ctx)).unwrap();
        assert_eq!(result, 84.0);
    }
}