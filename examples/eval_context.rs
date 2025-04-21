extern crate alloc;
use exp_rs::EvalContext;
use libm::*;
use std::println;

// The Real type alias would be useful if we needed to work with both f32 and f64,
// but since our functions are specialized via macros, we don't need it here.

// Helper macro to wrap C math functions for f64
#[cfg(feature = "f64")]
macro_rules! c_fn {
    ($name:ident) => {
        |args: &[f64]| $name(args[0])
    };
}

// Helper macro to wrap C math functions for f32
#[cfg(feature = "f32")]
macro_rules! c_fn {
    ($name:ident) => {
        |args: &[f32]| $name(args[0])
    };
}

fn main() {
    let mut ctx = EvalContext::new();

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
        ctx.register_native_function("sin", 1, c_fn!(sinf));
        ctx.register_native_function("cos", 1, c_fn!(cosf));
        ctx.register_native_function("tan", 1, c_fn!(tanf));
        ctx.register_native_function("exp", 1, c_fn!(expf));
        ctx.register_native_function("log", 1, c_fn!(logf));
        ctx.register_native_function("sqrt", 1, c_fn!(sqrtf));
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
