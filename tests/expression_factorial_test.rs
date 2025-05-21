//! Test for verifying recursion depth in factorial expression evaluation
//!
//! This test checks that our recursion depth tracking in the expression evaluator
//! correctly tracks recursive function calls in a factorial expression.

#[cfg(test)]
use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use exp_rs::error::ExprError;
use exp_rs::eval::{get_recursion_depth, reset_recursion_depth};
use std::rc::Rc;

/// Test that directly examines recursion depth during factorial evaluation
#[test]
fn test_expression_factorial_depth() {
    // Create a new evaluation context
    let mut ctx = EvalContext::default();

    // Reset recursion counter to ensure we start fresh
    reset_recursion_depth();

    // Create a logger function to monitor recursion depth during evaluation
    ctx.register_native_function("log_depth", 1, |args| {
        let n = args[0].round() as i32;
        let depth = get_recursion_depth();

        println!("log_depth: n={}, current recursion depth={}", n, depth);

        // Return n unchanged - this is just for logging
        args[0]
    });

    // Create helper functions for factorial calculation
    ctx.register_native_function("is_base_case", 1, |args| {
        let n = args[0].round() as i32;
        if n <= 1 { 1.0 } else { 0.0 }
    });

    ctx.register_native_function("choose", 3, |args| {
        let condition = args[0];
        let if_true = args[1];
        let if_false = args[2];

        if condition != 0.0 { if_true } else { if_false }
    });

    // Register a factorial function that logs its current recursion depth
    // Format: if n <= 1 then 1 else n * factorial(n-1)
    // Note: We need to ensure we don't continue past the base case with negative values
    ctx.register_native_function("strict_base_case", 1, |args| {
        let n = args[0].round() as i32;
        // We want to ensure n is either 0 or 1
        if n == 0 || n == 1 { 1.0 } else { 0.0 }
    });

    // Register the factorial function using a simpler approach
    // First register a version without logging to make sure it works
    ctx.register_expression_function(
        "factorial_base",
        &["n"],
        "n <= 1 ? 1 : n * factorial_base(n-1)",
    )
    .unwrap();
    
    // Now register the logging version that calls the base version, using a comma operator
    ctx.register_expression_function(
        "factorial",
        &["n"],
        "log_depth(n), factorial_base(n)",
    )
    .unwrap();

    // Create a thread-local Vec to store depth records
    thread_local! {
        static DEPTH_RECORDS: std::cell::RefCell<Vec<(i32, usize)>> = std::cell::RefCell::new(Vec::new());
    }

    // Clear the records for a fresh start
    DEPTH_RECORDS.with(|records| records.borrow_mut().clear());

    // Create a tracking function to record recursion depths
    ctx.register_native_function("track_depth", 1, move |args| {
        let n = args[0].round() as i32;
        let current_depth = get_recursion_depth();

        println!("track_depth called with n={}, depth={}", n, current_depth);

        // Record this depth in the thread-local storage
        DEPTH_RECORDS.with(|records| {
            records.borrow_mut().push((n, current_depth));
        });

        // Just return n unchanged
        args[0]
    });

    // Register a tracked factorial base implementation
    ctx.register_expression_function(
        "tracked_factorial_base",
        &["n"],
        "n <= 1 ? 1 : n * tracked_factorial_base(n-1)",
    )
    .unwrap();
    
    // Let's use a simpler approach with factorial
    ctx.register_expression_function(
        "factorial_fn",
        &["n"],
        "n <= 1 ? 1 : n * factorial_fn(n-1)",
    )
    .unwrap();
    
    // Register a tracked factorial function that increments recursion depth
    ctx.register_expression_function(
        "tracked_factorial",
        &["n"],
        r#"
        track_depth(n),
        n <= 1 ? 1 : n * tracked_factorial(n-1)
        "#,
    ).unwrap();
    
    // Register the track_depth function to record recursion depths
    ctx.register_native_function("track_depth", 1, |args| {
        let n = args[0].round() as i32;
        let depth = get_recursion_depth();
        
        println!("track_depth called with n={}, depth={}", n, depth);
        
        // Record this depth in thread-local storage
        DEPTH_RECORDS.with(|records| {
            records.borrow_mut().push((n, depth));
        });
        
        // Return the input unchanged
        args[0]
    });
    
    // We won't need the ensure_0 function anymore - we'll call factorial(0) directly later

    // First, evaluate factorial(4) and verify the result
    println!("\nEvaluating factorial(4):");
    let result = interp("factorial(4)", Some(Rc::new(ctx.clone())));

    assert!(result.is_ok(), "factorial(4) should evaluate successfully");
    assert_eq!(result.unwrap(), 24.0, "factorial(4) should equal 24");

    // Now evaluate the tracked version to collect depth information
    println!("\nEvaluating tracked_factorial(4) to record recursion depths:");
    reset_recursion_depth(); // Reset counter before this test
    let result = interp("tracked_factorial(4)", Some(Rc::new(ctx.clone())));

    assert!(
        result.is_ok(),
        "tracked_factorial(4) should evaluate successfully"
    );
    assert_eq!(
        result.unwrap(),
        24.0,
        "tracked_factorial(4) should equal 24"
    );

    // We don't need to track factorial(0) manually anymore
    // Our recursive implementation will automatically evaluate all steps from 4 to 1
    
    // Retrieve the recorded depths from thread_local storage
    let depth_records = DEPTH_RECORDS.with(|records| records.borrow().clone());

    // Analyze the recorded depths
    println!("\nRecorded recursion depths:");
    for (n, depth) in &depth_records {
        println!("factorial({}) called at depth {}", n, depth);
    }

    // Validate the maximum depth reached
    let max_depth = depth_records.iter().map(|(_, d)| *d).max().unwrap_or(0);
    println!("\nMaximum recursion depth: {}", max_depth);

    // Check that each recursive call has the expected depth
    // Note: depth numbering starts at different points in our implementation vs. logical call levels
    // We care about the relationship between n and depth, and the total depth range

    // Since we're calling factorial(4) through factorial(1) but not factorial(0),
    // we should have exactly 4 factorial calls
    assert_eq!(
        depth_records.len(),
        4,
        "Should have 4 factorial calls for factorial(4) through factorial(1)"
    );

    // Sort records by n value to check the relationship
    let mut sorted_records = depth_records.clone();
    sorted_records.sort_by_key(|(n, _)| -n); // Sort in descending order of n

    // Verify each n value is present exactly once
    let n_values: Vec<_> = sorted_records.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        n_values,
        vec![4, 3, 2, 1],
        "Should have calls for n values 4,3,2,1"
    );

    // Verify the depths are monotonically increasing as n decreases
    let depths: Vec<_> = sorted_records.iter().map(|(_, d)| *d).collect();
    for i in 1..depths.len() {
        assert!(
            depths[i] > depths[i - 1],
            "Depth should increase with each recursive call"
        );
    }

    // Calculate the actual recursion depth used (difference between max and min)
    let min_depth = depths.iter().min().unwrap_or(&0);
    let depth_range = max_depth - min_depth;

    // The actual recursion depth is higher than 4 due to how evaluation works:
    // Each function call goes through multiple AST nodes that each increment depth
    println!(
        "Depth range (difference between max and min depth): {}",
        depth_range
    );
    assert_eq!(
        depth_range, 12,
        "Recursion depth range for factorial(4) should reflect AST evaluation"
    );

    println!("Test passed: factorial(4) used the expected recursion depth!");
}

