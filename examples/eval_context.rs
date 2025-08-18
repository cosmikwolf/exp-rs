extern crate alloc;
use exp_rs::EvalContext;

// Import libm only when the feature is enabled
#[cfg(feature = "libm")]
use libm::{cos, exp, log, sin, sqrt, tan};

use std::println;

// Helper macro to wrap std math functions for f64
#[cfg(not(feature = "libm"))]
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
#[cfg(feature = "libm")]
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
    let mut ctx = EvalContext::new();

    #[cfg(not(feature = "f32"))]
    {
        let _ = ctx.register_native_function("sin", 1, c_fn!(sin));
        let _ = ctx.register_native_function("cos", 1, c_fn!(cos));
        let _ = ctx.register_native_function("tan", 1, c_fn!(tan));
        let _ = ctx.register_native_function("exp", 1, c_fn!(exp));
        let _ = ctx.register_native_function("log", 1, c_fn!(log));
        let _ = ctx.register_native_function("sqrt", 1, c_fn!(sqrt));
        ctx.register_expression_function("fancy", &["x"], "sin(x) + cos(x) + 42")
            .unwrap();
    }

    #[cfg(feature = "f32")]
    {
        #[cfg(feature = "libm")]
        {
            let _ = ctx.register_native_function("sin", 1, c_fn!(sinf));
            let _ = ctx.register_native_function("cos", 1, c_fn!(cosf));
            let _ = ctx.register_native_function("tan", 1, c_fn!(tanf));
            let _ = ctx.register_native_function("exp", 1, c_fn!(expf));
            let _ = ctx.register_native_function("log", 1, c_fn!(logf));
            let _ = ctx.register_native_function("sqrt", 1, c_fn!(sqrtf));
        }

        #[cfg(not(feature = "libm"))]
        {
            let _ = ctx.register_native_function("sin", 1, c_fn!(sin));
            let _ = ctx.register_native_function("cos", 1, c_fn!(cos));
            let _ = ctx.register_native_function("tan", 1, c_fn!(tan));
            let _ = ctx.register_native_function("exp", 1, c_fn!(exp));
            let _ = ctx.register_native_function("log", 1, c_fn!(log));
            let _ = ctx.register_native_function("sqrt", 1, c_fn!(sqrt));
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
