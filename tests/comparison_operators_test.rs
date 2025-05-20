//! Tests for comparison operators in expressions
//! 
//! These tests verify that comparison operators (<, >, <=, >=, ==, !=) function correctly
//! in the expression evaluator.

extern crate exp_rs;
use exp_rs::engine::interp;
use exp_rs::Real;

#[test]
fn test_equal_operator() {
    // Test equality
    assert_eq!(interp("1 == 1", None).unwrap(), 1.0); // Equal values
    assert_eq!(interp("1 == 2", None).unwrap(), 0.0); // Unequal values
    assert_eq!(interp("1 == 1.0", None).unwrap(), 1.0); // Equal with different representations
    assert_eq!(interp("0.1 + 0.2 == 0.3", None).unwrap(), 0.0); // Floating point precision
}

#[test]
fn test_not_equal_operator() {
    // Test inequality with !=
    assert_eq!(interp("1 != 1", None).unwrap(), 0.0); // Equal values
    assert_eq!(interp("1 != 2", None).unwrap(), 1.0); // Unequal values

    // Test alternative not-equal syntax with <>
    assert_eq!(interp("1 <> 1", None).unwrap(), 0.0); // Equal values
    assert_eq!(interp("1 <> 2", None).unwrap(), 1.0); // Unequal values
}

#[test]
fn test_less_than_operator() {
    // Test less than
    assert_eq!(interp("1 < 2", None).unwrap(), 1.0); // True
    assert_eq!(interp("2 < 1", None).unwrap(), 0.0); // False
    assert_eq!(interp("1 < 1", None).unwrap(), 0.0); // False (equal)
}

#[test]
fn test_greater_than_operator() {
    // Test greater than
    assert_eq!(interp("2 > 1", None).unwrap(), 1.0); // True
    assert_eq!(interp("1 > 2", None).unwrap(), 0.0); // False
    assert_eq!(interp("1 > 1", None).unwrap(), 0.0); // False (equal)
}

#[test]
fn test_less_than_or_equal_operator() {
    // Test less than or equal
    assert_eq!(interp("1 <= 2", None).unwrap(), 1.0); // True (less)
    assert_eq!(interp("1 <= 1", None).unwrap(), 1.0); // True (equal)
    assert_eq!(interp("2 <= 1", None).unwrap(), 0.0); // False
}

#[test]
fn test_greater_than_or_equal_operator() {
    // Test greater than or equal
    assert_eq!(interp("2 >= 1", None).unwrap(), 1.0); // True (greater)
    assert_eq!(interp("1 >= 1", None).unwrap(), 1.0); // True (equal)
    assert_eq!(interp("1 >= 2", None).unwrap(), 0.0); // False
}

#[test]
fn test_comparison_with_expressions() {
    // Test comparison operators with expressions on both sides
    assert_eq!(interp("(1 + 2) == 3", None).unwrap(), 1.0);
    assert_eq!(interp("(2 * 3) != (5 + 2)", None).unwrap(), 1.0);
    assert_eq!(interp("(3 * 4) > (10 + 1)", None).unwrap(), 1.0);
    assert_eq!(interp("(5 - 2) < (10 / 2)", None).unwrap(), 1.0);
    assert_eq!(interp("(10 / 2) >= (6 - 1)", None).unwrap(), 1.0);
    assert_eq!(interp("(4 ^ 2) <= (4 * 4)", None).unwrap(), 1.0);
}

#[test]
fn test_comparison_operator_precedence() {
    // Test that comparison operators have the correct precedence
    // Comparison should be lower than arithmetic but higher than logical operators
    assert_eq!(interp("1 + 2 == 3 + 0", None).unwrap(), 1.0); // (1+2) == (3+0)
    assert_eq!(interp("1 < 2 + 3", None).unwrap(), 1.0);      // 1 < (2+3)
    assert_eq!(interp("5 > 2 * 2", None).unwrap(), 1.0);      // 5 > (2*2)
}

#[test]
fn test_chained_comparisons() {
    // Test chained comparison expressions 
    // Note: This is not the same as a && b in other languages
    // These are parsed as (a < b) < c which is different behavior
    let result = interp("(1 < 2) == 1", None).unwrap();
    assert_eq!(result, 1.0);
    
    let result = interp("(5 > 3) == 1", None).unwrap();
    assert_eq!(result, 1.0);
}

#[test]
fn test_comparison_with_variables() {
    use exp_rs::context::EvalContext;
    use std::rc::Rc;
    
    // Create a context with variables
    let mut ctx = EvalContext::new();
    ctx.set_parameter("x", 10.0);
    ctx.set_parameter("y", 5.0);
    
    let ctx_rc = Rc::new(ctx);
    
    // Test comparisons with variables
    assert_eq!(interp("x > y", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("x < y", Some(ctx_rc.clone())).unwrap(), 0.0);
    assert_eq!(interp("x == 10", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("y != 10", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("x >= y * 2", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("y <= x / 2", Some(ctx_rc.clone())).unwrap(), 1.0);
    
    // Test with more complex expressions
    assert_eq!(interp("(x + y) > (x - y)", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("(x * y) == 50", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("(x / y) == 2", Some(ctx_rc)).unwrap(), 1.0);
}

#[test]
fn test_comparison_with_functions() {
    // Test that comparison operators work with function results
    assert_eq!(interp("sin(0) == 0", None).unwrap(), 1.0);
    assert_eq!(interp("cos(0) == 1", None).unwrap(), 1.0);
    assert_eq!(interp("sqrt(4) == 2", None).unwrap(), 1.0);
    assert_eq!(interp("abs(-5) > 0", None).unwrap(), 1.0);
    assert_eq!(interp("pow(2, 3) == 8", None).unwrap(), 1.0);
}

#[test]
fn test_registered_comparison_function() {
    use exp_rs::context::EvalContext;
    use std::rc::Rc;
    
    // Create a context
    let mut ctx = EvalContext::new();
    
    // Register a function that uses comparison operators
    ctx.register_expression_function("is_positive", &["x"], "x > 0").unwrap();
    
    // Use a workaround for logical AND since we haven't implemented it yet
    // We can use the product of two boolean results (1.0 * 1.0 = 1.0, 0.0 * anything = 0.0)
    ctx.register_expression_function("is_between", &["x", "min", "max"], "(x >= min) * (x <= max)").unwrap();
    
    let ctx_rc = Rc::new(ctx);
    
    // Test the comparison functions
    assert_eq!(interp("is_positive(5)", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("is_positive(-5)", Some(ctx_rc.clone())).unwrap(), 0.0);
    assert_eq!(interp("is_positive(0)", Some(ctx_rc.clone())).unwrap(), 0.0);
    
    // Test the is_between function (requires logical AND which is not implemented yet)
    // This test will fail until logical operators are implemented
    // Uncomment this when logical operators are added
    // assert_eq!(interp("is_between(5, 1, 10)", Some(ctx_rc.clone())).unwrap(), 1.0);
    // assert_eq!(interp("is_between(0, 1, 10)", Some(ctx_rc.clone())).unwrap(), 0.0);
}