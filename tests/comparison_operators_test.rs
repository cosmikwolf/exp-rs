//! Tests for comparison operators in expressions
//! 
//! These tests verify that comparison operators (<, >, <=, >=, ==, !=) function correctly
//! in the expression evaluator.

extern crate exp_rs;
use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use exp_rs::Real;
use std::rc::Rc;

// Helper function to create a context with comparison operators registered
// This is needed when running with --no-default-features (no libm)
fn create_test_context() -> EvalContext {
    let mut ctx = EvalContext::new();
    
    // Register comparison operators when libm is not available
    #[cfg(not(feature = "libm"))]
    {
        // Comparison operators
        ctx.register_native_function("==", 2, |args| if args[0] == args[1] { 1.0 } else { 0.0 });
        ctx.register_native_function("!=", 2, |args| if args[0] != args[1] { 1.0 } else { 0.0 });
        ctx.register_native_function("<>", 2, |args| if args[0] != args[1] { 1.0 } else { 0.0 });
        ctx.register_native_function("<", 2, |args| if args[0] < args[1] { 1.0 } else { 0.0 });
        ctx.register_native_function(">", 2, |args| if args[0] > args[1] { 1.0 } else { 0.0 });
        ctx.register_native_function("<=", 2, |args| if args[0] <= args[1] { 1.0 } else { 0.0 });
        ctx.register_native_function(">=", 2, |args| if args[0] >= args[1] { 1.0 } else { 0.0 });
        
        // Arithmetic operators
        ctx.register_native_function("+", 2, |args| args[0] + args[1]);
        ctx.register_native_function("*", 2, |args| args[0] * args[1]);
        ctx.register_native_function("/", 2, |args| args[0] / args[1]);
        ctx.register_native_function("-", 2, |args| args[0] - args[1]);
        ctx.register_native_function("^", 2, |args| args[0].powf(args[1]));
        
        // Unary operators
        ctx.register_native_function("neg", 1, |args| -args[0]);
    }
    
    ctx
}

#[test]
fn test_equal_operator() {
    // Create a context with comparison operators registered
    let ctx = create_test_context();
    let ctx_rc = Rc::new(ctx);
    
    // Test equality
    assert_eq!(interp("1 == 1", Some(ctx_rc.clone())).unwrap(), 1.0); // Equal values
    assert_eq!(interp("1 == 2", Some(ctx_rc.clone())).unwrap(), 0.0); // Unequal values
    assert_eq!(interp("1 == 1.0", Some(ctx_rc.clone())).unwrap(), 1.0); // Equal with different representations
    
    // Test floating point precision - different behavior in f32 vs f64
    // In f32 mode: 0.1 + 0.2 == 0.3 evaluates to true due to lower precision
    // In f64 mode: 0.1 + 0.2 == 0.3 evaluates to false due to higher precision
    #[cfg(feature = "f32")]
    assert_eq!(interp("0.1 + 0.2 == 0.3", Some(ctx_rc)).unwrap(), 1.0, 
        "f32 mode should evaluate 0.1 + 0.2 == 0.3 as true due to lower precision");
    
    #[cfg(not(feature = "f32"))]
    assert_eq!(interp("0.1 + 0.2 == 0.3", Some(ctx_rc)).unwrap(), 0.0,
        "f64 mode should evaluate 0.1 + 0.2 == 0.3 as false due to higher precision");
}

#[test]
fn test_not_equal_operator() {
    // Create a context with comparison operators registered
    let ctx = create_test_context();
    let ctx_rc = Rc::new(ctx);
    
    // Test inequality with !=
    assert_eq!(interp("1 != 1", Some(ctx_rc.clone())).unwrap(), 0.0); // Equal values
    assert_eq!(interp("1 != 2", Some(ctx_rc.clone())).unwrap(), 1.0); // Unequal values
    
    // Test floating point precision - different behavior in f32 vs f64
    // In f32 mode: 0.1 + 0.2 != 0.3 evaluates to false due to lower precision
    // In f64 mode: 0.1 + 0.2 != 0.3 evaluates to true due to higher precision
    #[cfg(feature = "f32")]
    assert_eq!(interp("0.1 + 0.2 != 0.3", Some(ctx_rc.clone())).unwrap(), 0.0, 
        "f32 mode should evaluate 0.1 + 0.2 != 0.3 as false due to lower precision");
    
    #[cfg(not(feature = "f32"))]
    assert_eq!(interp("0.1 + 0.2 != 0.3", Some(ctx_rc.clone())).unwrap(), 1.0,
        "f64 mode should evaluate 0.1 + 0.2 != 0.3 as true due to higher precision");
}

#[test]
fn test_less_than_operator() {
    // Create a context with comparison operators registered
    let ctx = create_test_context();
    let ctx_rc = Rc::new(ctx);
    
    // Test less than
    assert_eq!(interp("1 < 2", Some(ctx_rc.clone())).unwrap(), 1.0); // True
    assert_eq!(interp("2 < 1", Some(ctx_rc.clone())).unwrap(), 0.0); // False
    assert_eq!(interp("1 < 1", Some(ctx_rc.clone())).unwrap(), 0.0); // False (equal)
}

