//! Tests for the iterative evaluator
//!
//! This test suite verifies that the iterative evaluator produces
//! identical results to the recursive evaluator while handling
//! deeply nested expressions without stack overflow.

#[cfg(test)]
use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use exp_rs::eval::iterative::eval_iterative;
use exp_rs::engine::parse_expression;
use exp_rs::Real;
use std::rc::Rc;

#[path = "test_helpers.rs"]
mod test_helpers;
use test_helpers::{hstr, set_var, set_const, set_attr};

// Precision constant for floating point comparisons
const EPSILON: Real = if cfg!(feature = "f32") { 1e-6 } else { 1e-10 };

/// Test basic arithmetic operations
#[test]
fn test_iterative_basic_arithmetic() {
    let ctx = EvalContext::default();
    let ctx_rc = Some(Rc::new(ctx));
    
    // Test cases with expected results
    let test_cases = vec![
        ("2 + 3", 5.0),
        ("10 - 4", 6.0),
        ("3 * 4", 12.0),
        ("15 / 3", 5.0),
        ("17 % 5", 2.0),
        ("2 ^ 3", 8.0),
        ("-5", -5.0),
        ("-(3 + 2)", -5.0),
    ];
    
    for (expr_str, expected) in test_cases {
        let ast = parse_expression(expr_str).unwrap();
        let result = eval_iterative(&ast, ctx_rc.clone()).unwrap();
        assert_eq!(result, expected, "Failed for expression: {}", expr_str);
    }
}

/// Test comparison and logical operators
#[test]
fn test_iterative_comparison_logical() {
    let ctx = EvalContext::default();
    let ctx_rc = Some(Rc::new(ctx));
    
    let test_cases = vec![
        ("5 > 3", 1.0),
        ("3 > 5", 0.0),
        ("5 >= 5", 1.0),
        ("4 < 6", 1.0),
        ("6 <= 6", 1.0),
        ("5 == 5", 1.0),
        ("5 != 3", 1.0),
        ("1 && 1", 1.0),
        ("1 && 0", 0.0),
        ("0 || 1", 1.0),
        ("0 || 0", 0.0),
    ];
    
    for (expr_str, expected) in test_cases {
        let ast = parse_expression(expr_str).unwrap();
        let result = eval_iterative(&ast, ctx_rc.clone()).unwrap();
        assert_eq!(result, expected, "Failed for expression: {}", expr_str);
    }
}

/// Test short-circuit evaluation
#[test]
fn test_iterative_short_circuit() {
    let mut ctx = EvalContext::default();
    
    // Counter to track if function was called
    let counter = std::rc::Rc::new(std::cell::RefCell::new(0));
    let counter_clone = counter.clone();
    
    ctx.register_native_function("side_effect", 0, move |_| {
        *counter_clone.borrow_mut() += 1;
        1.0
    }).unwrap();
    
    let ctx_rc = Some(Rc::new(ctx));
    
    // Test AND short-circuit (should not call side_effect)
    *counter.borrow_mut() = 0;
    let ast = parse_expression("0 && side_effect()").unwrap();
    let result = eval_iterative(&ast, ctx_rc.clone()).unwrap();
    assert_eq!(result, 0.0);
    assert_eq!(*counter.borrow(), 0, "side_effect should not be called");
    
    // Test OR short-circuit (should not call side_effect)
    *counter.borrow_mut() = 0;
    let ast = parse_expression("1 || side_effect()").unwrap();
    let result = eval_iterative(&ast, ctx_rc.clone()).unwrap();
    assert_eq!(result, 1.0);
    assert_eq!(*counter.borrow(), 0, "side_effect should not be called");
    
    // Test non-short-circuit cases
    *counter.borrow_mut() = 0;
    let ast = parse_expression("1 && side_effect()").unwrap();
    let result = eval_iterative(&ast, ctx_rc.clone()).unwrap();
    assert_eq!(result, 1.0);
    assert_eq!(*counter.borrow(), 1, "side_effect should be called");
}

/// Test ternary operator
#[test]
fn test_iterative_ternary() {
    let ctx = EvalContext::default();
    let ctx_rc = Some(Rc::new(ctx));
    
    let test_cases = vec![
        ("1 ? 10 : 20", 10.0),
        ("0 ? 10 : 20", 20.0),
        ("5 > 3 ? 100 : 200", 100.0),
        ("2 < 1 ? 100 : 200", 200.0),
    ];
    
    for (expr_str, expected) in test_cases {
        let ast = parse_expression(expr_str).unwrap();
        let result = eval_iterative(&ast, ctx_rc.clone()).unwrap();
        assert_eq!(result, expected, "Failed for expression: {}", expr_str);
    }
}

