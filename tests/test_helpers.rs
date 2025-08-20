use exp_rs::context::EvalContext;
use exp_rs::error::ExprError;
use exp_rs::types::{FunctionName, HString, TryIntoFunctionName, TryIntoHeaplessString};

/// Helper function to convert a string slice to heapless string for tests
#[allow(dead_code)]
pub fn hstr(s: &str) -> HString {
    s.try_into_heapless().expect("String too long for test")
}

/// Helper function to convert a string slice to function name for tests
#[allow(dead_code)]
pub fn fname(s: &str) -> FunctionName {
    s.try_into_function_name()
        .expect("Function name too long for test")
}

/// Helper function to convert &str to HString but return Result for error handling
#[allow(dead_code)]
pub fn try_hstr(s: &str) -> Result<HString, ExprError> {
    s.try_into_heapless()
}

/// Helper function to register all necessary functions for tests
/// This ensures we have consistent function implementations across all tests
/// Functions are only registered when libm is not available
#[allow(dead_code)]
pub fn create_test_context() -> EvalContext {
    let mut ctx = EvalContext::default();

    // Only register functions when libm is not available
    // When libm is available, the default context already has these functions
    #[cfg(not(feature = "libm"))]
    {
        // Basic math operators
        ctx.register_native_function("+", 2, |args| args[0] + args[1]);
        ctx.register_native_function("-", 2, |args| args[0] - args[1]);
        ctx.register_native_function("*", 2, |args| args[0] * args[1]);
        ctx.register_native_function("/", 2, |args| args[0] / args[1]);
        ctx.register_native_function("^", 2, |args| args[0].powf(args[1]));
        ctx.register_native_function("neg", 1, |args| -args[0]);

        // Comparison operators
        ctx.register_native_function("<", 2, |args| if args[0] < args[1] { 1.0 } else { 0.0 });
        ctx.register_native_function(">", 2, |args| if args[0] > args[1] { 1.0 } else { 0.0 });
        ctx.register_native_function("<=", 2, |args| if args[0] <= args[1] { 1.0 } else { 0.0 });
        ctx.register_native_function(">=", 2, |args| if args[0] >= args[1] { 1.0 } else { 0.0 });
        ctx.register_native_function("==", 2, |args| if args[0] == args[1] { 1.0 } else { 0.0 });
        ctx.register_native_function("!=", 2, |args| if args[0] != args[1] { 1.0 } else { 0.0 });

        // Trigonometric functions
        ctx.register_native_function("sin", 1, |args| args[0].sin());
        ctx.register_native_function("cos", 1, |args| args[0].cos());
        ctx.register_native_function("tan", 1, |args| args[0].tan());
        ctx.register_native_function("asin", 1, |args| args[0].asin());
        ctx.register_native_function("acos", 1, |args| args[0].acos());
        ctx.register_native_function("atan", 1, |args| args[0].atan());
        ctx.register_native_function("atan2", 2, |args| args[0].atan2(args[1]));

        // Hyperbolic functions
        ctx.register_native_function("sinh", 1, |args| args[0].sinh());
        ctx.register_native_function("cosh", 1, |args| args[0].cosh());
        ctx.register_native_function("tanh", 1, |args| args[0].tanh());

        // Other math functions
        ctx.register_native_function("sqrt", 1, |args| args[0].sqrt());
        ctx.register_native_function("log", 1, |args| args[0].log10());
        ctx.register_native_function("ln", 1, |args| args[0].ln());
        ctx.register_native_function("log10", 1, |args| args[0].log10());
        ctx.register_native_function("floor", 1, |args| args[0].floor());
        ctx.register_native_function("ceil", 1, |args| args[0].ceil());
        ctx.register_native_function("round", 1, |args| args[0].round());
        ctx.register_native_function("abs", 1, |args| args[0].abs());
        ctx.register_native_function("exp", 1, |args| args[0].exp());

        // Control flow
        ctx.register_native_function(
            "?:",
            3,
            |args| if args[0] != 0.0 { args[1] } else { args[2] },
        );

        // Sequence operator
        ctx.register_native_function(",", 2, |args| args[1]);
        ctx.register_native_function("comma", 2, |args| args[1]);

        // Named math operators
        ctx.register_native_function("add", 2, |args| args[0] + args[1]);
        ctx.register_native_function("sub", 2, |args| args[0] - args[1]);
        ctx.register_native_function("mul", 2, |args| args[0] * args[1]);
        ctx.register_native_function("div", 2, |args| args[0] / args[1]);
        ctx.register_native_function("pow", 2, |args| args[0].powf(args[1]));
        ctx.register_native_function("fmod", 2, |args| args[0] % args[1]);
    }

    // Always add constants since they're needed for both libm and no-libm cases
    use exp_rs::Real;
    ctx.set_parameter("pi", std::f64::consts::PI as Real)
        .expect("Failed to set pi");
    ctx.set_parameter("e", std::f64::consts::E as Real)
        .expect("Failed to set e");

    ctx
}

/// Helper function to set a variable in the context for tests
#[allow(dead_code)]
pub fn set_var(ctx: &mut EvalContext, name: &str, value: exp_rs::Real) {
    ctx.variables
        .insert(hstr(name), value)
        .expect("Failed to set variable in test");
}

/// Helper function to set a constant in the context for tests  
#[allow(dead_code)]
pub fn set_const(ctx: &mut EvalContext, name: &str, value: exp_rs::Real) {
    ctx.constants
        .insert(hstr(name), value)
        .expect("Failed to set constant in test");
}

/// Helper function to set a parameter in the context for tests
#[allow(dead_code)]
pub fn set_param(ctx: &mut EvalContext, name: &str, value: exp_rs::Real) {
    ctx.set_parameter(name, value)
        .expect("Failed to set parameter in test");
}

/// Helper function to set an attribute in the context for tests
#[allow(dead_code)]
pub fn set_attr(ctx: &mut EvalContext, object: &str, attr: &str, value: exp_rs::Real) {
    ctx.set_attribute(object, attr, value)
        .expect("Failed to set attribute in test");
}

/// Helper function that just wraps create_test_context with an Rc
#[cfg(not(feature = "libm"))]
pub fn create_test_context_rc() -> std::rc::Rc<EvalContext> {
    std::rc::Rc::new(create_test_context())
}

/// Helper function to initialize a default context based on features
#[allow(dead_code)]
pub fn create_context<'a>() -> EvalContext {
    #[cfg(not(feature = "libm"))]
    return create_test_context();

    #[cfg(feature = "libm")]
    return EvalContext::default();
}

/// Helper function to initialize a default context as Rc based on features
#[allow(dead_code)]
pub fn create_context_rc<'a>() -> std::rc::Rc<EvalContext> {
    std::rc::Rc::new(create_context())
}
