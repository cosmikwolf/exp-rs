#[cfg(test)]
mod test {
    use crate::engine::parse_expression;
    use crate::eval::iterative::EvalEngine;
    use crate::expression::ArenaBatchBuilder;
    use crate::{EvalContext, Real};
    use bumpalo::Bump;
    use std::rc::Rc;

    #[test]
    fn test_expression_function_with_arena_minimal() {
        // Create arena
        let arena = Bump::new();

        // Create context and register an expression function
        let mut ctx = EvalContext::new();
        ctx.register_expression_function("double", &["x"], "x * 2")
            .unwrap();

        // Parse the expression in the arena
        let ast = parse_expression("double(5)", &arena).unwrap();

        // Create engine with arena support
        let mut engine = EvalEngine::new_with_arena(&arena);

        // Evaluate
        let result = engine.eval(&ast, Some(Rc::new(ctx))).unwrap();

        assert_eq!(result, 10.0);
    }

    #[test]
    fn test_expression_function_caching() {
        // Create arena
        let arena = Bump::new();

        // Create context
        let mut ctx = EvalContext::new();
        ctx.register_expression_function("square", &["x"], "x * x")
            .unwrap();

        // Create engine with arena
        let mut engine = EvalEngine::new_with_arena(&arena);
        let ctx_rc = Rc::new(ctx);

        // Record allocations before evaluation loop
        let allocated_before = arena.allocated_bytes();

        // Parse and evaluate expressions
        let mut asts = Vec::new();
        for i in 0..100 {
            let expr = arena.alloc_str(&format!("square({})", i));
            let ast = parse_expression(expr, &arena).unwrap();
            asts.push(ast);
        }

        // Evaluate them
        for (i, ast) in asts.iter().enumerate() {
            let result = engine.eval(ast, Some(ctx_rc.clone())).unwrap();
            assert_eq!(result, (i * i) as Real);
        }

        // Expression function body should only be parsed once and cached
        // The only new allocations should be for the new ASTs with different parameters
        let allocated_after = arena.allocated_bytes();
        println!(
            "Arena grew by {} bytes for 100 evaluations",
            allocated_after - allocated_before
        );

        // Should be much less than if we parsed the function body 100 times
        assert!(
            allocated_after - allocated_before < 10000,
            "Too many allocations"
        );
    }
}