/// Test variable lookup
#[test]
fn test_iterative_variables() {
    let mut ctx = EvalContext::default();
    set_var(&mut ctx, "x", 10.0);
    set_var(&mut ctx, "y", 20.0);
    set_const(&mut ctx, "z", 30.0);
    
    let ctx_rc = Some(Rc::new(ctx));
    
    let test_cases = vec![
        ("x", 10.0),
        ("y", 20.0),
        ("z", 30.0),
        ("x + y", 30.0),
        ("x * 2 + y", 40.0),
        ("pi", core::f64::consts::PI as Real),
        ("e", core::f64::consts::E as Real),
    ];
    
    for (expr_str, expected) in test_cases {
        let ast = parse_expression(expr_str).unwrap();
        let result = eval_iterative(&ast, ctx_rc.clone()).unwrap();
        assert!((result - expected).abs() < EPSILON, 
                "Failed for expression: {} (got {}, expected {})", 
                expr_str, result, expected);
    }
}

/// Test function calls
#[test]
fn test_iterative_functions() {
    let mut ctx = EvalContext::default();
    
    // Register some native functions
    ctx.register_native_function("double", 1, |args| args[0] * 2.0).unwrap();
    ctx.register_native_function("add3", 3, |args| args[0] + args[1] + args[2]).unwrap();
    
    let ctx_rc = Some(Rc::new(ctx));
    
    let test_cases = vec![
        ("sin(0)", 0.0),
        ("cos(0)", 1.0),
        ("abs(-5)", 5.0),
        ("max(3, 7)", 7.0),
        ("min(3, 7)", 3.0),
        ("sqrt(16)", 4.0),
        ("double(21)", 42.0),
        ("add3(1, 2, 3)", 6.0),
    ];
    
    for (expr_str, expected) in test_cases {
        let ast = parse_expression(expr_str).unwrap();
        let result = eval_iterative(&ast, ctx_rc.clone()).unwrap();
        assert!((result - expected).abs() < EPSILON, 
                "Failed for expression: {} (got {}, expected {})", 
                expr_str, result, expected);
    }
}

/// Test expression functions (user-defined functions)
#[test]
fn test_iterative_expression_functions() {
    let mut ctx = EvalContext::default();
    
    // Define some expression functions
    ctx.register_expression_function("square", &["x"], "x * x").unwrap();
    ctx.register_expression_function("sum3", &["a", "b", "c"], "a + b + c").unwrap();
    ctx.register_expression_function("factorial", &["n"], 
        "n <= 1 ? 1 : n * factorial(n - 1)").unwrap();
    
    let ctx_rc = Some(Rc::new(ctx));
    
    let test_cases = vec![
        ("square(5)", 25.0),
        ("sum3(10, 20, 30)", 60.0),
        ("factorial(5)", 120.0),
        ("square(square(2))", 16.0),
    ];
    
    for (expr_str, expected) in test_cases {
        let ast = parse_expression(expr_str).unwrap();
        let result = eval_iterative(&ast, ctx_rc.clone()).unwrap();
        assert_eq!(result, expected, "Failed for expression: {}", expr_str);
    }
}

/// Test deeply nested expressions that would overflow with recursive evaluation
#[test]
fn test_iterative_deep_nesting() {
    let ctx = EvalContext::default();
    let ctx_rc = Some(Rc::new(ctx));
    
    // Create a deeply nested expression: ((((1 + 1) + 1) + 1) + ...)
    let mut expr = String::from("1");
    for _ in 0..100 {
        expr = format!("({} + 1)", expr);
    }
    
    let ast = parse_expression(&expr).unwrap();
    let result = eval_iterative(&ast, ctx_rc.clone()).unwrap();
    assert_eq!(result, 101.0, "Failed for deeply nested expression");
    
    // Create a deeply nested multiplication
    let mut expr = String::from("2");
    for _ in 0..10 {
        expr = format!("({} * 2)", expr);
    }
    
    let ast = parse_expression(&expr).unwrap();
    let result = eval_iterative(&ast, ctx_rc).unwrap();
    assert_eq!(result, 2048.0, "Failed for deeply nested multiplication");
}

