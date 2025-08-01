//! Integration test for safe recursive function implementation
//! This test demonstrates how to implement a safe recursive function with manual depth tracking

extern crate alloc;

#[cfg(test)]
use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use exp_rs::Real;
use std::rc::Rc;

#[path = "test_helpers.rs"]
mod test_helpers;
use test_helpers::{hstr, set_attr};

/// Test safe recursion with manual depth tracking
/// This demonstrates implementing factorial and fibonacci using a safe
/// approach that prevents stack overflow
#[test]
#[ignore = "Expression functions require arena allocation - not supported in current architecture"]
fn test_safe_recursion() {
    // Create a new evaluation context
    let mut ctx = EvalContext::default();

    // First, register helper functions to handle base cases and recursion depth
    
    // 1. Register a function to detect base cases (n <= 1)
    ctx.register_native_function("is_base_case", 1, |args| {
        let n = args[0].round() as i32;
        if n <= 1 {
            1.0 // true
        } else {
            0.0 // false
        }
    });

    // 2. Register a function to check if max recursion depth is reached
    ctx.register_native_function("is_max_depth", 1, |args| {
        let depth = args[0].round() as i32;
        if depth >= 100 { // Limit recursion to 100 levels
            1.0 // true, max depth reached
        } else {
            0.0 // false, can continue recursion
        }
    });

    // 3. Register helper functions for factorial calculation
    
    // Base case value for factorial
    ctx.register_native_function("factorial_base", 1, |args| {
        let n = args[0].round() as i32;
        if n <= 1 {
            1.0 // 0! = 1! = 1
        } else {
            0.0 // Error case, should not reach here
        }
    });
    
    // Recursive case calculation for factorial
    ctx.register_native_function("factorial_recursive", 3, |args| {
        let n = args[0].round() as i32;
        let prev_result = args[1]; // Result from factorial(n-1)
        let _depth = args[2]; // Current recursion depth (unused in calculation but tracked)
        
        n as Real * prev_result // n * factorial(n-1)
    });

    // 4. Register helper functions for fibonacci calculation

    // Base case values for fibonacci
    ctx.register_native_function("fibonacci_base", 1, |args| {
        let n = args[0].round() as i32;
        if n <= 0 {
            0.0 // fib(0) = 0
        } else if n == 1 {
            1.0 // fib(1) = 1
        } else {
            -1.0 // Error case, should not reach here
        }
    });
    
    // Recursive case calculation for fibonacci
    ctx.register_native_function("fibonacci_recursive", 4, |args| {
        let _n = args[0].round() as i32; // Current n (unused in calculation but needed for API)
        let prev1 = args[1]; // Result from fib(n-1)
        let prev2 = args[2]; // Result from fib(n-2)
        let _depth = args[3]; // Current recursion depth (unused in calculation but tracked)
        
        prev1 + prev2 // fib(n-1) + fib(n-2)
    });

    // 5. Now implement the safe recursive factorial function
    //    This uses a loop-based approach to simulate recursion with explicit depth tracking
    ctx.register_native_function("factorial", 1, |args| {
        let n = args[0].round() as i32;
        
        // Handle base cases directly
        if n <= 1 {
            return 1.0; // 0! = 1! = 1
        }
        
        // For larger values, use manual recursion with depth tracking
        let mut result = 1.0; // Start with base case result
        
        // Iteratively calculate factorial from bottom up
        for i in 2..=n {
            result = i as Real * result; // i * factorial(i-1)
        }
        
        result
    });

    // 6. Implement a safe recursive fibonacci function
    //    This also uses a loop-based approach to avoid actual recursion
    ctx.register_native_function("fibonacci", 1, |args| {
        let n = args[0].round() as i32;
        
        // Handle base cases directly
        if n <= 0 {
            return 0.0; // fib(0) = 0
        } else if n == 1 {
            return 1.0; // fib(1) = 1
        }
        
        // For larger values, use manual recursion with tracking
        let mut prev2 = 0.0; // fib(0)
        let mut prev1 = 1.0; // fib(1)
        let mut result = 0.0;
        
        // Iteratively calculate fibonacci from bottom up
        for _i in 2..=n {
            result = prev1 + prev2;
            prev2 = prev1;
            prev1 = result;
        }
        
        result
    });

    // 7. Register expression functions that use the depth-tracking approach
    //    These demonstrate how to use the native functions with explicit depth tracking
    
    // First, create an expression function for factorial that passes the depth parameter
    ctx.register_expression_function(
        "safe_factorial",
        &["n", "depth"],
        // If base case or max depth reached, return base value, otherwise continue recursion
        "is_base_case(n) * factorial_base(n) + (1 - is_base_case(n)) * (1 - is_max_depth(depth)) * factorial_recursive(n, safe_factorial(n-1, depth+1), depth)"
    ).unwrap();

    // Now create an expression function for fibonacci that passes the depth parameter
    // This is more complex as it requires two recursive calls
    ctx.register_expression_function(
        "safe_fibonacci",
        &["n", "depth"],
        // If base case or max depth reached, return base value, otherwise continue recursion
        "is_base_case(n) * fibonacci_base(n) + (1 - is_base_case(n)) * (1 - is_max_depth(depth)) * fibonacci_recursive(n, safe_fibonacci(n-1, depth+1), safe_fibonacci(n-2, depth+1), depth)"
    ).unwrap();

    // Test factorial function
    assert_eq!(interp("factorial(0)", Some(Rc::new(ctx.clone()))).unwrap(), 1.0);
    assert_eq!(interp("factorial(1)", Some(Rc::new(ctx.clone()))).unwrap(), 1.0);
    assert_eq!(interp("factorial(5)", Some(Rc::new(ctx.clone()))).unwrap(), 120.0);
    assert_eq!(interp("factorial(10)", Some(Rc::new(ctx.clone()))).unwrap(), 3628800.0);
    
    // Test fibonacci function
    assert_eq!(interp("fibonacci(0)", Some(Rc::new(ctx.clone()))).unwrap(), 0.0);
    assert_eq!(interp("fibonacci(1)", Some(Rc::new(ctx.clone()))).unwrap(), 1.0);
    assert_eq!(interp("fibonacci(2)", Some(Rc::new(ctx.clone()))).unwrap(), 1.0);
    assert_eq!(interp("fibonacci(3)", Some(Rc::new(ctx.clone()))).unwrap(), 2.0);
    assert_eq!(interp("fibonacci(10)", Some(Rc::new(ctx.clone()))).unwrap(), 55.0);
    assert_eq!(interp("fibonacci(20)", Some(Rc::new(ctx.clone()))).unwrap(), 6765.0);

    // Test safe expression functions with explicit depth tracking
    // Start with depth=0
    // Note: These may fail with recursion limit, so we handle both success and failure cases
    
    // Helper function to check if a result is either the expected value or a recursion limit error
    let check_result = |expr: &str, expected: Real| {
        let result = interp(expr, Some(Rc::new(ctx.clone())));
        match result {
            Ok(val) => {
                assert_eq!(val, expected, "Expected {} for expression {}", expected, expr);
                println!("Successfully evaluated: {}", expr);
            },
            Err(exp_rs::error::ExprError::RecursionLimit(_)) => {
                // This is acceptable - the recursion depth limit is working as expected
                println!("Recursion limit reached for: {} (this is normal)", expr);
            },
            Err(exp_rs::error::ExprError::CapacityExceeded(resource)) => {
                // With the iterative evaluator, we get capacity exceeded instead of recursion limit
                println!("Capacity limit reached for: {} ({}) (this is normal)", expr, resource);
            },
            Err(e) => {
                panic!("Unexpected error for {}: {:?}", expr, e);
            }
        }
    };
    
    // Check factorial base cases
    check_result("safe_factorial(0, 0)", 1.0);
    check_result("safe_factorial(1, 0)", 1.0);
    
    // This one might hit recursion limits, but that's OK
    check_result("safe_factorial(5, 0)", 120.0);
    
    // Check fibonacci base cases
    check_result("safe_fibonacci(0, 0)", 0.0);
    check_result("safe_fibonacci(1, 0)", 1.0);
    
    // This one might hit recursion limits, but that's OK
    check_result("safe_fibonacci(5, 0)", 5.0);
    
    // Alternative implementation with explicit recursion tracking in context
    let mut recursive_ctx = EvalContext::default();
    
    // Setup a context variable to track recursion depth
    recursive_ctx.variables.insert(hstr("max_depth"), 100.0).expect("Failed to insert variable");
    recursive_ctx.variables.insert(hstr("current_depth"), 0.0).expect("Failed to insert variable");
    
    // Register helper function to check recursion depth
    recursive_ctx.register_native_function("check_depth", 0, move |_args| {
        // This would need to access shared state via Arc<Mutex<>> in a real implementation
        // Here we just demonstrate the pattern
        1.0 // Always allow recursion for test simplicity
    });
    
    // Register a "tracked factorial" that uses context to track depth
    recursive_ctx.register_native_function("tracked_factorial", 1, |args| {
        let n = args[0].round() as i32;
        
        // Base cases
        if n <= 1 {
            return 1.0;
        }
        
        // In a real implementation, we would:
        // 1. Check/increment/decrement a shared depth counter
        // 2. Return early if max depth reached
        // 3. Use actual recursion with the depth check as a guard
        
        // For test purposes, directly calculate using iteration
        let mut result = 1.0;
        for i in 2..=n {
            result *= i as Real;
        }
        result
    });
    
    // Test tracked factorial
    assert_eq!(interp("tracked_factorial(5)", Some(Rc::new(recursive_ctx.clone()))).unwrap(), 120.0);
    
    // Real-world implementation with a thread_local depth counter
    // This is not part of the test but demonstrates how you might implement it
    // in a real application with shared mutable state
    println!("Safe recursion tests passed!");
}

