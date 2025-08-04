//! Property-based tests for expression functions with arena allocation
//!
//! This consolidates the many individual tests from expression_function_arena_test.rs
//! into comprehensive property-based tests.

#[cfg(test)]
use bumpalo::Bump;
use exp_rs::expression::ArenaBatchBuilder;
use exp_rs::{EvalContext, Real};
use proptest::prelude::*;
use std::rc::Rc;

/// Generate valid parameter names
fn param_name_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,7}".prop_map(|s| s.to_string())
}

/// Generate simple arithmetic expressions
fn arithmetic_expr_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("x".to_string()),
        Just("x + 1".to_string()),
        Just("x * 2".to_string()),
        Just("x * x".to_string()),
        Just("x + y".to_string()),
        Just("x * y + z".to_string()),
    ]
}

/// Generate function names
fn function_name_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,15}".prop_map(|s| s.to_string())
}

proptest! {
    /// Test that expression functions work correctly with various expressions
    #[test]
    fn prop_expression_function_evaluation(
        func_name in function_name_strategy(),
        param_name in param_name_strategy(),
        x in -100.0..100.0f64,
    ) {
        let arena = Bump::new();
        let mut builder = ArenaBatchBuilder::new(&arena);
        let mut ctx = EvalContext::new();
        
        // Register some basic functions
        ctx.register_expression_function(&func_name, &[&param_name], &format!("{} * 2", param_name))
            .unwrap();
        
        // Add expression
        let expr = format!("{}({})", func_name, x);
        builder.add_expression(&expr).unwrap();
        
        // Evaluate
        let ctx_rc = Rc::new(ctx);
        builder.eval(&ctx_rc).unwrap();
        
        // Check result
        let result = builder.get_result(0).unwrap();
        prop_assert_eq!(result, x * 2.0);
    }

    /// Test nested expression functions
    #[test]
    fn prop_nested_expression_functions(
        x in -50.0..50.0f64,
        y in -50.0..50.0f64,
    ) {
        let arena = Bump::new();
        let mut builder = ArenaBatchBuilder::new(&arena);
        let mut ctx = EvalContext::new();
        
        // Register functions that compose
        ctx.register_expression_function("add_one", &["x"], "x + 1").unwrap();
        ctx.register_expression_function("double", &["x"], "x * 2").unwrap();
        ctx.register_expression_function("compose", &["x"], "double(add_one(x))").unwrap();
        ctx.register_expression_function("f", &["x", "y"], "x + y").unwrap();
        ctx.register_expression_function("g", &["x"], "f(x, x)").unwrap();
        
        // Test various compositions
        builder.add_expression(&format!("compose({})", x)).unwrap();
        builder.add_expression(&format!("g({})", y)).unwrap();
        
        let ctx_rc = Rc::new(ctx);
        builder.eval(&ctx_rc).unwrap();
        
        prop_assert_eq!(builder.get_result(0).unwrap(), (x + 1.0) * 2.0);
        prop_assert_eq!(builder.get_result(1).unwrap(), y + y);
    }

    /// Test parameter validation
    #[test]
    fn prop_parameter_count_validation(
        params in prop::collection::vec(param_name_strategy(), 0..5),
        args in prop::collection::vec(-100.0..100.0f64, 0..5),
    ) {
        // Skip if we have duplicate parameter names
        let unique_params: Vec<_> = params.iter().collect::<std::collections::HashSet<_>>().into_iter().cloned().collect();
        if unique_params.len() != params.len() {
            return Ok(());
        }
        
        let arena = Bump::new();
        let mut builder = ArenaBatchBuilder::new(&arena);
        let mut ctx = EvalContext::new();
        
        if params.is_empty() {
            // Zero-parameter function
            ctx.register_expression_function("constant", &[], "42").unwrap();
            builder.add_expression("constant()").unwrap();
            let ctx_rc = Rc::new(ctx);
            builder.eval(&ctx_rc).unwrap();
            prop_assert_eq!(builder.get_result(0).unwrap(), 42.0);
        } else {
            // Create parameter list and expression
            let param_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();
            let expr = params.join(" + ");
            ctx.register_expression_function("sum", &param_refs, &expr).unwrap();
            
            // Build function call with the actual args we have
            let args_str = args.iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            
            // Only test if we have a valid expression to test
            if !args_str.is_empty() {
                builder.add_expression(&format!("sum({})", args_str)).unwrap();
                let ctx_rc = Rc::new(ctx);
                let result = builder.eval(&ctx_rc);
                
                if args.len() == params.len() {
                    // Correct number of arguments - should succeed
                    prop_assert!(result.is_ok());
                    let expected: f64 = args.iter().sum();
                    let actual = builder.get_result(0).unwrap();
                    prop_assert!((actual - expected).abs() < 1e-10);
                } else {
                    // Wrong number of arguments - should fail
                    prop_assert!(result.is_err(), "Expected error for {} params with {} args", params.len(), args.len());
                }
            }
        }
    }

    /// Test expression validation
    #[test] 
    fn prop_expression_validation(
        valid_expr in arithmetic_expr_strategy(),
        param_count in 1..4usize,
    ) {
        let mut ctx = EvalContext::new();
        
        // Generate parameter names
        let params: Vec<String> = (0..param_count)
            .map(|i| format!("p{}", i))
            .collect();
        let param_refs: Vec<&str> = params.iter().map(|s| s.as_str()).collect();
        
        // Valid expression should register successfully
        let result = ctx.register_expression_function("test_func", &param_refs, &valid_expr);
        prop_assert!(result.is_ok());
        
        // Invalid expressions
        let invalid_exprs = vec![
            "x +",           // Incomplete
            "sin(",          // Unclosed paren
            "x ) + 1",       // Extra paren
            "x x",           // Missing operator
        ];
        
        for invalid in invalid_exprs {
            let result = ctx.register_expression_function_validated(
                "invalid_func", 
                &param_refs, 
                invalid,
                false
            );
            match result {
                Ok(report) => prop_assert!(!report.syntax_valid),
                Err(_) => {} // Also acceptable
            }
        }
    }

    /// Test function name validation  
    #[test]
    fn prop_function_name_validation(
        name in "([a-zA-Z][a-zA-Z0-9_]*)?", // Optional: empty or starts with letter
        starts_with_digit in any::<bool>(),
    ) {
        let mut ctx = EvalContext::new();
        
        let test_name = if starts_with_digit && !name.is_empty() {
            format!("1{}", name)
        } else {
            name.clone()
        };
        
        let result = ctx.register_expression_function(&test_name, &["x"], "x + 1");
        
        // Check various name validation rules
        if test_name.len() > 32 {
            // Names that are too long should fail
            prop_assert!(result.is_err(), "Function name longer than 32 chars should fail");
        } else if test_name.is_empty() {
            // Empty names might be allowed - check actual behavior
            // If it succeeds, that's fine; if it fails, that's also fine
            // Just ensure it doesn't panic
        } else if test_name.chars().next().map_or(false, |c| c.is_numeric()) {
            // Names starting with digit might fail during parsing
            // Again, just ensure no panic
        } else if test_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            // Valid names should generally succeed
            if result.is_err() {
                // But some names might be reserved or have other issues
                prop_assert!(test_name.len() > 32 || test_name.is_empty(), 
                    "Valid function name '{}' unexpectedly failed", test_name);
            }
        }
    }
}