/// Test array access
#[test]
fn test_iterative_array_access() {
    let mut ctx = EvalContext::default();
    
    // Create an array
    ctx.arrays.insert(hstr("data"), Default::default()).unwrap();
    ctx.arrays.get_mut(&hstr("data")).unwrap().insert(0, 10.0);
    ctx.arrays.get_mut(&hstr("data")).unwrap().insert(1, 20.0);
    ctx.arrays.get_mut(&hstr("data")).unwrap().insert(2, 30.0);
    
    let ctx_rc = Some(Rc::new(ctx));
    
    let test_cases = vec![
        ("data[0]", 10.0),
        ("data[1]", 20.0),
        ("data[2]", 30.0),
        ("data[1] + data[2]", 50.0),
        ("data[0] * 2", 20.0),
    ];
    
    for (expr_str, expected) in test_cases {
        let ast = parse_expression(expr_str).unwrap();
        let result = eval_iterative(&ast, ctx_rc.clone()).unwrap();
        assert_eq!(result, expected, "Failed for expression: {}", expr_str);
    }
}

/// Test attribute access
#[test]
fn test_iterative_attribute_access() {
    let mut ctx = EvalContext::default();
    
    // Set some attributes
    set_attr(&mut ctx, "player", "health", 100.0);
    set_attr(&mut ctx, "player", "mana", 50.0);
    set_attr(&mut ctx, "enemy", "health", 75.0);
    
    let ctx_rc = Some(Rc::new(ctx));
    
    let test_cases = vec![
        ("player.health", 100.0),
        ("player.mana", 50.0),
        ("enemy.health", 75.0),
        ("player.health - enemy.health", 25.0),
    ];
    
    for (expr_str, expected) in test_cases {
        let ast = parse_expression(expr_str).unwrap();
        let result = eval_iterative(&ast, ctx_rc.clone()).unwrap();
        assert_eq!(result, expected, "Failed for expression: {}", expr_str);
    }
}

/// Test error handling
#[test]
fn test_iterative_error_handling() {
    let ctx = EvalContext::default();
    let ctx_rc = Some(Rc::new(ctx));
    
    // Test undefined variable
    let ast = parse_expression("undefined_var").unwrap();
    assert!(eval_iterative(&ast, ctx_rc.clone()).is_err());
    
    // Test undefined function
    let ast = parse_expression("undefined_func()").unwrap();
    assert!(eval_iterative(&ast, ctx_rc.clone()).is_err());
    
    // Test array out of bounds
    let ast = parse_expression("data[999]").unwrap();
    assert!(eval_iterative(&ast, ctx_rc.clone()).is_err());
    
    // Test undefined attribute
    let ast = parse_expression("obj.undefined_attr").unwrap();
    assert!(eval_iterative(&ast, ctx_rc).is_err());
}

/// Compare results between recursive and iterative evaluators
#[test]
fn test_iterative_matches_recursive() {
    let mut ctx = EvalContext::default();
    
    // Set up a complex context
    set_var(&mut ctx, "x", 5.0);
    set_var(&mut ctx, "y", 10.0);
    set_const(&mut ctx, "k", 2.0);
    
    ctx.register_native_function("f", 1, |args| args[0] * args[0]).unwrap();
    ctx.register_expression_function("g", &["a", "b"], "a + b * 2").unwrap();
    
    let ctx_rc = Some(Rc::new(ctx));
    
    // Test various expressions
    let test_expressions = vec![
        "x + y * k",
        "sin(x) + cos(y)",
        "x > 3 ? f(x) : g(x, y)",
        "1 && (x > 0 || y < 0)",
        "g(f(2), 3) + k",
        "max(x, y) * min(x, y)",
    ];
    
    for expr_str in test_expressions {
        let ast = parse_expression(expr_str).unwrap();
        
        // Get result from iterative evaluator
        let iter_result = eval_iterative(&ast, ctx_rc.clone()).unwrap();
        
        // Get result from recursive evaluator (via interp)
        let rec_result = interp(expr_str, ctx_rc.clone()).unwrap();
        
        assert!((iter_result - rec_result).abs() < EPSILON,
                "Results differ for {}: iterative={}, recursive={}", 
                expr_str, iter_result, rec_result);
    }
}