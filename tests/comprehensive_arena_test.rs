//! Comprehensive test for zero allocations during evaluation
//! This test closely matches what the FFI C test does

use bumpalo::Bump;
use exp_rs::{EvalContext, Real, expression::ArenaBatchBuilder};
use std::rc::Rc;

#[test]
fn test_batch_zero_allocations_with_functions() {
    // Create arena (matching C test)
    let arena = Bump::with_capacity(256 * 1024); // 256KB like C test

    // Create context and register functions (like C test does)
    let mut ctx = EvalContext::new();

    // Register native functions
    #[cfg(feature = "libm")]
    {
        // These are already registered by default with libm
    }
    #[cfg(not(feature = "libm"))]
    {
        use libm::{cos, sin, sqrt};
        ctx.register_native_function("sin", 1, |args| sin(args[0] as f64) as Real)
            .unwrap();
        ctx.register_native_function("cos", 1, |args| cos(args[0] as f64) as Real)
            .unwrap();
        ctx.register_native_function("sqrt", 1, |args| sqrt(args[0] as f64) as Real)
            .unwrap();
    }

    // Also test expression functions
    ctx.register_expression_function("double", &["x"], "x * 2")
        .unwrap();
    ctx.register_expression_function("pythag", &["a", "b"], "sqrt(a*a + b*b)")
        .unwrap();

    let ctx = Rc::new(ctx);

    // Create batch builder (like expr_batch_new)
    let mut builder = ArenaBatchBuilder::new(&arena);

    // Add expressions (like C test)
    // Test 1: Native functions only (like C test)
    builder
        .add_expression("sin(x) * cos(y) + sqrt(x*x + y*y)")
        .unwrap();

    // Test 2: Expression function
    builder.add_expression("double(x) + double(y)").unwrap();

    // Test 3: Nested expression function
    builder.add_expression("pythag(x, y) * 2").unwrap();

    // Test 4: Deeply nested expression functions (expression function calling expression function)
    builder.add_expression("double(pythag(x, y))").unwrap();

    // Add variables
    builder.add_parameter("x", 1.0).unwrap();
    builder.add_parameter("y", 2.0).unwrap();

    // Record initial arena size
    let bytes_after_setup = arena.allocated_bytes();
    println!("Arena after setup: {} bytes", bytes_after_setup);

    // First evaluation (might set up some internals)
    builder.eval(&ctx).unwrap();
    let bytes_after_first = arena.allocated_bytes();
    println!(
        "Arena after first eval: {} bytes (growth: {})",
        bytes_after_first,
        bytes_after_first - bytes_after_setup
    );

    // Now test repeated evaluations - should have ZERO arena growth
    for i in 0..10000 {
        // Update parameters (like C test does)
        builder.set_param(0, (i as Real) * 0.1).unwrap(); // x
        builder.set_param(1, (i as Real) * 0.2).unwrap(); // y

        // Evaluate
        builder.eval(&ctx).unwrap();

        // Check for arena growth
        let current_bytes = arena.allocated_bytes();
        assert_eq!(
            current_bytes, bytes_after_first,
            "Arena grew during evaluation #{}: {} -> {} bytes",
            i, bytes_after_first, current_bytes
        );

        // Verify results are reasonable
        let result1 = builder.get_result(0).unwrap();
        let result2 = builder.get_result(1).unwrap();
        let result3 = builder.get_result(2).unwrap();
        let result4 = builder.get_result(3).unwrap();
        assert!(result1.is_finite());
        assert!(result2.is_finite());
        assert!(result3.is_finite());
        assert!(result4.is_finite());
    }

    println!(
        "✓ Zero arena growth during 100 batch evaluations with native and expression functions!"
    );
}

#[test]
fn test_expression_api_zero_allocations() {
    use exp_rs::Expression;

    // Test the higher-level Expression API too
    let arena = Bump::with_capacity(128 * 1024);

    let mut expr = Expression::new(&arena);

    // Add parameters
    expr.add_parameter("x", 0.0).unwrap();
    expr.add_parameter("y", 0.0).unwrap();
    expr.add_parameter("z", 0.0).unwrap();

    // Add complex expressions
    expr.add_expression("x*x + y*y + z*z").unwrap();
    expr.add_expression("sin(x) * cos(y) + tan(z)").unwrap();
    expr.add_expression("(x + y) * (x - y) / (z + 1)").unwrap();

    let ctx = Rc::new(EvalContext::new());

    // Initial evaluation
    expr.set("x", 1.0).unwrap();
    expr.set("y", 2.0).unwrap();
    expr.set("z", 3.0).unwrap();
    expr.eval(&ctx).unwrap();

    let bytes_after_first = arena.allocated_bytes();

    // Many evaluations with different parameters
    for i in 0..100 {
        expr.set("x", i as Real * 0.1).unwrap();
        expr.set("y", i as Real * 0.2).unwrap();
        expr.set("z", i as Real * 0.3).unwrap();

        expr.eval(&ctx).unwrap();

        assert_eq!(
            arena.allocated_bytes(),
            bytes_after_first,
            "Arena grew during Expression API evaluation #{}",
            i
        );
    }

    println!("✓ Zero arena growth with Expression API!");
}
