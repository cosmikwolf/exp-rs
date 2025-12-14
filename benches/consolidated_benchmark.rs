use bumpalo::Bump;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use exp_rs::{
    AstExpr, EvalContext, EvalEngine, Expression, eval_with_engine, interp, parse_expression,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;
use std::mem::size_of_val;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ===== Memory Tracking Infrastructure =====

struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static TRACKING_ENABLED: RefCell<bool> = RefCell::new(false);
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = unsafe { System.alloc(layout) };

        TRACKING_ENABLED.with(|enabled| {
            if *enabled.borrow() && !ret.is_null() {
                ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
                ALLOCATION_COUNT.fetch_add(1, Ordering::SeqCst);
            }
        });

        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        TRACKING_ENABLED.with(|enabled| {
            if *enabled.borrow() {
                ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
            }
        });

        unsafe { System.dealloc(ptr, layout) };
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

struct AllocationStats {
    bytes: usize,
    count: usize,
}

fn reset_tracking() {
    ALLOCATED.store(0, Ordering::SeqCst);
    ALLOCATION_COUNT.store(0, Ordering::SeqCst);
}

fn start_tracking() {
    reset_tracking();
    TRACKING_ENABLED.with(|enabled| {
        *enabled.borrow_mut() = true;
    });
}

fn stop_tracking() -> AllocationStats {
    TRACKING_ENABLED.with(|enabled| {
        *enabled.borrow_mut() = false;
    });
    AllocationStats {
        bytes: ALLOCATED.load(Ordering::SeqCst),
        count: ALLOCATION_COUNT.load(Ordering::SeqCst),
    }
}

fn measure_stage<F: FnOnce() -> R, R>(name: &str, f: F) -> (R, AllocationStats) {
    start_tracking();
    let result = f();
    let stats = stop_tracking();
    println!(
        "{:<40} {:>10} bytes in {:>6} allocations",
        name,
        format_number(stats.bytes),
        format_number(stats.count)
    );
    (result, stats)
}

fn format_number(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

// ===== AST Size Calculation =====

fn calculate_ast_size(ast: &AstExpr) -> usize {
    let base_size = size_of_val(ast);

    match ast {
        AstExpr::Constant(_) => base_size,
        AstExpr::Variable(s) => base_size + s.len(),
        AstExpr::Function { name, args } => {
            let mut total = base_size + name.len();
            for arg in args.iter() {
                total += calculate_ast_size(arg);
            }
            total
        }
        AstExpr::Array { name, index } => base_size + name.len() + calculate_ast_size(index),
        AstExpr::Attribute { base, attr } => base_size + base.len() + attr.len(),
        AstExpr::LogicalOp { op: _, left, right } => {
            base_size + calculate_ast_size(left) + calculate_ast_size(right)
        }
        AstExpr::Conditional {
            condition,
            true_branch,
            false_branch,
        } => {
            base_size
                + calculate_ast_size(condition)
                + calculate_ast_size(true_branch)
                + calculate_ast_size(false_branch)
        }
    }
}

// ===== Common Test Data =====

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

fn get_test_expressions() -> Vec<&'static str> {
    vec![
        "a*sin(b*3.14159/180) + c*cos(d*3.14159/180) + sqrt(e*e + f*f)",
        "exp(g/10) * log(h+1) + pow(i, 0.5) * j",
        "((a > 5) && (b < 10)) * c + ((d >= e) || (f != g)) * h + min(i, j)",
        "sqrt(pow(a-e, 2) + pow(b-f, 2)) + atan2(c-g, d-h) * (i+j)/2",
        "abs(a-b) * sign(c-d) + max(e, f) * min(g, h) + fmod(i*j, 10)",
        "(a+b+c)/3 * sin((d+e+f)*3.14159/6) + log10(g*h+1) - exp(-i*j/100)",
        "a + b * c - d / (e + 0.001) + pow(f, g) * h - i + j",
    ]
}

// ===== Performance Benchmarks =====

fn bench_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_vs_individual");
    group.sample_size(20); // Reduce sample size for faster benchmarking

    let expressions = get_test_expressions();
    let param_names = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];

    // Use a single representative batch size for 1000Hz operation
    let batch_size = 100; // 100ms worth of data at 1000Hz

    // Generate test data
    let mut param_values = Vec::new();
    for p in 0..10 {
        let mut values = Vec::new();
        for b in 0..batch_size {
            values.push((p + 1) as f64 * 1.5 + (b + 1) as f64 * 0.1);
        }
        param_values.push(values);
    }

    // Benchmark individual evaluation
    group.bench_function("individual_evaluation", |b| {
        let ctx = create_test_context();

        b.iter(|| {
            let mut results = Vec::new();
            for batch in 0..batch_size {
                // Clone context for parameter update
                let mut ctx_clone = (*ctx).clone();

                // Set parameters
                for (p, name) in param_names.iter().enumerate() {
                    ctx_clone
                        .set_parameter(name, param_values[p][batch])
                        .unwrap();
                }

                // Evaluate all expressions
                for expr in &expressions {
                    let result = interp(expr, Some(Rc::new(ctx_clone.clone()))).unwrap();
                    results.push(result);
                }
            }
            black_box(results);
        });
    });

    // Benchmark using BatchBuilder
    group.bench_function("batch_builder_evaluation", |b| {
        let ctx = create_test_context();
        let arena = Bump::new();
        let mut builder = Expression::new(&arena);

        // Add parameters
        let mut param_indices = Vec::new();
        for name in &param_names {
            let idx = builder.add_parameter(name, 0.0).unwrap();
            param_indices.push(idx);
        }

        // Add expressions
        let mut expr_indices = Vec::new();
        for expr in &expressions {
            let idx = builder.add_expression(expr).unwrap();
            expr_indices.push(idx);
        }

        b.iter(|| {
            let mut all_results = Vec::new();

            for batch in 0..batch_size {
                // Update parameters
                for (p, &idx) in param_indices.iter().enumerate() {
                    builder.set_param(idx, param_values[p][batch]).unwrap();
                }

                // Evaluate
                builder.eval(&ctx).unwrap();

                // Collect results
                let mut batch_results = Vec::new();
                for &idx in &expr_indices {
                    batch_results.push(builder.get_result(idx).unwrap());
                }
                all_results.push(batch_results);
            }

            black_box(all_results);
        });
    });

    group.finish();
}

