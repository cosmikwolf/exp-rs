use exp_rs::context::EvalContext;

/// Helper function to register all necessary functions for tests
/// This ensures we have consistent function implementations across all tests
/// Functions are only registered when libm is not available
pub fn create_test_context() -> EvalContext<'static> {
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
        ctx.register_native_function("?:", 3, |args| if args[0] != 0.0 { args[1] } else { args[2] });
        
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
    ctx.set_parameter("pi", std::f64::consts::PI as Real);
    ctx.set_parameter("e", std::f64::consts::E as Real);
    
    ctx
}

/// Helper function that just wraps create_test_context with an Rc
#[cfg(not(feature = "libm"))]
pub fn create_test_context_rc() -> std::rc::Rc<EvalContext<'static>> {
    std::rc::Rc::new(create_test_context())
}

/// Helper function to initialize a default context based on features
pub fn create_context<'a>() -> EvalContext<'a> {
    #[cfg(not(feature = "libm"))]
    return create_test_context();
    
    #[cfg(feature = "libm")]
    return EvalContext::default();
}

/// Helper function to initialize a default context as Rc based on features
pub fn create_context_rc<'a>() -> std::rc::Rc<EvalContext<'a>> {
    std::rc::Rc::new(create_context())
}