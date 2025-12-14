use bumpalo::Bump;
use exp_rs::{EvalContext, expression::Expression};
use std::rc::Rc;

#[test]
fn temp_test_simple_expression_function() {
    let arena = Bump::new();
    let mut builder = Expression::new(&arena);
    let ctx = EvalContext::new();

    // Register basic math operators
    #[cfg(not(feature = "libm"))]
    {
        ctx.register_native_function("+", 2, |args| args[0] + args[1]);
        ctx.register_native_function("*", 2, |args| args[0] * args[1]);
    }

    // Register expression function in the batch instead of context
    builder
        .register_expression_function("double", &["x"], "x * 2")
        .unwrap();

    let ctx_rc = Rc::new(ctx);

    // Add a parameter and expression using the function
    builder.add_parameter("value", 5.0).unwrap();
    builder.add_expression("double(value)").unwrap();

    match builder.eval(&ctx_rc) {
        Ok(_) => {
            let result = builder.get_result(0).unwrap();
            println!("double(5) = {}", result);
            assert_eq!(result, 10.0);
        }
        Err(e) => {
            panic!("Expression function evaluation failed: {:?}", e);
        }
    }
}

#[test]
fn temp_test_expression_function_with_direct_arg() {
    let arena = Bump::new();
    let mut builder = Expression::new(&arena);
    let ctx = EvalContext::new();

    // Register basic math operators
    #[cfg(not(feature = "libm"))]
    {
        ctx.register_native_function("+", 2, |args| args[0] + args[1]);
        ctx.register_native_function("*", 2, |args| args[0] * args[1]);
    }

    // Register expression function in the batch instead of context
    builder
        .register_expression_function("double", &["x"], "x * 2")
        .unwrap();

    let ctx_rc = Rc::new(ctx);

    // Test with direct argument (no batch parameters)
    builder.add_expression("double(3)").unwrap();

    match builder.eval(&ctx_rc) {
        Ok(_) => {
            let result = builder.get_result(0).unwrap();
            println!("double(3) = {}", result);
            assert_eq!(result, 6.0);
        }
        Err(e) => {
            panic!("Expression function evaluation failed: {:?}", e);
        }
    }
}