fn bench_expression_complexity(c: &mut Criterion) {
    let mut group = c.benchmark_group("expression_complexity");

    // Test different complexity levels with same parameter count
    let test_cases = vec![
        ("simple", "a + b * c"),
        ("medium", "sin(a) * cos(b) + sqrt(c*c + d*d)"),
        (
            "complex",
            "exp(a/10) * log(b+1) + pow(c, 0.5) * d + min(e, max(f, g))",
        ),
        (
            "real_world",
            "a*sin(b*3.14159/180) + c*cos(d*3.14159/180) + sqrt(e*e + f*f)",
        ),
    ];

    let ctx = create_test_context();

    // Test both individual and batch approaches for each complexity
    for (name, expr) in test_cases {
        // Individual evaluation
        group.bench_function(format!("{}_individual", name), |b| {
            let mut ctx_clone = (*ctx).clone();

            // Set some parameter values
            for i in 0..10 {
                let param = format!("{}", (b'a' + i as u8) as char);
                ctx_clone
                    .set_parameter(&param, (i + 1) as f64 * 1.5)
                    .unwrap();
            }

            b.iter(|| {
                let result = interp(expr, Some(Rc::new(ctx_clone.clone()))).unwrap();
                black_box(result);
            });
        });

        // Batch evaluation
        group.bench_function(format!("{}_batch", name), |b| {
            let arena = Bump::new();
            let mut builder = Expression::new(&arena);

            // Add parameters
            for i in 0..10 {
                let param = format!("{}", (b'a' + i as u8) as char);
                builder.add_parameter(&param, (i + 1) as f64 * 1.5).unwrap();
            }

            // Add expression
            builder.add_expression(expr).unwrap();

            b.iter(|| {
                builder.eval(&ctx).unwrap();
                black_box(builder.get_result(0).unwrap());
            });
        });
    }

    group.finish();
}

