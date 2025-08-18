use bumpalo::Bump;
use exp_rs::{EvalContext, Expression};
use std::rc::Rc;
use std::time::Instant;

fn main() {
    println!("=== Exact Comparison Test ===");
    println!("Measuring: Set 10 parameters + evaluate 7 expressions");
    println!("No result collection to match C test\n");

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

    for expr in &expressions {
        builder.add_expression(expr).unwrap();
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

    // Warm up
    for _ in 0..1000 {
        builder.eval(&ctx).unwrap();
    }

    // Test matching C benchmark exactly
    println!("Test: Matching C benchmark pattern");
    let batch_sizes = vec![1, 10, 100, 1000];

    for &batch_size in &batch_sizes {
        let iterations = 10000 / batch_size;
        println!(
            "\nBatch size {} (simulating {}ms at 1000Hz):",
            batch_size, batch_size
        );

        let start = Instant::now();

        for _iter in 0..iterations {
            for batch in 0..batch_size {
                // Update parameters (matching C test pattern)
                for p in 0..10 {
                    let value = (p + 1) as f64 * 1.5 + (batch + 1) as f64 * 0.1;
                    builder.set_param(param_indices[p], value).unwrap();
                }

                // Evaluate all 7 expressions
                builder.eval(&ctx).unwrap();
            }
        }

        let duration = start.elapsed();
        let total_us = duration.as_secs_f64() * 1e6;
        let total_evals = (iterations * batch_size * 7) as f64;
        let us_per_eval = total_us / total_evals;
        let us_per_batch = total_us / (iterations * batch_size) as f64;
        let batch_rate = 1e6 / us_per_batch;

        println!("  Total evaluations: {}", total_evals);
        println!("  Total time: {:.2} ms", total_us / 1000.0);
        println!("  Time per batch: {:.3} µs", us_per_batch);
        println!("  Time per expression: {:.3} µs", us_per_eval);
        println!("  Batch rate: {:.0} Hz", batch_rate);
    }
}
