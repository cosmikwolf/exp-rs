use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use std::rc::Rc;

#[test]
fn test_simple_infinite_recursion() {
    let mut ctx = EvalContext::new();
    
    // Register a function that calls itself without a base case
    ctx.register_expression_function("infinite", &["x"], "infinite(x + 1)")
        .unwrap();
    
    // Try to evaluate - should fail with recursion limit
    let result = interp("infinite(0)", Some(Rc::new(ctx)));
    
    // Check if we get an error
    assert!(result.is_err(), "Should have failed with an error");
    
    // Print the error for debugging
    println!("Error: {:?}", result.unwrap_err());
}

#[test]
fn test_recursion_with_small_depth() {
    let mut ctx = EvalContext::new();
    
    // Test a function that recurses only a few times
    ctx.register_expression_function("countdown", &["n"], "n <= 0 ? 0 : countdown(n - 1)")
        .unwrap();
    
    // This should work fine with small values
    let result = interp("countdown(5)", Some(Rc::new(ctx)));
    assert!(result.is_ok(), "Should succeed with small recursion depth");
    assert_eq!(result.unwrap(), 0.0);
}