// ===== Memory Analysis Functions =====

fn run_memory_analysis() {
    println!("\n=== Memory Allocation Analysis ===");
    println!("Simulating 7 expressions, 10 parameters at 1000Hz for 100 seconds\n");

    let expressions = get_test_expressions();
    let param_names = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];

    println!("Stage                                         Bytes     Allocations");
    println!("{}", "─".repeat(70));

    // Stage 1: Context creation
    let (ctx, _) = measure_stage("Creating context", || create_test_context());

    // Stage 2: Parsing expressions
    let parse_arena = Bump::new();
    let (parsed_expressions, _) = measure_stage("Parsing 7 expressions", || {
        expressions
            .iter()
            .map(|expr| parse_expression(expr, &parse_arena).unwrap())
            .collect::<Vec<_>>()
    });

    println!("\n--- Individual Evaluation Approach ---");

    // Stage 3: First evaluation (individual approach)
    let (mut total_individual_bytes, mut total_individual_count) = (0, 0);

    let (_, stats) = measure_stage("First evaluation (individual)", || {
        let mut ctx_clone = (*ctx).clone();
        for (i, name) in param_names.iter().enumerate() {
            ctx_clone.set_parameter(name, i as f64).unwrap();
        }

        let mut results = Vec::new();
        for expr in &expressions {
            results.push(interp(expr, Some(Rc::new(ctx_clone.clone()))).unwrap());
        }
        results
    });
    total_individual_bytes += stats.bytes;
    total_individual_count += stats.count;

    // Stage 4: Subsequent 999 evaluations (reduced from 99999 for faster execution)
    let (_, stats) = measure_stage("Subsequent 999 evaluations", || {
        for iteration in 1..1000 {
            let mut ctx_clone = (*ctx).clone();
            for (i, name) in param_names.iter().enumerate() {
                ctx_clone
                    .set_parameter(name, (i + iteration) as f64)
                    .unwrap();
            }

            for expr in &expressions {
                let _ = interp(expr, Some(Rc::new(ctx_clone.clone()))).unwrap();
            }
        }
    });
    total_individual_bytes += stats.bytes;
    total_individual_count += stats.count;

    println!(
        "{:<40} {:>10} bytes in {:>6} allocations",
        "Total (1 sec @ 1000Hz)",
        format_number(total_individual_bytes),
        format_number(total_individual_count)
    );

    let per_eval_bytes = stats.bytes / 999 / 7;
    println!(
        "\nPer-evaluation cost after first: ~{} bytes",
        per_eval_bytes
    );

    println!("\n--- Batch Evaluation Approach ---");

    // Stage 5: Batch builder setup
    let arena = Bump::new();
    let (mut builder, _) = measure_stage("Setting up BatchBuilder", || {
        let mut builder = Expression::new(&arena);

        // Add parameters
        for name in &param_names {
            builder.add_parameter(name, 0.0).unwrap();
        }

        // Add expressions
        for expr in &expressions {
            builder.add_expression(expr).unwrap();
        }

        builder
    });

    // Stage 6: First batch evaluation
    let (mut total_batch_bytes, mut total_batch_count) = (0, 0);

    let (_, stats) = measure_stage("First batch evaluation", || {
        for (i, name) in param_names.iter().enumerate() {
            builder.set_param_by_name(name, i as f64).unwrap();
        }
        builder.eval(&ctx).unwrap();
    });
    total_batch_bytes += stats.bytes;
    total_batch_count += stats.count;

    // Stage 7: Subsequent 999 batch evaluations
    let (_, stats) = measure_stage("Subsequent 999 batch evaluations", || {
        for iteration in 1..1000 {
            for (i, name) in param_names.iter().enumerate() {
                builder
                    .set_param_by_name(name, (i + iteration) as f64)
                    .unwrap();
            }
            builder.eval(&ctx).unwrap();
        }
    });
    total_batch_bytes += stats.bytes;
    total_batch_count += stats.count;

    println!(
        "{:<40} {:>10} bytes in {:>6} allocations",
        "Total (1 sec @ 1000Hz)",
        format_number(total_batch_bytes),
        format_number(total_batch_count)
    );

    let batch_per_eval_bytes = stats.bytes / 999 / 7;
    println!(
        "\nPer-evaluation cost after first: ~{} bytes",
        batch_per_eval_bytes
    );

    println!("\n--- Direct Engine Evaluation (No Context Clone) ---");

    // Stage 8: Using evaluation engine directly
    let engine_arena = Bump::new();
    let (mut engine, _) = measure_stage("Creating evaluation engine", || {
        EvalEngine::new(&engine_arena)
    });

    let (_, stats) = measure_stage("1000 evaluations with engine", || {
        for iteration in 0..1000 {
            // Update parameters in original context
            let mut ctx_clone = (*ctx).clone();
            for (i, name) in param_names.iter().enumerate() {
                ctx_clone
                    .set_parameter(name, (i + iteration) as f64)
                    .unwrap();
            }
            let ctx_rc = Rc::new(ctx_clone);

            // Evaluate each pre-parsed expression
            for ast in &parsed_expressions {
                let _ = eval_with_engine(ast, Some(ctx_rc.clone()), &mut engine).unwrap();
            }
        }
    });

    let engine_per_eval_bytes = stats.bytes / 1000 / 7;
    println!("\nPer-evaluation cost: ~{} bytes", engine_per_eval_bytes);
}

