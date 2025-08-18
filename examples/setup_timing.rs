use bumpalo::Bump;
use exp_rs::{EvalContext, Expression};
use std::rc::Rc;
use std::time::Instant;

fn create_test_context() -> Rc<EvalContext> {
    let mut ctx = EvalContext::new();

    // Register basic math functions
    let _ = ctx.register_native_function("abs", 1, |args| args[0].abs())
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
    let _ = ctx.register_native_function("sin", 1, |args| args[0].sin())
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
    println!("=== Setup Time Analysis ===\n");

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

    // Measure context creation time
    println!("1. Context Creation Time");
    let iterations = 1000;

    let start = Instant::now();
    for _ in 0..iterations {
        let _ctx = create_test_context();
    }
    let ctx_duration = start.elapsed();
    let ctx_us = ctx_duration.as_secs_f64() * 1e6 / iterations as f64;
    println!("   Average time: {:.1} µs", ctx_us);

    // Create one context for subsequent tests
    let _ctx = create_test_context();

    // Measure arena creation time
    println!("\n2. Arena Creation Time");
    let start = Instant::now();
    for _ in 0..iterations {
        let _arena = Bump::new();
    }
    let arena_duration = start.elapsed();
    let arena_us = arena_duration.as_secs_f64() * 1e6 / iterations as f64;
    println!("   Average time: {:.1} µs", arena_us);

    // Measure BatchBuilder creation time (just the builder, no expressions)
    println!("\n3. BatchBuilder Creation Time (empty)");
    let arena = Bump::new();
    let start = Instant::now();
    for _ in 0..iterations {
        let _builder = Expression::new(&arena);
    }
    let builder_duration = start.elapsed();
    let builder_us = builder_duration.as_secs_f64() * 1e6 / iterations as f64;
    println!("   Average time: {:.1} µs", builder_us);

    // Measure parameter addition time
    println!("\n4. Adding 10 Parameters");
    let mut total_param_time = 0.0;
    for _ in 0..100 {
        // Fewer iterations due to arena reset
        let arena = Bump::new();
        let mut builder = Expression::new(&arena);

        let start = Instant::now();
        for name in &param_names {
            builder.add_parameter(name, 0.0).unwrap();
        }
        let duration = start.elapsed();
        total_param_time += duration.as_secs_f64();
    }
    let param_us = total_param_time * 1e6 / 100.0;
    println!("   Average time: {:.1} µs", param_us);
    println!("   Per parameter: {:.1} µs", param_us / 10.0);

    // Measure expression parsing and addition time
    println!("\n5. Parsing and Adding 7 Expressions");
    let mut total_expr_time = 0.0;
    for _ in 0..100 {
        // Fewer iterations due to arena reset
        let arena = Bump::new();
        let mut builder = Expression::new(&arena);

        // Add parameters first
        for name in &param_names {
            builder.add_parameter(name, 0.0).unwrap();
        }

        let start = Instant::now();
        for expr in &expressions {
            builder.add_expression(expr).unwrap();
        }
        let duration = start.elapsed();
        total_expr_time += duration.as_secs_f64();
    }
    let expr_us = total_expr_time * 1e6 / 100.0;
    println!("   Average time: {:.1} µs", expr_us);
    println!("   Per expression: {:.1} µs", expr_us / 7.0);

    // Measure complete setup time
    println!("\n6. Complete Setup Time");
    println!("   (Context + Arena + Builder + 10 params + 7 expressions)");

    let mut total_setup_time = 0.0;
    for _ in 0..100 {
        let start = Instant::now();

        // Complete setup
        let ctx = create_test_context();
        let arena = Bump::new();
        let mut builder = Expression::new(&arena);

        // Add parameters
        for name in &param_names {
            builder.add_parameter(name, 0.0).unwrap();
        }

        // Add expressions
        for expr in &expressions {
            builder.add_expression(expr).unwrap();
        }

        // First evaluation (may have additional setup cost)
        builder.eval(&ctx).unwrap();

        let duration = start.elapsed();
        total_setup_time += duration.as_secs_f64();
    }
    let total_us = total_setup_time * 1e6 / 100.0;
    println!("   Average time: {:.1} µs", total_us);

    // Breakdown
    println!("\n7. Setup Time Breakdown");
    println!(
        "   Context creation:    {:>6.1} µs ({:>4.1}%)",
        ctx_us,
        (ctx_us / total_us) * 100.0
    );
    println!(
        "   Arena creation:      {:>6.1} µs ({:>4.1}%)",
        arena_us,
        (arena_us / total_us) * 100.0
    );
    println!(
        "   Builder creation:    {:>6.1} µs ({:>4.1}%)",
        builder_us,
        (builder_us / total_us) * 100.0
    );
    println!(
        "   Add parameters:      {:>6.1} µs ({:>4.1}%)",
        param_us,
        (param_us / total_us) * 100.0
    );
    println!(
        "   Parse expressions:   {:>6.1} µs ({:>4.1}%)",
        expr_us,
        (expr_us / total_us) * 100.0
    );
    let first_eval_us = total_us - (ctx_us + arena_us + builder_us + param_us + expr_us);
    println!(
        "   First evaluation:    {:>6.1} µs ({:>4.1}%)",
        first_eval_us,
        (first_eval_us / total_us) * 100.0
    );
    println!("   ─────────────────────────────────────────");
    println!("   Total:               {:>6.1} µs", total_us);

    // Amortization analysis
    println!("\n8. Amortization Analysis");
    println!("   Setup cost: {:.1} µs", total_us);
    println!("   Per-evaluation cost: ~13.7 µs");
    println!();
    println!(
        "   Break-even point: {} evaluations",
        (total_us / 13.7) as usize + 1
    );
    println!("   At 1000 Hz for 1 second:");
    println!(
        "     Setup overhead: {:.2}%",
        (total_us / 1_000_000.0) * 100.0
    );
    println!(
        "     Amortized setup cost: {:.3} µs per evaluation",
        total_us / 1000.0
    );
}