/// Test recursive functions work with arena batch evaluation
#[test]
fn test_recursive_with_arena_batch() {
    let arena = Bump::new();
    let mut builder = ArenaBatchBuilder::new(&arena);
    let mut ctx = EvalContext::new();
    
    // Register operators if not using libm
    #[cfg(not(feature = "libm"))]
    {
        ctx.register_native_function("<=", 2, |args| if args[0] <= args[1] { 1.0 } else { 0.0 });
        ctx.register_native_function("-", 2, |args| args[0] - args[1]);
        ctx.register_native_function("*", 2, |args| args[0] * args[1]);
    }
    
    // Register factorial
    ctx.register_expression_function(
        "factorial",
        &["n"],
        "n <= 1 ? 1 : n * factorial(n - 1)"
    ).unwrap();
    
    // Test multiple factorial calls in batch
    for n in 0..8 {
        builder.add_expression(&format!("factorial({})", n)).unwrap();
    }
    
    let ctx_rc = Rc::new(ctx);
    builder.eval(&ctx_rc).unwrap();
    
    // Verify results
    let expected = vec![1.0, 1.0, 2.0, 6.0, 24.0, 120.0, 720.0, 5040.0];
    for (i, &exp) in expected.iter().enumerate() {
        assert_eq!(builder.get_result(i).unwrap(), exp);
    }
}

/// Test that very long expressions work  
#[test]
fn test_long_expression_handling() {
    let arena = Bump::new();
    let mut builder = ArenaBatchBuilder::new(&arena);
    let mut ctx = EvalContext::new();
    
    // Register operators if not using libm
    #[cfg(not(feature = "libm"))]
    {
        ctx.register_native_function("+", 2, |args| args[0] + args[1]);
    }
    
    // Create a long but valid expression
    let long_expr = (0..50)
        .map(|i| format!("x + {}", i))
        .collect::<Vec<_>>()
        .join(" + ");
    
    ctx.register_expression_function("long_func", &["x"], &long_expr).unwrap();
    builder.add_expression("long_func(1)").unwrap();
    
    let ctx_rc = Rc::new(ctx);
    builder.eval(&ctx_rc).unwrap();
    
    // x=1, plus sum of 0..49 = 1 + (49*50/2) = 1 + 1225 = 1226
    // But each term is (x + i), so it's 50*1 + sum(0..49) = 50 + 1225 = 1275
    let expected = 50.0 + (0..50).sum::<i32>() as f64;
    assert_eq!(builder.get_result(0).unwrap(), expected);
}

/// Test semantic validation
#[test] 
fn test_semantic_validation() {
    let mut ctx = EvalContext::new();
    
    // Register operators if not using libm
    #[cfg(not(feature = "libm"))]
    {
        ctx.register_native_function("*", 2, |args| args[0] * args[1]);
    }
    
    // Register a function
    ctx.register_expression_function("helper", &["x"], "x * 2").unwrap();
    
    // Test semantic validation - undefined function
    let result = ctx.register_expression_function_validated(
        "uses_undefined",
        &["x"],
        "undefined_func(x)",
        true // Enable semantic validation
    );
    
    match result {
        Ok(report) => {
            assert!(report.syntax_valid);
            assert!(report.semantic_validated);
            assert!(!report.undefined_functions.is_empty());
        }
        Err(_) => panic!("Should return report, not error"),
    }
    
    // Test semantic validation - wrong arity
    let result = ctx.register_expression_function_validated(
        "wrong_arity",
        &["x"],
        "helper(x, 42)", // helper expects 1 arg, given 2
        true
    );
    
    match result {
        Ok(report) => {
            assert!(report.syntax_valid);
            assert!(report.semantic_validated);
            assert!(!report.arity_warnings.is_empty());
        }
        Err(_) => panic!("Should return report, not error"),
    }
}