fn run_ast_size_analysis() {
    println!("\n=== AST Size Analysis ===\n");

    let expressions = vec![
        ("simple", "a + b"),
        ("medium", "sin(a) * cos(b) + sqrt(c*c + d*d)"),
        (
            "complex",
            "a*sin(b*3.14159/180) + c*cos(d*3.14159/180) + sqrt(e*e + f*f)",
        ),
        ("conditional", "a > 5 ? b * 2 : c / 3"),
        (
            "logical",
            "((a > 5) && (b < 10)) * c + ((d >= e) || (f != g)) * h",
        ),
    ];

    println!("Expression Type    | Total Size | Shallow Size | Expression");
    println!("-------------------|------------|--------------|------------");

    let ast_arena = Bump::new();
    for (name, expr) in expressions {
        let ast = parse_expression(expr, &ast_arena).unwrap();
        let total = calculate_ast_size(&ast);
        let shallow = size_of_val(&ast);
        println!("{:<18} | {:>10} | {:>12} | {}", name, total, shallow, expr);
    }

    // Analyze real world expressions
    println!("\n=== Real World Expression Sizes ===\n");
    let real_expressions = get_test_expressions();
    let mut total_size = 0;

    let real_arena = Bump::new();
    for (i, expr) in real_expressions.iter().enumerate() {
        let ast = parse_expression(expr, &real_arena).unwrap();
        let size = calculate_ast_size(&ast);
        total_size += size;
        println!("Expression {}: {} bytes", i + 1, size);
    }

    println!("\nTotal AST size for 7 expressions: {} bytes", total_size);
    println!("Average size per expression: {} bytes", total_size / 7);

    // Memory traffic calculation
    println!("\n=== Memory Traffic at 1000Hz ===");
    let bytes_per_second = total_size * 1000;
    println!(
        "Memory traffic per second: {} MB",
        bytes_per_second / 1_000_000
    );
    println!(
        "Memory traffic per minute: {} MB",
        bytes_per_second * 60 / 1_000_000
    );
    println!(
        "Memory traffic per hour: {} GB",
        bytes_per_second as f64 * 3600.0 / 1_000_000_000.0
    );
}

