extern crate alloc;
use exp_rs::EvalContext;

// Import libm only when the feature is enabled

use std::println;

// Helper macro to wrap std math functions for f64
#[cfg(all(feature = "f64", not(feature = "libm")))]
macro_rules! c_fn {
    (sin) => {
        |args: &[f64]| args[0].sin()
    };
    (cos) => {
        |args: &[f64]| args[0].cos()
    };
    (tan) => {
        |args: &[f64]| args[0].tan()
    };
    (exp) => {
        |args: &[f64]| args[0].exp()
    };
    (log) => {
        |args: &[f64]| args[0].ln()
    };
    (sqrt) => {
        |args: &[f64]| args[0].sqrt()
    };
}

// Helper macro to wrap libm functions for f64
#[cfg(all(feature = "f64", feature = "libm"))]
macro_rules! c_fn {
    ($name:ident) => {
        |args: &[f64]| $name(args[0])
    };
}

// Helper macro for f32 without libm
#[cfg(all(feature = "f32", not(feature = "libm")))]
macro_rules! c_fn {
    (sin) => {
        |args: &[f32]| args[0].sin()
    };
    (cos) => {
        |args: &[f32]| args[0].cos()
    };
    (tan) => {
        |args: &[f32]| args[0].tan()
    };
    (exp) => {
        |args: &[f32]| args[0].exp()
    };
    (log) => {
        |args: &[f32]| args[0].ln()
    };
    (sqrt) => {
        |args: &[f32]| args[0].sqrt()
    };
}

// Helper macro for f32 with libm
#[cfg(all(feature = "f32", feature = "libm"))]
macro_rules! c_fn {
    ($name:ident) => {
        |args: &[f32]| $name(args[0])
    };
}

fn main() {
    let ctx = EvalContext::new();

    #[cfg(feature = "f64")]
    {
        ctx.register_native_function("sin", 1, c_fn!(sin));
        ctx.register_native_function("cos", 1, c_fn!(cos));
        ctx.register_native_function("tan", 1, c_fn!(tan));
        ctx.register_native_function("exp", 1, c_fn!(exp));
        ctx.register_native_function("log", 1, c_fn!(log));
        ctx.register_native_function("sqrt", 1, c_fn!(sqrt));
        ctx.register_expression_function("fancy", &["x"], "sin(x) + cos(x) + 42")
            .unwrap();
    }

    #[cfg(feature = "f32")]
    {
        #[cfg(feature = "libm")]
        {
            ctx.register_native_function("sin", 1, c_fn!(sinf));
            ctx.register_native_function("cos", 1, c_fn!(cosf));
            ctx.register_native_function("tan", 1, c_fn!(tanf));
            ctx.register_native_function("exp", 1, c_fn!(expf));
            ctx.register_native_function("log", 1, c_fn!(logf));
            ctx.register_native_function("sqrt", 1, c_fn!(sqrtf));
        }
        
        #[cfg(not(feature = "libm"))]
        {
            ctx.register_native_function("sin", 1, c_fn!(sin));
            ctx.register_native_function("cos", 1, c_fn!(cos));
            ctx.register_native_function("tan", 1, c_fn!(tan));
            ctx.register_native_function("exp", 1, c_fn!(exp));
            ctx.register_native_function("log", 1, c_fn!(log));
            ctx.register_native_function("sqrt", 1, c_fn!(sqrt));
        }
        
        ctx.register_expression_function("fancy", &["x"], "sin(x) + cos(x) + 42")
            .unwrap();
    }

    let exprs = [
        "sin(1.0)",
        "cos(1.0)",
        "sqrt(9)",
        "fancy(0.5)",
        "fancy(2.0) + sqrt(16)",
    ];

    for expr in &exprs {
        match exp_rs::engine::interp(expr, Some(std::rc::Rc::new(ctx.clone()))) {
            Ok(val) => {
                println!("{} = {}", expr, val);
                // For no_std, replace with your platform's output method
            }
            Err(e) => {
                println!("Error evaluating {}: {}", expr, e);
            }
        }
    }
}
