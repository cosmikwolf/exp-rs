//! Test utilities for arena-based AST creation
//!
//! This module provides thread-local arenas and helper functions
//! for tests that need to create AST nodes manually.

#![cfg(test)]

/// Macro to help migrate old tests from parse_expression to parse_expression_arena
#[macro_export]
macro_rules! parse_expression {
    ($expr:expr) => {
        $crate::test_utils::parse($expr)
    };
}

use bumpalo::Bump;
use crate::types::AstExpr;
use crate::Real;
use std::cell::RefCell;

thread_local! {
    /// Thread-local arena for test AST creation
    static TEST_ARENA: RefCell<Bump> = RefCell::new(Bump::new());
}

/// Reset the test arena (call between tests to avoid memory growth)
pub fn reset_test_arena() {
    TEST_ARENA.with(|arena| {
        arena.borrow_mut().reset();
    });
}

/// Create a constant AST node
pub fn constant(value: Real) -> AstExpr<'static> {
    AstExpr::Constant(value)
}

/// Create a variable AST node
pub fn variable(name: &str) -> AstExpr<'static> {
    TEST_ARENA.with(|arena| {
        let arena_ref = arena.borrow();
        let arena_ptr = &*arena_ref as *const Bump;
        unsafe {
            let name_str = (*arena_ptr).alloc_str(name);
            AstExpr::Variable(name_str)
        }
    })
}

/// Create a function AST node
pub fn function(name: &str, args: Vec<AstExpr<'static>>) -> AstExpr<'static> {
    TEST_ARENA.with(|arena| {
        let arena_ref = arena.borrow();
        let arena_ptr = &*arena_ref as *const Bump;
        unsafe {
            let name_str = (*arena_ptr).alloc_str(name);
            let args_slice = (*arena_ptr).alloc_slice_fill_iter(args.into_iter());
            AstExpr::Function {
                name: name_str,
                args: args_slice,
            }
        }
    })
}

/// Create an array AST node
pub fn array(name: &str, index: AstExpr<'static>) -> AstExpr<'static> {
    TEST_ARENA.with(|arena| {
        let arena_ref = arena.borrow();
        let arena_ptr = &*arena_ref as *const Bump;
        unsafe {
            let name_str = (*arena_ptr).alloc_str(name);
            let index_ref = (*arena_ptr).alloc(index);
            AstExpr::Array {
                name: name_str,
                index: index_ref,
            }
        }
    })
}

/// Create an attribute AST node
pub fn attribute(base: &str, attr: &str) -> AstExpr<'static> {
    TEST_ARENA.with(|arena| {
        let arena_ref = arena.borrow();
        let arena_ptr = &*arena_ref as *const Bump;
        unsafe {
            let base_str = (*arena_ptr).alloc_str(base);
            let attr_str = (*arena_ptr).alloc_str(attr);
            AstExpr::Attribute {
                base: base_str,
                attr: attr_str,
            }
        }
    })
}

/// Create a logical operation AST node
pub fn logical_op(op: crate::types::LogicalOperator, left: AstExpr<'static>, right: AstExpr<'static>) -> AstExpr<'static> {
    TEST_ARENA.with(|arena| {
        let arena_ref = arena.borrow();
        let arena_ptr = &*arena_ref as *const Bump;
        unsafe {
            let left_ref = (*arena_ptr).alloc(left);
            let right_ref = (*arena_ptr).alloc(right);
            AstExpr::LogicalOp {
                op,
                left: left_ref,
                right: right_ref,
            }
        }
    })
}

/// Create a conditional AST node
pub fn conditional(condition: AstExpr<'static>, true_branch: AstExpr<'static>, false_branch: AstExpr<'static>) -> AstExpr<'static> {
    TEST_ARENA.with(|arena| {
        let arena_ref = arena.borrow();
        let arena_ptr = &*arena_ref as *const Bump;
        unsafe {
            let condition_ref = (*arena_ptr).alloc(condition);
            let true_ref = (*arena_ptr).alloc(true_branch);
            let false_ref = (*arena_ptr).alloc(false_branch);
            AstExpr::Conditional {
                condition: condition_ref,
                true_branch: true_ref,
                false_branch: false_ref,
            }
        }
    })
}

/// Parse an expression into the test arena
pub fn parse_test_expr(expr: &str) -> Result<AstExpr<'static>, crate::error::ExprError> {
    TEST_ARENA.with(|arena| {
        let arena_ref = arena.borrow();
        let arena_ptr = &*arena_ref as *const Bump;
        unsafe {
            crate::engine::parse_expression_arena(expr, &*arena_ptr)
        }
    })
}

/// For tests that need a parsed expression but don't care about error handling
pub fn parse(expr: &str) -> AstExpr<'static> {
    parse_test_expr(expr).unwrap()
}