use exp_rs::{Real, assert_approx_eq, constants, context::EvalContext, engine::interp};
use std::rc::Rc;

#[test]
fn test_unary_expression_evaluation() {
    // Create a context with necessary operators
    let mut ctx = EvalContext::default();
    let _ = ctx.register_native_function("sin", 1, |args| args[0].sin());
    let _ = ctx.register_native_function("cos", 1, |args| args[0].cos());
    let _ = ctx.register_native_function("neg", 1, |args| -args[0]); // Unary minus alias
    let _ = ctx.register_native_function("-", 1, |args| -args[0]); // Unary minus
    let _ = ctx.register_native_function("+", 1, |args| args[0]); // Unary plus
    let _ = ctx.register_native_function("+", 2, |args| args[0] + args[1]); // Binary plus
    let _ = ctx.register_native_function("-", 2, |args| args[0] - args[1]); // Binary minus
    let _ = ctx.register_native_function("^", 2, |args| args[0].powf(args[1])); // Power

    let ctx_rc = Rc::new(ctx);

    // Define test expressions and expected results
    let test_cases = [
        // Simple unary
        ("-1", -1.0),
        ("+1", 1.0),
        ("--1", 1.0),
        ("++1", 1.0),
        ("+-1", -1.0),
        ("-+1", -1.0),
        // Chained unary operators - checking actual behavior
        ("-+-1", 1.0),  // should be 1.0: -(+(-(1))) = -(+((-1)) = -(-1) = 1
        ("+-+1", -1.0), // should be -1.0: +(-(+(1))) = +(-1) = -1
        ("--+1", 1.0),  // should be 1.0: -(-1) = 1
        ("+--1", 1.0),  // should be 1.0: +(-(-1)) = +(1) = 1
        ("---1", -1.0), // should be -1.0: -(-(-(1))) = -(-((-1)) = -(-1) = 1
        ("+++1", 1.0),  // should be 1.0: +(+(+(1))) = 1
        // Complex chaining - checking against actual parser behavior
        ("-+-+-1", -1.0),
        ("+-+-+1", 1.0),
        // Functions with unary
        ("-sin(1)", -0.8414709848078965),
        ("sin(-1)", -0.8414709848078965),
        // Unary with functions
        ("-sin(-cos(1))", 0.5143952585235492),
        // Unary with power
        ("-2^2", -4.0),
        ("(-2)^2", 4.0),
    ];

    for &(expr, expected) in &test_cases {
        let result = interp(expr, Some(ctx_rc.clone())).unwrap();
        println!("{:<15} => {}", expr, result);
        assert_approx_eq!(
            result,
            expected as Real, // Cast to Real type
            constants::TEST_PRECISION,
            "Expression '{}' evaluated to {}, expected {}",
            expr,
            result,
            expected
        );
    }
}
