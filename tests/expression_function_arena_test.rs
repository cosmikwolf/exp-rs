//! Test expression functions with arena allocation

use exp_rs::{Real, EvalContext};
use exp_rs::batch_builder::ArenaBatchBuilder;
use bumpalo::Bump;
use std::rc::Rc;

#[test]
fn test_expression_function_with_arena() {
    // Create arena
    let arena = Bump::new();
    
    // Create batch builder with arena
    let mut builder = ArenaBatchBuilder::new(&arena);
    
    // Create context and register an expression function
    let mut ctx = EvalContext::new();
    ctx.register_expression_function("double", &["x"], "x * 2").unwrap();
    ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4").unwrap();
    
    // Add expressions that use the functions
    assert_eq!(builder.add_expression("double(5)").unwrap(), 0);
    assert_eq!(builder.add_expression("polynomial(2)").unwrap(), 1);
    assert_eq!(builder.add_expression("double(x) + polynomial(y)").unwrap(), 2);
    
    // Add parameters for the third expression
    assert_eq!(builder.add_parameter("x", 3.0).unwrap(), 0);
    assert_eq!(builder.add_parameter("y", 1.0).unwrap(), 1);
    
    // Evaluate
    let ctx_rc = Rc::new(ctx);
    builder.eval(&ctx_rc).unwrap();
    
    // Check results
    assert_eq!(builder.get_result(0).unwrap(), 10.0); // double(5) = 10
    assert_eq!(builder.get_result(1).unwrap(), 26.0); // 2^3 + 2*2^2 + 3*2 + 4 = 8 + 8 + 6 + 4 = 26
    assert_eq!(builder.get_result(2).unwrap(), 16.0); // double(3) + polynomial(1) = 6 + 10 = 16
}

#[test]
fn test_nested_expression_functions() {
    // Create arena
    let arena = Bump::new();
    
    // Create batch builder
    let mut builder = ArenaBatchBuilder::new(&arena);
    
    // Create context with nested functions
    let mut ctx = EvalContext::new();
    ctx.register_expression_function("add_one", &["x"], "x + 1").unwrap();
    ctx.register_expression_function("double", &["x"], "x * 2").unwrap();
    ctx.register_expression_function("compose", &["x"], "double(add_one(x))").unwrap();
    
    // Test nested function call
    assert_eq!(builder.add_expression("compose(5)").unwrap(), 0);
    
    // Evaluate
    let ctx_rc = Rc::new(ctx);
    builder.eval(&ctx_rc).unwrap();
    
    // compose(5) = double(add_one(5)) = double(6) = 12
    assert_eq!(builder.get_result(0).unwrap(), 12.0);
}

#[test]
fn test_expression_function_zero_allocations() {
    // Create arena
    let arena = Bump::with_capacity(16384);
    
    // Create batch builder
    let mut builder = ArenaBatchBuilder::new(&arena);
    
    // Create context
    let mut ctx = EvalContext::new();
    ctx.register_expression_function("compute", &["x", "y"], "x^2 + y^2 + sin(x*y)").unwrap();
    
    // Add expression
    builder.add_expression("compute(a, b)").unwrap();
    builder.add_parameter("a", 0.0).unwrap();
    builder.add_parameter("b", 0.0).unwrap();
    
    let ctx_rc = Rc::new(ctx);
    
    // Record allocations before evaluation loop
    let allocated_before = arena.allocated_bytes();
    
    // Evaluate many times - should not allocate after first parse
    for i in 0..1000 {
        builder.set_param(0, i as Real * 0.1).unwrap();
        builder.set_param(1, i as Real * 0.05).unwrap();
        builder.eval(&ctx_rc).unwrap();
    }
    
    // Check that no additional allocations occurred
    let allocated_after = arena.allocated_bytes();
    
    // The expression function should be parsed only once and cached
    // No allocations should occur during the 1000 evaluations
    assert_eq!(allocated_before, allocated_after, 
        "Expression functions should not allocate during evaluation");
}

#[test]
fn test_expression_function_with_all_param_types() {
    let arena = Bump::new();
    let mut builder = ArenaBatchBuilder::new(&arena);
    
    let mut ctx = EvalContext::new();
    
    // Register functions with various parameter counts
    ctx.register_expression_function("zero_params", &[], "42").unwrap();
    ctx.register_expression_function("one_param", &["x"], "x * x").unwrap();
    ctx.register_expression_function("three_params", &["a", "b", "c"], "a + b * c").unwrap();
    
    // Test all functions
    builder.add_expression("zero_params()").unwrap();
    builder.add_expression("one_param(7)").unwrap();
    builder.add_expression("three_params(1, 2, 3)").unwrap();
    
    let ctx_rc = Rc::new(ctx);
    builder.eval(&ctx_rc).unwrap();
    
    assert_eq!(builder.get_result(0).unwrap(), 42.0);
    assert_eq!(builder.get_result(1).unwrap(), 49.0);
    assert_eq!(builder.get_result(2).unwrap(), 7.0); // 1 + 2*3
}