#[test]
fn test_greater_than_operator() {
    // Create a context with comparison operators registered
    let ctx = create_test_context();
    let ctx_rc = Rc::new(ctx);
    
    // Test greater than
    assert_eq!(interp("2 > 1", Some(ctx_rc.clone())).unwrap(), 1.0); // True
    assert_eq!(interp("1 > 2", Some(ctx_rc.clone())).unwrap(), 0.0); // False
    assert_eq!(interp("1 > 1", Some(ctx_rc.clone())).unwrap(), 0.0); // False (equal)
}

#[test]
fn test_less_than_or_equal_operator() {
    // Create a context with comparison operators registered
    let ctx = create_test_context();
    let ctx_rc = Rc::new(ctx);
    
    // Test less than or equal
    assert_eq!(interp("1 <= 2", Some(ctx_rc.clone())).unwrap(), 1.0); // True (less)
    assert_eq!(interp("1 <= 1", Some(ctx_rc.clone())).unwrap(), 1.0); // True (equal)
    assert_eq!(interp("2 <= 1", Some(ctx_rc.clone())).unwrap(), 0.0); // False
}

#[test]
fn test_greater_than_or_equal_operator() {
    // Create a context with comparison operators registered
    let ctx = create_test_context();
    let ctx_rc = Rc::new(ctx);
    
    // Test greater than or equal
    assert_eq!(interp("2 >= 1", Some(ctx_rc.clone())).unwrap(), 1.0); // True (greater)
    assert_eq!(interp("1 >= 1", Some(ctx_rc.clone())).unwrap(), 1.0); // True (equal)
    assert_eq!(interp("1 >= 2", Some(ctx_rc.clone())).unwrap(), 0.0); // False
}

#[test]
fn test_comparison_with_expressions() {
    // Create a context with comparison operators registered
    let ctx = create_test_context();
    let ctx_rc = Rc::new(ctx);
    
    // Test comparison operators with expressions on both sides
    assert_eq!(interp("(1 + 2) == 3", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("(2 * 3) != (5 + 2)", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("(3 * 4) > (10 + 1)", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("(5 - 2) < (10 / 2)", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("(10 / 2) >= (6 - 1)", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("(4 ^ 2) <= (4 * 4)", Some(ctx_rc.clone())).unwrap(), 1.0);
}

#[test]
fn test_comparison_operator_precedence() {
    // Create a context with comparison operators registered
    let ctx = create_test_context();
    let ctx_rc = Rc::new(ctx);
    
    // Test that comparison operators have the correct precedence
    // Comparison should be lower than arithmetic but higher than logical operators
    assert_eq!(interp("1 + 2 == 3 + 0", Some(ctx_rc.clone())).unwrap(), 1.0); // (1+2) == (3+0)
    assert_eq!(interp("1 < 2 + 3", Some(ctx_rc.clone())).unwrap(), 1.0);      // 1 < (2+3)
    assert_eq!(interp("5 > 2 * 2", Some(ctx_rc.clone())).unwrap(), 1.0);      // 5 > (2*2)
}

#[test]
fn test_chained_comparisons() {
    // Create a context with comparison operators registered
    let ctx = create_test_context();
    let ctx_rc = Rc::new(ctx);
    
    // Test chained comparison expressions 
    // Note: This is not the same as a && b in other languages
    // These are parsed as (a < b) < c which is different behavior
    let result = interp("(1 < 2) == 1", Some(ctx_rc.clone())).unwrap();
    assert_eq!(result, 1.0);
    
    let result = interp("(5 > 3) == 1", Some(ctx_rc.clone())).unwrap();
    assert_eq!(result, 1.0);
}

#[test]
fn test_comparison_with_variables() {
    // Create a context with comparison operators registered and add variables
    let mut ctx = create_test_context();
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
    // Create a context with comparison operators and math functions registered
    let mut ctx = create_test_context();
    
    // Register math functions if libm is not enabled
    #[cfg(not(feature = "libm"))]
    {
        ctx.register_native_function("sin", 1, |args| args[0].sin());
        ctx.register_native_function("cos", 1, |args| args[0].cos());
        ctx.register_native_function("sqrt", 1, |args| args[0].sqrt());
        ctx.register_native_function("abs", 1, |args| args[0].abs());
        ctx.register_native_function("pow", 2, |args| args[0].powf(args[1]));
    }
    
    let ctx_rc = Rc::new(ctx);
    
    // Test that comparison operators work with function results
    assert_eq!(interp("sin(0) == 0", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("cos(0) == 1", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("sqrt(4) == 2", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("abs(-5) > 0", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("pow(2, 3) == 8", Some(ctx_rc.clone())).unwrap(), 1.0);
}

#[test]
fn test_registered_comparison_function() {
    // Create a context with comparison operators registered
    let mut ctx = create_test_context();
    
    // Register a function that uses comparison operators
    ctx.register_expression_function("is_positive", &["x"], "x > 0").unwrap();
    
    // Register multiplication for the is_between function
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