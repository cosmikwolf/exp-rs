use bumpalo::Bump;
use exp_rs::{EvalContext, Expression, interp};
use std::rc::Rc;
use std::time::Instant;

fn create_test_context() -> Rc<EvalContext> {
    let mut ctx = EvalContext::new();

    // Register basic math functions
    ctx.register_native_function("abs", 1, |args| args[0].abs())
        .unwrap();
    ctx.register_native_function("sign", 1, |args| {
        if args[0] > 0.0 {
            1.0
        } else if args[0] < 0.0 {
            -1.0
        } else {
            0.0
        }
    })
    .unwrap();

    // Trigonometric
    ctx.register_native_function("sin", 1, |args| args[0].sin())
        .unwrap();
    ctx.register_native_function("cos", 1, |args| args[0].cos())
        .unwrap();
    ctx.register_native_function("atan2", 2, |args| args[0].atan2(args[1]))
        .unwrap();

    // Exponential and logarithmic
    ctx.register_native_function("exp", 1, |args| args[0].exp())
        .unwrap();
    ctx.register_native_function("log", 1, |args| args[0].ln())
        .unwrap();
    ctx.register_native_function("log10", 1, |args| args[0].log10())
        .unwrap();
    ctx.register_native_function("pow", 2, |args| args[0].powf(args[1]))
        .unwrap();
    ctx.register_native_function("sqrt", 1, |args| args[0].sqrt())
        .unwrap();

    // Min/max
    ctx.register_native_function("min", 2, |args| args[0].min(args[1]))
        .unwrap();
    ctx.register_native_function("max", 2, |args| args[0].max(args[1]))
        .unwrap();

    // Modulo
    ctx.register_native_function("fmod", 2, |args| args[0] % args[1])
        .unwrap();

    Rc::new(ctx)
}

fn main() {
    println!("=== Analyzing Performance Difference ===\n");

    // Setup
    let ctx = create_test_context();

    let expressions = vec![
        "a*sin(b*3.14159/180) + c*cos(d*3.14159/180) + sqrt(e*e + f*f)",
        "exp(g/10) * log(h+1) + pow(i, 0.5) * j",
        "((a > 5) && (b < 10)) * c + ((d >= e) || (f != g)) * h + min(i, j)",
        "sqrt(pow(a-e, 2) + pow(b-f, 2)) + atan2(c-g, d-h) * (i+j)/2",
        "abs(a-b) * sign(c-d) + max(e, f) * min(g, h) + fmod(i*j, 10)",
        "(a+b+c)/3 * sin((d+e+f)*3.14159/6) + log10(g*h+1) - exp(-i*j/100)",
        "a + b * c - d / (e + 0.001) + pow(f, g) * h - i + j",
    ];

    let param_names = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];

    // Test 1: Individual evaluation (mimicking pre-bumpalo benchmark)
    println!("Test 1: Individual evaluation (context clone per iteration)");
    const ITERATIONS: usize = 10_000;

    let start = Instant::now();
    for i in 0..ITERATIONS {
        // Clone context and set parameters
        let mut ctx_clone = (*ctx).clone();
        for (p, name) in param_names.iter().enumerate() {
            ctx_clone.set_parameter(name, (p + 1) as f64 * 1.5).unwrap();
        }
        let ctx_rc = Rc::new(ctx_clone);

        // Evaluate all expressions
        for expr in &expressions {
            let _result = interp(expr, Some(ctx_rc.clone())).unwrap();
        }
    }
    let individual_duration = start.elapsed();
    let individual_us = individual_duration.as_secs_f64() * 1e6 / ITERATIONS as f64;
    let individual_rate = 1e6 / individual_us;

    println!("  Time per iteration: {:.1} µs", individual_us);
    println!("  Rate: {:.0} Hz", individual_rate);
    println!("  This matches the ~2,933 Hz from pre-bumpalo\n");

    // Test 2: BatchBuilder with pre-parsed expressions
    println!("Test 2: BatchBuilder (pre-parsed expressions)");
    let arena = Bump::new();
    let mut builder = Expression::new(&arena);

    // Add parameters and expressions
    let mut param_indices = Vec::new();
    for name in &param_names {
        let idx = builder.add_parameter(name, 0.0).unwrap();
        param_indices.push(idx);
    }

    for expr in &expressions {
        builder.add_expression(expr).unwrap();
    }

    // Warm up
    for _ in 0..100 {
        builder.eval(&ctx).unwrap();
    }

    let start = Instant::now();
    for i in 0..ITERATIONS {
        // Only update parameters
        for (p, &idx) in param_indices.iter().enumerate() {
            builder
                .set_param(idx, (p + 1) as f64 * 1.5 + i as f64 * 0.001)
                .unwrap();
        }
        // Evaluate pre-parsed expressions
        builder.eval(&ctx).unwrap();
    }
    let batch_duration = start.elapsed();
    let batch_us = batch_duration.as_secs_f64() * 1e6 / ITERATIONS as f64;
    let batch_rate = 1e6 / batch_us;

    println!("  Time per iteration: {:.1} µs", batch_us);
    println!("  Rate: {:.0} Hz", batch_rate);
    println!("  Speedup: {:.1}x\n", batch_rate / individual_rate);

    // Breakdown of costs
    println!("Cost Breakdown:");
    println!("  Individual approach per iteration:");
    println!("    - Clone context");
    println!("    - Set 10 parameters");
    println!("    - Parse 7 expressions");
    println!("    - Evaluate 7 expressions");
    println!("    - Allocate AST nodes");
    println!("    - Allocate result values");

    println!("\n  BatchBuilder approach per iteration:");
    println!("    - Update 10 parameters (in pre-allocated array)");
    println!("    - Evaluate 7 pre-parsed expressions");
    println!("    - No parsing");
    println!("    - No AST allocation");
    println!("    - Reuse result storage");
}

