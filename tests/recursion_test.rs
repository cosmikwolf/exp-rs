//! Comprehensive tests for recursive expression functions
//!
//! This test suite validates that the iterative evaluator correctly handles
//! recursive expression functions and enforces appropriate limits to prevent
//! stack overflow.

#[cfg(test)]
use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use exp_rs::error::ExprError;
use std::rc::Rc;

#[cfg(test)]
use proptest::prelude::*;

/// Test basic recursive functions work correctly
#[test]
fn test_recursive_functions_correctness() {
    let mut ctx = EvalContext::default();
    
    // Register operators for tests without libm
    #[cfg(not(feature = "libm"))]
    {
        ctx.register_native_function("<=", 2, |args| if args[0] <= args[1] { 1.0 } else { 0.0 });
        ctx.register_native_function("-", 2, |args| args[0] - args[1]);
        ctx.register_native_function("*", 2, |args| args[0] * args[1]);
        ctx.register_native_function("+", 2, |args| args[0] + args[1]);
    }
    
    // Test factorial
    ctx.register_expression_function(
        "factorial",
        &["n"],
        "n <= 1 ? 1 : n * factorial(n - 1)"
    ).unwrap();
    
    // Test known factorial values
    let test_cases = vec![
        (0, 1.0),
        (1, 1.0),
        (2, 2.0),
        (3, 6.0),
        (4, 24.0),
        (5, 120.0),
        (6, 720.0),
        (7, 5040.0),
        (8, 40320.0),
        (9, 362880.0),
        (10, 3628800.0),
    ];
    
    for (n, expected) in test_cases {
        let result = interp(&format!("factorial({})", n), Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(result, expected, "factorial({}) should equal {}", n, expected);
    }
}

/// Test fibonacci with memoization pattern
#[test]
fn test_fibonacci_recursion() {
    let mut ctx = EvalContext::default();
    
    #[cfg(not(feature = "libm"))]
    {
        ctx.register_native_function("<=", 2, |args| if args[0] <= args[1] { 1.0 } else { 0.0 });
        ctx.register_native_function("-", 2, |args| args[0] - args[1]);
        ctx.register_native_function("+", 2, |args| args[0] + args[1]);
    }
    
    // Simple recursive fibonacci
    ctx.register_expression_function(
        "fib",
        &["n"],
        "n <= 1 ? n : fib(n - 1) + fib(n - 2)"
    ).unwrap();
    
    // Test small fibonacci values
    let test_cases = vec![
        (0, 0.0),
        (1, 1.0),
        (2, 1.0),
        (3, 2.0),
        (4, 3.0),
        (5, 5.0),
        (6, 8.0),
        (7, 13.0),
        (8, 21.0),
    ];
    
    for (n, expected) in test_cases {
        let result = interp(&format!("fib({})", n), Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(result, expected, "fib({}) should equal {}", n, expected);
    }
}

/// Test mutual recursion
#[test]
fn test_mutual_recursion() {
    let mut ctx = EvalContext::default();
    
    #[cfg(not(feature = "libm"))]
    {
        ctx.register_native_function("==", 2, |args| if args[0] == args[1] { 1.0 } else { 0.0 });
        ctx.register_native_function("-", 2, |args| args[0] - args[1]);
    }
    
    // is_even and is_odd functions that call each other
    ctx.register_expression_function(
        "is_even",
        &["n"],
        "n == 0 ? 1 : is_odd(n - 1)"
    ).unwrap();
    
    ctx.register_expression_function(
        "is_odd",
        &["n"],
        "n == 0 ? 0 : is_even(n - 1)"
    ).unwrap();
    
    // Test mutual recursion
    for n in 0..10 {
        let even_result = interp(&format!("is_even({})", n), Some(Rc::new(ctx.clone()))).unwrap();
        let odd_result = interp(&format!("is_odd({})", n), Some(Rc::new(ctx.clone()))).unwrap();
        
        if n % 2 == 0 {
            assert_eq!(even_result, 1.0, "is_even({}) should be true", n);
            assert_eq!(odd_result, 0.0, "is_odd({}) should be false", n);
        } else {
            assert_eq!(even_result, 0.0, "is_even({}) should be false", n);
            assert_eq!(odd_result, 1.0, "is_odd({}) should be true", n);
        }
    }
}

/// Test recursion depth limits
#[test]
fn test_recursion_limits() {
    let mut ctx = EvalContext::default();
    
    #[cfg(not(feature = "libm"))]
    {
        ctx.register_native_function("<=", 2, |args| if args[0] <= args[1] { 1.0 } else { 0.0 });
        ctx.register_native_function("-", 2, |args| args[0] - args[1]);
        ctx.register_native_function("*", 2, |args| args[0] * args[1]);
        ctx.register_native_function("+", 2, |args| args[0] + args[1]);
    }
    
    // Linear recursion (factorial)
    ctx.register_expression_function(
        "factorial",
        &["n"],
        "n <= 1 ? 1 : n * factorial(n - 1)"
    ).unwrap();
    
    // Find the limit for linear recursion
    let mut linear_limit = 0;
    for n in 1..200 {
        match interp(&format!("factorial({})", n), Some(Rc::new(ctx.clone()))) {
            Ok(_) => linear_limit = n,
            Err(ExprError::CapacityExceeded(_)) => break,
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
    println!("Linear recursion limit: factorial({}) succeeds", linear_limit);
    assert!(linear_limit >= 10, "Should handle at least factorial(10)");
    
    // Exponential recursion (fibonacci)
    ctx.register_expression_function(
        "fib",
        &["n"],
        "n <= 1 ? n : fib(n - 1) + fib(n - 2)"
    ).unwrap();
    
    // Find the limit for exponential recursion
    let mut exp_limit = 0;
    for n in 1..30 {
        match interp(&format!("fib({})", n), Some(Rc::new(ctx.clone()))) {
            Ok(_) => exp_limit = n,
            Err(ExprError::CapacityExceeded(_)) => break,
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
    println!("Exponential recursion limit: fib({}) succeeds", exp_limit);
    assert!(exp_limit >= 5, "Should handle at least fib(5)");
    assert!(exp_limit < linear_limit, "Exponential recursion should hit limit sooner");
}

/// Property-based test for factorial limits
#[cfg(test)]
proptest! {
    #[test]
    fn prop_factorial_within_limits(n in 0u32..50) {
        let mut ctx = EvalContext::default();
        
        #[cfg(not(feature = "libm"))]
        {
            ctx.register_native_function("<=", 2, |args| if args[0] <= args[1] { 1.0 } else { 0.0 });
            ctx.register_native_function("-", 2, |args| args[0] - args[1]);
            ctx.register_native_function("*", 2, |args| args[0] * args[1]);
        }
        
        ctx.register_expression_function(
            "factorial",
            &["n"],
            "n <= 1 ? 1 : n * factorial(n - 1)"
        ).unwrap();
        
        match interp(&format!("factorial({})", n), Some(Rc::new(ctx))) {
            Ok(result) => {
                // Verify it's positive
                prop_assert!(result > 0.0);
                // For small values, verify correctness
                if n <= 10 {
                    let expected: f64 = (1..=n).map(|i| i as f64).product();
                    prop_assert_eq!(result, expected);
                }
            }
            Err(ExprError::CapacityExceeded(_)) => {
                // This is acceptable for large n
                prop_assert!(n > 10, "Small factorials should not exceed capacity");
            }
            Err(e) => {
                prop_assert!(false, "Unexpected error: {:?}", e);
            }
        }
    }
}

/// Property-based test for fibonacci limits  
#[cfg(test)]
proptest! {
    #[test]
    fn prop_fibonacci_behavior(n in 0u32..20) {
        let mut ctx = EvalContext::default();
        
        #[cfg(not(feature = "libm"))]
        {
            ctx.register_native_function("<=", 2, |args| if args[0] <= args[1] { 1.0 } else { 0.0 });
            ctx.register_native_function("-", 2, |args| args[0] - args[1]);
            ctx.register_native_function("+", 2, |args| args[0] + args[1]);
        }
        
        ctx.register_expression_function(
            "fib",
            &["n"],
            "n <= 1 ? n : fib(n - 1) + fib(n - 2)"
        ).unwrap();
        
        match interp(&format!("fib({})", n), Some(Rc::new(ctx))) {
            Ok(result) => {
                // Verify it's non-negative
                prop_assert!(result >= 0.0);
                // For small values, verify correctness
                if n <= 8 {
                    let expected = fibonacci_reference(n);
                    prop_assert_eq!(result, expected as f64);
                }
            }
            Err(ExprError::CapacityExceeded(_)) => {
                // This is expected for larger values due to exponential recursion
                prop_assert!(n >= 8, "Small fibonacci values should not exceed capacity");
            }
            Err(e) => {
                prop_assert!(false, "Unexpected error: {:?}", e);
            }
        }
    }
}

/// Reference implementation of fibonacci
fn fibonacci_reference(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci_reference(n - 1) + fibonacci_reference(n - 2),
    }
}

/// Test that infinite recursion is caught
#[test]
fn test_infinite_recursion_protection() {
    let mut ctx = EvalContext::default();
    
    // Register a function that calls itself without a base case
    ctx.register_expression_function(
        "infinite",
        &["n"],
        "infinite(n)"
    ).unwrap();
    
    // This should hit capacity limit
    match interp("infinite(1)", Some(Rc::new(ctx))) {
        Err(ExprError::CapacityExceeded(resource)) => {
            assert_eq!(resource, "context stack");
        }
        Ok(_) => panic!("Infinite recursion should fail"),
        Err(e) => panic!("Expected CapacityExceeded, got: {:?}", e),
    }
}