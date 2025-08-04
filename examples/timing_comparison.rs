use bumpalo::Bump;
use exp_rs::{EvalContext, Expression};
use std::rc::Rc;
use std::time::Instant;

fn main() {
    println!("=== Rust Timing Analysis ===\n");

    // Create context with all functions
    let mut ctx = EvalContext::new();
    ctx.register_native_function("sin", 1, |args| args[0].sin())
        .unwrap();
    ctx.register_native_function("cos", 1, |args| args[0].cos())
        .unwrap();
    ctx.register_native_function("sqrt", 1, |args| args[0].sqrt())
        .unwrap();
    ctx.register_native_function("exp", 1, |args| args[0].exp())
        .unwrap();
    ctx.register_native_function("log", 1, |args| args[0].ln())
        .unwrap();
    ctx.register_native_function("log10", 1, |args| args[0].log10())
        .unwrap();
    ctx.register_native_function("pow", 2, |args| args[0].powf(args[1]))
        .unwrap();
    ctx.register_native_function("atan2", 2, |args| args[0].atan2(args[1]))
        .unwrap();
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
    ctx.register_native_function("min", 2, |args| args[0].min(args[1]))
        .unwrap();
    ctx.register_native_function("max", 2, |args| args[0].max(args[1]))
        .unwrap();
    ctx.register_native_function("fmod", 2, |args| args[0] % args[1])
        .unwrap();

    let ctx = Rc::new(ctx);
    let arena = Bump::new();
    let mut builder = Expression::new(&arena);

    // Add the same 7 expressions
    let expressions = vec![
        "a*sin(b*3.14159/180) + c*cos(d*3.14159/180) + sqrt(e*e + f*f)",
        "exp(g/10) * log(h+1) + pow(i, 0.5) * j",
        "((a > 5) && (b < 10)) * c + ((d >= e) || (f != g)) * h + min(i, j)",
        "sqrt(pow(a-e, 2) + pow(b-f, 2)) + atan2(c-g, d-h) * (i+j)/2",
        "abs(a-b) * sign(c-d) + max(e, f) * min(g, h) + fmod(i*j, 10)",
        "(a+b+c)/3 * sin((d+e+f)*3.14159/6) + log10(g*h+1) - exp(-i*j/100)",
        "a + b * c - d / (e + 0.001) + pow(f, g) * h - i + j",
    ];

    let mut expr_indices = Vec::new();
    for expr in &expressions {
        let idx = builder.add_expression(expr).unwrap();
        expr_indices.push(idx);
    }

    // Add 10 parameters
    let param_names = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];
    let mut param_indices = Vec::new();
    for (i, name) in param_names.iter().enumerate() {
        let idx = builder.add_parameter(name, (i + 1) as f64 * 1.5).unwrap();
        param_indices.push(idx);
    }

    // Initial evaluation
    builder.eval(&ctx).unwrap();

    println!("Warming up...");
    // Warm up
    for i in 0..1000 {
        for (p, &idx) in param_indices.iter().enumerate() {
            builder
                .set_param(idx, (p + 1) as f64 * 1.5 + i as f64 * 0.001)
                .unwrap();
        }
        builder.eval(&ctx).unwrap();
    }

    // Test 1: Time just the eval call
    println!("\nTest 1: Time just eval() call");
    const EVAL_ITERATIONS: usize = 100_000;
    let start = Instant::now();

    for _ in 0..EVAL_ITERATIONS {
        builder.eval(&ctx).unwrap();
    }

    let eval_duration = start.elapsed();
    let eval_only_us = eval_duration.as_secs_f64() * 1e6 / EVAL_ITERATIONS as f64;
    println!("  Time per eval (7 expressions): {:.3} µs", eval_only_us);
    println!("  Time per expression: {:.3} µs", eval_only_us / 7.0);
    println!("  Evaluations per second: {:.0}", 1e6 / eval_only_us);

    // Test 2: Time parameter updates only
    println!("\nTest 2: Time parameter updates only");
    let start = Instant::now();

    for i in 0..EVAL_ITERATIONS {
        for (p, &idx) in param_indices.iter().enumerate() {
            builder
                .set_param(idx, (p + 1) as f64 * 1.5 + i as f64 * 0.001)
                .unwrap();
        }
    }

    let param_duration = start.elapsed();
    let param_update_us = param_duration.as_secs_f64() * 1e6 / EVAL_ITERATIONS as f64;
    println!("  Time for 10 param updates: {:.3} µs", param_update_us);
    println!("  Time per param update: {:.3} µs", param_update_us / 10.0);

    // Test 3: Time full cycle (params + eval)
    println!("\nTest 3: Time full cycle (params + eval)");
    const FULL_ITERATIONS: usize = 10_000;
    let start = Instant::now();

    for i in 0..FULL_ITERATIONS {
        // Update all 10 parameters
        for (p, &idx) in param_indices.iter().enumerate() {
            builder
                .set_param(idx, (p + 1) as f64 * 1.5 + i as f64 * 0.001)
                .unwrap();
        }
        // Evaluate
        builder.eval(&ctx).unwrap();
    }

    let full_duration = start.elapsed();
    let full_cycle_us = full_duration.as_secs_f64() * 1e6 / FULL_ITERATIONS as f64;
    println!("  Time per full cycle: {:.3} µs", full_cycle_us);
    println!("  Rate: {:.0} Hz", 1e6 / full_cycle_us);
    println!("  Breakdown:");
    println!(
        "    Parameter updates: {:.3} µs ({:.1}%)",
        param_update_us,
        (param_update_us / full_cycle_us) * 100.0
    );
    println!(
        "    Evaluation: {:.3} µs ({:.1}%)",
        eval_only_us,
        (eval_only_us / full_cycle_us) * 100.0
    );
}

