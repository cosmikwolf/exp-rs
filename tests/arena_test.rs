//! Test arena-based expression evaluation

use exp_rs::{Real};
use exp_rs::engine::parse_expression_arena;
use exp_rs::eval::eval_ast;
use exp_rs::context::EvalContext;
use bumpalo::Bump;
use std::rc::Rc;

#[test]
fn test_arena_basic_expression() {
    // Create arena
    let arena = Bump::new();
    
    // Parse expression into arena
    let ast = parse_expression_arena("2 + 3", &arena).unwrap();
    
    // Evaluate
    let result = eval_ast(&ast, None).unwrap();
    assert_eq!(result, 5.0);
}

#[test]
fn test_arena_with_variables() {
    // Create arena
    let arena = Bump::new();
    
    // Create context with variables
    let mut ctx = EvalContext::new();
    ctx.set_parameter("x", 10.0);
    ctx.set_parameter("y", 20.0);
    let ctx = Rc::new(ctx);
    
    // Parse expression
    let ast = parse_expression_arena("x + y", &arena).unwrap();
    
    // Evaluate
    let result = eval_ast(&ast, Some(ctx)).unwrap();
    assert_eq!(result, 30.0);
}

#[test]
fn test_arena_zero_allocations() {
    // Create arena with fixed capacity
    let arena = Bump::with_capacity(4096);
    
    // Parse expression once
    let ast = parse_expression_arena("x * 2 + y", &arena).unwrap();
    let allocated_after_parse = arena.allocated_bytes();
    
    // Create context
    let mut ctx = EvalContext::new();
    ctx.set_parameter("x", 1.0);
    ctx.set_parameter("y", 1.0);
    let ctx = Rc::new(ctx);
    
    // Evaluate many times - should not allocate
    for i in 0..1000 {
        ctx.set_parameter("x", i as Real);
        let result = eval_ast(&ast, Some(ctx.clone())).unwrap();
        assert_eq!(result, (i as Real) * 2.0 + 1.0);
        
        // Verify no new allocations
        assert_eq!(arena.allocated_bytes(), allocated_after_parse,
            "Arena grew during evaluation #{}", i);
    }
}