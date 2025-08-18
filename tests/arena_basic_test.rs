use bumpalo::Bump;
use exp_rs::{
    EvalContext,
    engine::parse_expression,
    eval::iterative::{EvalEngine, eval_with_engine},
    expression::Expression,
};
use std::rc::Rc;

#[test]
fn test_arena_zero_allocations() {
    // Create arena
    let arena = Bump::with_capacity(64 * 1024); // 64KB

    // Parse expressions into arena
    let expr1 = parse_expression("x * 2 + y", &arena).unwrap();
    let expr2 = parse_expression("sin(x) + cos(y)", &arena).unwrap();

    let initial_bytes = arena.allocated_bytes();
    println!("Arena bytes after parsing: {}", initial_bytes);

    // Create context
    let mut ctx = EvalContext::new();
    ctx.set_parameter("x", 1.0).unwrap();
    ctx.set_parameter("y", 2.0).unwrap();
    let ctx = Rc::new(ctx);

    // Create a reusable evaluation engine (this allocates the stacks once)
    let mut engine = EvalEngine::new(&arena);

    // Do a first evaluation to set up the evaluator stacks
    let mut ctx_first = (*ctx).clone();
    ctx_first.set_parameter("x", 0.0).unwrap();
    let ctx_rc_first = Rc::new(ctx_first);
    let _result1 = eval_with_engine(&expr1, Some(ctx_rc_first.clone()), &mut engine).unwrap();
    let _result2 = eval_with_engine(&expr2, Some(ctx_rc_first), &mut engine).unwrap();

    // Get arena size after initial evaluation (evaluator stacks are now allocated)
    let bytes_after_first_eval = arena.allocated_bytes();
    println!(
        "Arena bytes after first evaluation: {}",
        bytes_after_first_eval
    );

    // Evaluate many times - should not allocate beyond the initial setup
    for i in 1..1000 {
        // Update context for this test
        let mut ctx_clone = (*ctx).clone();
        ctx_clone.set_parameter("x", i as f64).unwrap();
        let ctx_rc = Rc::new(ctx_clone);

        let result1 = eval_with_engine(&expr1, Some(ctx_rc.clone()), &mut engine).unwrap();
        let _result2 = eval_with_engine(&expr2, Some(ctx_rc), &mut engine).unwrap();

        // Verify results are correct
        assert_eq!(result1, (i as f64) * 2.0 + 2.0);

        // Verify no new arena allocations beyond the initial evaluator setup
        assert_eq!(
            arena.allocated_bytes(),
            bytes_after_first_eval,
            "Arena grew during evaluation #{} (beyond initial evaluator setup)",
            i
        );
    }

    println!("✓ No additional arena growth beyond evaluator setup during 1000 evaluations!");
}

#[test]
fn test_batch_builder_arena() {
    // Create arena
    let arena = Bump::with_capacity(64 * 1024);

    // Create batch builder
    let mut builder = Expression::new(&arena);

    // Add parameters
    builder.add_parameter("x", 0.0).unwrap();
    builder.add_parameter("y", 0.0).unwrap();

    // Add expressions
    let idx1 = builder.add_expression("x * 2 + y").unwrap();
    let idx2 = builder.add_expression("x + y * 3").unwrap();

    let initial_bytes = arena.allocated_bytes();
    println!("Arena bytes after setup: {}", initial_bytes);

    // Create context
    let ctx = Rc::new(EvalContext::new());

    // Evaluate many times with different parameters
    for i in 0..1000 {
        builder.set_param_by_name("x", i as f64).unwrap();
        builder.set_param_by_name("y", (i * 2) as f64).unwrap();

        builder.eval(&ctx).unwrap();

        // Check results
        assert_eq!(
            builder.get_result(idx1).unwrap(),
            (i as f64) * 2.0 + (i * 2) as f64
        );
        assert_eq!(
            builder.get_result(idx2).unwrap(),
            (i as f64) + (i * 2) as f64 * 3.0
        );

        // Verify no new arena allocations
        assert_eq!(
            arena.allocated_bytes(),
            initial_bytes,
            "Arena grew during batch evaluation #{}",
            i
        );
    }

    println!("✓ Zero arena allocations during 1000 batch evaluations!");
}