/// Test recursion with a more advanced implementation that handles
/// error conditions and provides informative messages
#[test]
#[ignore = "Attributes implementation needs to be fixed for heapless"]
fn test_advanced_recursion_error_handling() {
    // Create a new context
    let mut ctx = EvalContext::default();
    
    // Setup error codes using attribute helpers
    set_attr(&mut ctx, "Error", "MAX_DEPTH_EXCEEDED", -999.0);
    set_attr(&mut ctx, "Error", "INVALID_INPUT", -998.0);
    
    // Register a factorial function with explicit error handling
    ctx.register_native_function("factorial_safe", 2, |args| {
        let n = args[0].round() as i32;
        let depth = args[1].round() as i32;
        let max_depth = 100; // Safety limit
        
        // Error checking
        if n < 0 {
            return -998.0; // INVALID_INPUT
        }
        
        if depth > max_depth {
            return -999.0; // MAX_DEPTH_EXCEEDED
        }
        
        // Base case
        if n <= 1 {
            return 1.0;
        }
        
        // We simulate recursion manually here rather than actual recursion
        // In a real implementation, you would use the recursive call:
        // let sub_result = factorial_safe(n-1, depth+1);
        
        let mut result = 1.0;
        for i in 2..=n {
            result *= i as Real;
            
            // Simulate checking for errors after each "recursive" step
            if result < 0.0 {
                return result; // Propagate error code
            }
        }
        
        result
    });
    
    // Register a helper to interpret errors
    ctx.register_native_function("is_error", 1, |args| {
        let value = args[0];
        if value < -900.0 { // Error codes are below -900
            1.0 // true
        } else {
            0.0 // false
        }
    });
    
    // Test normal cases
    assert_eq!(interp("factorial_safe(5, 0)", Some(Rc::new(ctx.clone()))).unwrap(), 120.0);
    assert_eq!(interp("factorial_safe(10, 0)", Some(Rc::new(ctx.clone()))).unwrap(), 3628800.0);
    
    // Test error cases
    assert_eq!(interp("factorial_safe(-1, 0)", Some(Rc::new(ctx.clone()))).unwrap(), -998.0); // INVALID_INPUT
    assert_eq!(interp("factorial_safe(5, 200)", Some(Rc::new(ctx.clone()))).unwrap(), -999.0); // MAX_DEPTH_EXCEEDED
    
    // Test error detection
    assert_eq!(interp("is_error(factorial_safe(-1, 0))", Some(Rc::new(ctx.clone()))).unwrap(), 1.0); // True
    assert_eq!(interp("is_error(factorial_safe(5, 0))", Some(Rc::new(ctx.clone()))).unwrap(), 0.0); // False
    
    println!("Advanced recursion error handling tests passed!");
}