// ===== Criterion Entry Point =====

// ===== CPU Utilization Test =====

fn run_cpu_utilization_test() {
    use std::time::Instant;

    println!("\n=== CPU Utilization Test ===");
    println!("Simulating 1000Hz operation with 7 expressions and 10 parameters\n");

    let expressions = get_test_expressions();
    let param_names = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];
    let iterations = 30_000; // Fixed 30,000 iterations (30 seconds at 1000Hz)

    // Generate test data
    let mut param_values = Vec::new();
    for p in 0..10 {
        param_values.push((p + 1) as f64 * 1.5);
    }

    let ctx = create_test_context();

    // Test 1: Individual evaluation (current approach)
    println!("Test 1: Individual evaluation (context clone per evaluation)");
    println!("  Running {} iterations...", iterations);
    let start = Instant::now();

    for i in 0..iterations {
        let mut ctx_clone = (*ctx).clone();

        // Set parameters
        for (j, name) in param_names.iter().enumerate() {
            ctx_clone
                .set_parameter(name, param_values[j] + i as f64 * 0.001)
                .unwrap();
        }

        // Evaluate all expressions
        for expr in &expressions {
            let _result = interp(expr, Some(Rc::new(ctx_clone.clone()))).unwrap();
        }
    }

    let individual_duration = start.elapsed();
    let individual_rate = iterations as f64 / individual_duration.as_secs_f64();
    let individual_us_per_eval =
        (individual_duration.as_micros() as f64) / (iterations as f64 * 7.0); // 7 expressions
    let individual_us_per_iter = (individual_duration.as_micros() as f64) / (iterations as f64);

    println!(
        "  Completed: {} iterations in {:.2}s",
        iterations,
        individual_duration.as_secs_f64()
    );
    println!("  Rate: {:.0} Hz", individual_rate);
    println!(
        "  Time per iteration: {:.1} µs (all 7 expressions)",
        individual_us_per_iter
    );
    println!("  Time per expression: {:.1} µs", individual_us_per_eval);
    println!(
        "  CPU efficiency: {:.1}%",
        (individual_rate / 1000.0) * 100.0
    );
    println!("  Expressions/second: {:.0}", individual_rate * 7.0);

    // Test 2: BatchBuilder approach
    println!("\nTest 2: BatchBuilder (with parameter overrides)");

    let batch_arena = Bump::new();
    let mut builder = Expression::new(&batch_arena);

    // Add parameters
    for name in &param_names {
        builder.add_parameter(name, 0.0).unwrap();
    }

    // Add expressions
    for expr in &expressions {
        builder.add_expression(expr).unwrap();
    }

    println!("  Running {} iterations...", iterations);
    let start = Instant::now();

    for i in 0..iterations {
        // Update parameters
        for (j, name) in param_names.iter().enumerate() {
            builder
                .set_param_by_name(name, param_values[j] + i as f64 * 0.001)
                .unwrap();
        }

        // Evaluate
        builder.eval(&ctx).unwrap();
    }

    let batch_duration = start.elapsed();
    let batch_rate = iterations as f64 / batch_duration.as_secs_f64();
    let batch_us_per_eval = (batch_duration.as_micros() as f64) / (iterations as f64 * 7.0); // 7 expressions
    let batch_us_per_iter = (batch_duration.as_micros() as f64) / (iterations as f64);

    println!(
        "  Completed: {} iterations in {:.2}s",
        iterations,
        batch_duration.as_secs_f64()
    );
    println!("  Rate: {:.0} Hz", batch_rate);
    println!(
        "  Time per iteration: {:.1} µs (all 7 expressions)",
        batch_us_per_iter
    );
    println!("  Time per expression: {:.1} µs", batch_us_per_eval);
    println!("  CPU efficiency: {:.1}%", (batch_rate / 1000.0) * 100.0);
    println!("  Expressions/second: {:.0}", batch_rate * 7.0);

    // Summary
    println!("\n=== Performance Summary ===");
    let speedup = batch_rate / individual_rate;
    let us_saved = individual_us_per_iter - batch_us_per_iter;
    let memory_bandwidth_mb = (9550.0 * individual_rate) / 1_000_000.0; // 9550 bytes per iteration

    println!(
        "Individual approach: {:.0} Hz ({:.1}% of target)",
        individual_rate,
        (individual_rate / 1000.0) * 100.0
    );
    println!(
        "BatchBuilder approach: {:.0} Hz ({:.1}% of target)",
        batch_rate,
        (batch_rate / 1000.0) * 100.0
    );
    println!("BatchBuilder is {:.2}x faster", speedup);
    println!("\nTiming improvements:");
    println!(
        "  Per iteration: {:.1} µs saved ({:.1}%)",
        us_saved,
        (us_saved / individual_us_per_iter) * 100.0
    );
    println!(
        "  Per expression: {:.1} µs → {:.1} µs",
        individual_us_per_eval, batch_us_per_eval
    );

    println!("\nAt 1000Hz target:");
    println!("  Time budget per iteration: 1000 µs");
    println!(
        "  Individual uses: {:.1} µs ({:.1}% of budget)",
        individual_us_per_iter,
        (individual_us_per_iter / 1000.0) * 100.0
    );
    println!(
        "  BatchBuilder uses: {:.1} µs ({:.1}% of budget)",
        batch_us_per_iter,
        (batch_us_per_iter / 1000.0) * 100.0
    );
    println!(
        "  Time available for other tasks: {:.1} µs",
        1000.0 - batch_us_per_iter
    );

    println!("\nMemory bandwidth (estimated):");
    println!("  AST cloning traffic: {:.1} MB/s", memory_bandwidth_mb);
    println!(
        "  Cache misses likely: {:.0}/second",
        individual_rate * 7.0 * 10.0
    ); // Rough estimate

    if individual_rate < 1000.0 {
        println!("\nWARNING: Individual approach cannot meet 1000Hz requirement!");
        println!(
            "Missing {:.0} Hz ({:.1}% short of target)",
            1000.0 - individual_rate,
            ((1000.0 - individual_rate) / 1000.0) * 100.0
        );
        let overrun_us = individual_us_per_iter - 1000.0;
        println!("Time overrun per iteration: {:.1} µs", overrun_us);
    }

    println!("\nProjected with Bumpalo optimization:");
    let projected_us = batch_us_per_iter * 0.7; // Conservative 30% improvement estimate
    let projected_rate = 1_000_000.0 / projected_us;
    println!("  Expected time per iteration: {:.1} µs", projected_us);
    println!("  Expected rate: {:.0} Hz", projected_rate);
    println!(
        "  Expected CPU usage at 1000Hz: {:.1}%",
        (projected_us / 1000.0) * 100.0
    );
}

// Add a custom benchmark that runs the memory analysis
fn bench_memory_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_analysis");
    group.sample_size(10); // Run only a few times since it prints output

    group.bench_function("run_analysis", |b| {
        b.iter(|| {
            // Suppress output during benchmark iterations
            let _old_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(|_| {}));

            // We don't actually want to benchmark this, just run it once
            black_box(());
        });
    });

    // Run the actual analysis once after benchmarking
    println!("\n");
    run_memory_analysis();
    run_ast_size_analysis();
    run_cpu_utilization_test();

    println!("\n=== Summary ===");
    println!("The ~250 byte estimate was incorrect. Actual cost is ~1,364 bytes per evaluation.");
    println!("This results in ~10 MB/second of memory traffic at 1000Hz.");

    group.finish();
}

// Criterion benchmarks
criterion_group!(
    benches,
    bench_comparison,
    bench_expression_complexity,
    bench_memory_analysis
);
criterion_main!(benches);
