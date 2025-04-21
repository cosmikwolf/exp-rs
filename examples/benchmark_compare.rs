extern crate alloc;
extern crate libm;

use exp_rs::Real;
use exp_rs::context::EvalContext;
use exp_rs::engine::parse_expression;
use exp_rs::eval::eval_ast;
use std::time::Instant;
use alloc::rc::Rc;

// Native Rust implementations for benchmarking
fn native_sqrt_expr(a: Real) -> Real {
    (a.powf(1.5) + a.powf(2.5)).sqrt()
}
fn native_a_plus_5(a: Real) -> Real {
    a + 5.0
}
fn native_a_plus_5_times_2(a: Real) -> Real {
    a + (5.0 * 2.0)
}
fn native_a_plus_5_all_times_2(a: Real) -> Real {
    (a + 5.0) * 2.0
}
fn native_sum_fractions(a: Real) -> Real {
    1.0 / (a + 1.0) + 2.0 / (a + 2.0) + 3.0 / (a + 3.0)
}

const N: usize = 100_000;

fn main() {
    let benchmarks = [
        ("sqrt(a^1.5+a^2.5)", native_sqrt_expr as fn(Real) -> Real),
        ("a+5", native_a_plus_5),
        ("a+(5*2)", native_a_plus_5_times_2),
        ("(a+5)*2", native_a_plus_5_all_times_2),
        ("(1/(a+1)+2/(a+2)+3/(a+3))", native_sum_fractions),
        // Additional benchmark expressions with other functions:
        ("sin(a)", |a: Real| a.sin()),
        ("cos(a)", |a: Real| a.cos()),
        ("tan(a)", |a: Real| a.tan()),
        ("log(a+10)", |a: Real| (a+10.0).log10()),
        ("ln(a+10)", |a: Real| (a+10.0).ln()),
        ("abs(a-50)", |a: Real| (a-50.0).abs()),
        ("max(a,100-a)", |a: Real| a.max(100.0-a)),
        ("min(a,100-a)", |a: Real| a.min(100.0-a)),
        ("pow(a,1.5)", |a: Real| a.powf(1.5)),
        ("exp(a/100.0)", |a: Real| (a/100.0).exp()),
        ("floor(a/3.1)", |a: Real| (a/3.1).floor()),
        ("ceil(a/3.1)", |a: Real| (a/3.1).ceil()),
        ("fmod(a,7)", |a: Real| a % 7.0),
        ("neg(a)", |a: Real| -a),
    ];

    for (expr, native_func) in benchmarks.iter() {
        println!("Benchmarking: {}", expr);

        let ast = parse_expression(expr).expect("parse failed");

        // Create a mutable context first before wrapping in Rc
        let mut ctx_base = EvalContext::new();
        
        // Only register default math functions if the feature is available
        #[cfg(not(feature = "no-builtin-math"))]
        {
            ctx_base.register_default_math_functions();
        }
        
        // When no-builtin-math is enabled, we need to register the functions manually
        #[cfg(feature = "no-builtin-math")]
        {
            // Register the minimum functions needed for our benchmarks
            #[cfg(feature = "f32")]
            {
                ctx_base.register_native_function("sqrt", 1, |args| libm::sqrtf(args[0]));
                ctx_base.register_native_function("sin", 1, |args| libm::sinf(args[0]));
                ctx_base.register_native_function("cos", 1, |args| libm::cosf(args[0]));
                ctx_base.register_native_function("tan", 1, |args| libm::tanf(args[0]));
                ctx_base.register_native_function("log", 1, |args| libm::logf(args[0]));
                ctx_base.register_native_function("log10", 1, |args| libm::log10f(args[0]));
                ctx_base.register_native_function("ln", 1, |args| libm::logf(args[0]));
                ctx_base.register_native_function("abs", 1, |args| args[0].abs());
                ctx_base.register_native_function("max", 2, |args| args[0].max(args[1]));
                ctx_base.register_native_function("min", 2, |args| args[0].min(args[1]));
                ctx_base.register_native_function("pow", 2, |args| libm::powf(args[0], args[1]));
                ctx_base.register_native_function("^", 2, |args| libm::powf(args[0], args[1]));
                ctx_base.register_native_function("exp", 1, |args| libm::expf(args[0]));
                ctx_base.register_native_function("floor", 1, |args| libm::floorf(args[0]));
                ctx_base.register_native_function("ceil", 1, |args| libm::ceilf(args[0]));
                ctx_base.register_native_function("neg", 1, |args| -args[0]);
                ctx_base.register_native_function("fmod", 2, |args| args[0] % args[1]);
            }
            #[cfg(not(feature = "f32"))]
            {
                ctx_base.register_native_function("sqrt", 1, |args| libm::sqrt(args[0]));
                ctx_base.register_native_function("sin", 1, |args| libm::sin(args[0]));
                ctx_base.register_native_function("cos", 1, |args| libm::cos(args[0]));
                ctx_base.register_native_function("tan", 1, |args| libm::tan(args[0]));
                ctx_base.register_native_function("log", 1, |args| libm::log(args[0]));
                ctx_base.register_native_function("log10", 1, |args| libm::log10(args[0]));
                ctx_base.register_native_function("ln", 1, |args| libm::log(args[0]));
                ctx_base.register_native_function("abs", 1, |args| args[0].abs());
                ctx_base.register_native_function("max", 2, |args| args[0].max(args[1]));
                ctx_base.register_native_function("min", 2, |args| args[0].min(args[1]));
                ctx_base.register_native_function("pow", 2, |args| libm::pow(args[0], args[1]));
                ctx_base.register_native_function("^", 2, |args| libm::pow(args[0], args[1]));
                ctx_base.register_native_function("exp", 1, |args| libm::exp(args[0]));
                ctx_base.register_native_function("floor", 1, |args| libm::floor(args[0]));
                ctx_base.register_native_function("ceil", 1, |args| libm::ceil(args[0]));
                ctx_base.register_native_function("neg", 1, |args| -args[0]);
                ctx_base.register_native_function("fmod", 2, |args| args[0] % args[1]);
            }
        }
        
        let mut evalctx_sum = 0.0;
        let start = Instant::now();
        for j in 0..N {
            // Create a new context for each iteration with the parameter set
            let mut ctx = ctx_base.clone();
            ctx.set_parameter("a", j as Real);
            let ctx_rc = Rc::new(ctx);
            evalctx_sum += eval_ast(&ast, Some(ctx_rc)).unwrap();
        }
        let evalctx_time = start.elapsed();
        std::hint::black_box(evalctx_sum);

        // Create a mutable context first before wrapping in Rc
        let mut ctx_interp_base = EvalContext::new();
        
        // Only register default math functions if the feature is available
        #[cfg(not(feature = "no-builtin-math"))]
        {
            ctx_interp_base.register_default_math_functions();
        }
        
        // When no-builtin-math is enabled, we need to register the functions manually
        #[cfg(feature = "no-builtin-math")]
        {
            // Register the minimum functions needed for our benchmarks
            #[cfg(feature = "f32")]
            {
                ctx_interp_base.register_native_function("sqrt", 1, |args| libm::sqrtf(args[0]));
                ctx_interp_base.register_native_function("sin", 1, |args| libm::sinf(args[0]));
                ctx_interp_base.register_native_function("cos", 1, |args| libm::cosf(args[0]));
                ctx_interp_base.register_native_function("tan", 1, |args| libm::tanf(args[0]));
                ctx_interp_base.register_native_function("log", 1, |args| libm::logf(args[0]));
                ctx_interp_base.register_native_function("log10", 1, |args| libm::log10f(args[0]));
                ctx_interp_base.register_native_function("ln", 1, |args| libm::logf(args[0]));
                ctx_interp_base.register_native_function("abs", 1, |args| args[0].abs());
                ctx_interp_base.register_native_function("max", 2, |args| args[0].max(args[1]));
                ctx_interp_base.register_native_function("min", 2, |args| args[0].min(args[1]));
                ctx_interp_base.register_native_function("pow", 2, |args| libm::powf(args[0], args[1]));
                ctx_interp_base.register_native_function("^", 2, |args| libm::powf(args[0], args[1]));
                ctx_interp_base.register_native_function("exp", 1, |args| libm::expf(args[0]));
                ctx_interp_base.register_native_function("floor", 1, |args| libm::floorf(args[0]));
                ctx_interp_base.register_native_function("ceil", 1, |args| libm::ceilf(args[0]));
                ctx_interp_base.register_native_function("neg", 1, |args| -args[0]);
                ctx_interp_base.register_native_function("fmod", 2, |args| args[0] % args[1]);
            }
            #[cfg(not(feature = "f32"))]
            {
                ctx_interp_base.register_native_function("sqrt", 1, |args| libm::sqrt(args[0]));
                ctx_interp_base.register_native_function("sin", 1, |args| libm::sin(args[0]));
                ctx_interp_base.register_native_function("cos", 1, |args| libm::cos(args[0]));
                ctx_interp_base.register_native_function("tan", 1, |args| libm::tan(args[0]));
                ctx_interp_base.register_native_function("log", 1, |args| libm::log(args[0]));
                ctx_interp_base.register_native_function("log10", 1, |args| libm::log10(args[0]));
                ctx_interp_base.register_native_function("ln", 1, |args| libm::log(args[0]));
                ctx_interp_base.register_native_function("abs", 1, |args| args[0].abs());
                ctx_interp_base.register_native_function("max", 2, |args| args[0].max(args[1]));
                ctx_interp_base.register_native_function("min", 2, |args| args[0].min(args[1]));
                ctx_interp_base.register_native_function("pow", 2, |args| libm::pow(args[0], args[1]));
                ctx_interp_base.register_native_function("^", 2, |args| libm::pow(args[0], args[1]));
                ctx_interp_base.register_native_function("exp", 1, |args| libm::exp(args[0]));
                ctx_interp_base.register_native_function("floor", 1, |args| libm::floor(args[0]));
                ctx_interp_base.register_native_function("ceil", 1, |args| libm::ceil(args[0]));
                ctx_interp_base.register_native_function("neg", 1, |args| -args[0]);
                ctx_interp_base.register_native_function("fmod", 2, |args| args[0] % args[1]);
            }
        }
        
        // Enable AST cache for the base context
        ctx_interp_base.enable_ast_cache();
        
        let mut interp_sum = 0.0;
        let start = Instant::now();
        for j in 0..N {
            // Create a new context for each iteration with the parameter set
            let mut ctx_interp = ctx_interp_base.clone();
            ctx_interp.set_parameter("a", j as Real);
            let ctx_interp_rc = Rc::new(ctx_interp);
            interp_sum += exp_rs::engine::interp(expr, Some(ctx_interp_rc)).unwrap();
        }
        let interp_eval_time = start.elapsed();
        std::hint::black_box(interp_sum);

        let mut native_sum = 0.0;
        let start = Instant::now();
        for j in 0..N {
            native_sum += native_func(j as Real);
        }
        let native_time = start.elapsed();
        std::hint::black_box(native_sum);

        let evalctx_us = evalctx_time.as_micros();
        let interp_us = interp_eval_time.as_micros();
        let native_us = native_time.as_micros();

        let slowdown_evalctx_vs_native = if native_us > 0 {
            evalctx_us as f64 / native_us as f64
        } else {
            f64::NAN
        };
        let slowdown_interp_vs_native = if native_us > 0 {
            interp_us as f64 / native_us as f64
        } else {
            f64::NAN
        };
        let slowdown_interp_vs_evalctx = if evalctx_us > 0 {
            interp_us as f64 / evalctx_us as f64
        } else {
            f64::NAN
        };

        println!("evalctx - time: {} us, {:.2}x slower than native", evalctx_us, slowdown_evalctx_vs_native);
        println!("interp - time: {} us, {:.2}x slower than native", interp_us, slowdown_interp_vs_native);
        println!("native - time: {} us", native_us);
        println!("evalctx vs native: {:.2}x slower", slowdown_evalctx_vs_native);
        println!("interp vs native: {:.2}x slower", slowdown_interp_vs_native);
        println!("interp vs evalctx: {:.2}x slower\n", slowdown_interp_vs_evalctx);
    }
}