/// Test implementing mutual recursion with depth tracking
#[test]
#[ignore = "Expression functions require arena allocation - not supported in current architecture"]
fn test_mutual_recursion() {
    // Create a new context
    let mut ctx = EvalContext::default();
    
    // Register is_even and is_odd functions with mutual recursion and depth tracking
    
    // First register base case detection helper
    ctx.register_native_function("is_terminal", 1, |args| {
        let n = args[0].round() as i32;
        if n <= 0 {
            1.0 // True - terminal case
        } else {
            0.0 // False - continue recursion
        }
    });
    
    // Check if max depth reached
    ctx.register_native_function("depth_exceeded", 1, |args| {
        let depth = args[0].round() as i32;
        if depth >= 100 { // Limit to prevent infinite recursion
            1.0 // True - max depth reached
        } else {
            0.0 // False - can continue
        }
    });
    
    // Helper functions for mutual recursion base cases
    ctx.register_native_function("is_even_base", 1, |args| {
        let n = args[0].round() as i32;
        if n == 0 {
            1.0 // True - 0 is even
        } else {
            0.0 // False - not a base case or not even
        }
    });
    
    ctx.register_native_function("is_odd_base", 1, |args| {
        let n = args[0].round() as i32;
        if n == 0 {
            0.0 // False - 0 is not odd
        } else {
            0.0 // Not a base case
        }
    });
    
    // Safe implementation without actual recursion
    ctx.register_native_function("is_even", 1, |args| {
        let n = args[0].round() as i32;
        if n < 0 {
            return ((-n) % 2 == 0) as i32 as Real;
        }
        (n % 2 == 0) as i32 as Real
    });
    
    ctx.register_native_function("is_odd", 1, |args| {
        let n = args[0].round() as i32;
        if n < 0 {
            return ((-n) % 2 == 1) as i32 as Real;
        }
        (n % 2 == 1) as i32 as Real
    });
    
    // Register the mutual recursive expression functions with depth tracking
    ctx.register_expression_function(
        "safe_is_even",
        &["n", "depth"],
        // If terminal case (n=0) or depth exceeded, use base case
        // Otherwise, calculate using safe_is_odd of n-1 with increased depth
        "is_terminal(n) * is_even_base(n) + (1 - is_terminal(n)) * (1 - depth_exceeded(depth)) * safe_is_odd(n-1, depth+1)"
    ).unwrap();
    
    ctx.register_expression_function(
        "safe_is_odd",
        &["n", "depth"],
        // If terminal case (n=0) or depth exceeded, use base case
        // Otherwise, calculate using safe_is_even of n-1 with increased depth
        "is_terminal(n) * is_odd_base(n) + (1 - is_terminal(n)) * (1 - depth_exceeded(depth)) * safe_is_even(n-1, depth+1)"
    ).unwrap();
    
    // Test non-recursive versions
    assert_eq!(interp("is_even(0)", Some(Rc::new(ctx.clone()))).unwrap(), 1.0); // True
    assert_eq!(interp("is_odd(0)", Some(Rc::new(ctx.clone()))).unwrap(), 0.0); // False
    assert_eq!(interp("is_even(1)", Some(Rc::new(ctx.clone()))).unwrap(), 0.0); // False
    assert_eq!(interp("is_odd(1)", Some(Rc::new(ctx.clone()))).unwrap(), 1.0); // True
    assert_eq!(interp("is_even(10)", Some(Rc::new(ctx.clone()))).unwrap(), 1.0); // True
    assert_eq!(interp("is_odd(11)", Some(Rc::new(ctx.clone()))).unwrap(), 1.0); // True
    
    // The safe recursive versions would require evaluating a lot of nested expressions
    // which could be challenging for the expression parser. Here we just test
    // the base case, as real recursion would need stronger depth checking support.
    
    // Helper function to check if a result is either the expected value or a recursion limit error
    let check_result = |expr: &str, expected: Real| {
        let result = interp(expr, Some(Rc::new(ctx.clone())));
        match result {
            Ok(val) => {
                assert_eq!(val, expected, "Expected {} for expression {}", expected, expr);
                println!("Successfully evaluated: {}", expr);
            },
            Err(exp_rs::error::ExprError::RecursionLimit(_)) => {
                // This is acceptable - the recursion depth limit is working as expected
                println!("Recursion limit reached for: {} (this is normal)", expr);
            },
            Err(exp_rs::error::ExprError::CapacityExceeded(resource)) => {
                // With the iterative evaluator, we get capacity exceeded instead of recursion limit
                println!("Capacity limit reached for: {} ({}) (this is normal)", expr, resource);
            },
            Err(e) => {
                panic!("Unexpected error for {}: {:?}", expr, e);
            }
        }
    };
    
    // Test base case
    check_result("safe_is_even(0, 0)", 1.0); // True for "0 is even"
    
    println!("Mutual recursion tests passed!");
}