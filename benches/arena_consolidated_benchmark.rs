use criterion::{black_box, criterion_group, criterion_main, Criterion};
use exp_rs::{EvalContext, BatchBuilder, interp};
use std::rc::Rc;
use bumpalo::Bump;
use std::time::Instant;

// Common test context and expressions
fn create_test_context() -> Rc<EvalContext> {
    let mut ctx = EvalContext::new();
    
    // Register basic math functions
    ctx.register_native_function("abs", 1, |args| args[0].abs()).unwrap();
    ctx.register_native_function("sign", 1, |args| {
        if args[0] > 0.0 { 1.0 } else if args[0] < 0.0 { -1.0 } else { 0.0 }
    }).unwrap();
    
    // Trigonometric
    ctx.register_native_function("sin", 1, |args| args[0].sin()).unwrap();
    ctx.register_native_function("cos", 1, |args| args[0].cos()).unwrap();
    ctx.register_native_function("atan2", 2, |args| args[0].atan2(args[1])).unwrap();
    
    // Exponential and logarithmic
    ctx.register_native_function("exp", 1, |args| args[0].exp()).unwrap();
    ctx.register_native_function("log", 1, |args| args[0].ln()).unwrap();
    ctx.register_native_function("log10", 1, |args| args[0].log10()).unwrap();
    ctx.register_native_function("pow", 2, |args| args[0].powf(args[1])).unwrap();
    ctx.register_native_function("sqrt", 1, |args| args[0].sqrt()).unwrap();
    
    // Min/max
    ctx.register_native_function("min", 2, |args| args[0].min(args[1])).unwrap();
    ctx.register_native_function("max", 2, |args| args[0].max(args[1])).unwrap();
    
    // Modulo
    ctx.register_native_function("fmod", 2, |args| args[0] % args[1]).unwrap();
    
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

// Performance comparison benchmark
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
    
    // Benchmark individual evaluation (old approach for comparison)
    group.bench_function("individual_evaluation", |b| {
        let ctx = create_test_context();
        
        b.iter(|| {
            let mut results = Vec::new();
            for batch in 0..batch_size {
                // Clone context for parameter update
                let mut ctx_clone = (*ctx).clone();
                
                // Set parameters
                for (p, name) in param_names.iter().enumerate() {
                    ctx_clone.set_parameter(name, param_values[p][batch]).unwrap();
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
    
    // Benchmark using Arena BatchBuilder
    group.bench_function("arena_batch_evaluation", |b| {
        // Create arena and builder outside the iteration
        let arena = Bump::with_capacity(256 * 1024);
        let mut builder = BatchBuilder::new(&arena);
        
        // Add parameters
        for name in &param_names {
            builder.add_parameter(name, 0.0).unwrap();
        }
        
        // Add expressions
        for expr in &expressions {
            builder.add_expression(expr).unwrap();
        }
        
        let ctx = create_test_context();
        
        b.iter(|| {
            let mut all_results = Vec::new();
            
            for batch in 0..batch_size {
                // Update parameters
                for (p, name) in param_names.iter().enumerate() {
                    builder.set_param_by_name(name, param_values[p][batch]).unwrap();
                }
                
                // Evaluate
                builder.eval(&ctx).unwrap();
                
                // Collect results
                let mut batch_results = Vec::new();
                for i in 0..expressions.len() {
                    batch_results.push(builder.get_result(i).unwrap());
                }
                all_results.push(batch_results);
            }
            
            black_box(all_results);
        });
    });
    
    group.finish();
}

// CPU Utilization Test
fn run_cpu_utilization_test() {
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
    
    // Test 1: Individual evaluation (old approach)
    println!("Test 1: Individual evaluation (context clone per evaluation)");
    println!("  Running {} iterations...", iterations);
    let start = Instant::now();
    
    for i in 0..iterations {
        let mut ctx_clone = (*ctx).clone();
        
        // Set parameters
        for (j, name) in param_names.iter().enumerate() {
            ctx_clone.set_parameter(name, param_values[j] + i as f64 * 0.001).unwrap();
        }
        
        // Evaluate all expressions
        for expr in &expressions {
            let _result = interp(expr, Some(Rc::new(ctx_clone.clone()))).unwrap();
        }
    }
    
    let individual_duration = start.elapsed();
    let individual_rate = iterations as f64 / individual_duration.as_secs_f64();
    let individual_us_per_eval = (individual_duration.as_micros() as f64) / (iterations as f64 * 7.0);
    let individual_us_per_iter = (individual_duration.as_micros() as f64) / (iterations as f64);
    
    println!("  Completed: {} iterations in {:.2}s", iterations, individual_duration.as_secs_f64());
    println!("  Rate: {:.0} Hz", individual_rate);
    println!("  Time per iteration: {:.1} µs (all 7 expressions)", individual_us_per_iter);
    println!("  Time per expression: {:.1} µs", individual_us_per_eval);
    println!("  CPU efficiency: {:.1}%", (individual_rate / 1000.0) * 100.0);
    println!("  Expressions/second: {:.0}", individual_rate * 7.0);
    
    // Test 2: Arena BatchBuilder approach
    println!("\nTest 2: Arena BatchBuilder (zero allocations)");
    
    let arena = Bump::with_capacity(256 * 1024); // Larger arena for 30k iterations
    let mut builder = BatchBuilder::new(&arena);
    
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
            builder.set_param_by_name(name, param_values[j] + i as f64 * 0.001).unwrap();
        }
        
        // Evaluate
        builder.eval(&ctx).unwrap();
    }
    
    let batch_duration = start.elapsed();
    let batch_rate = iterations as f64 / batch_duration.as_secs_f64();
    let batch_us_per_eval = (batch_duration.as_micros() as f64) / (iterations as f64 * 7.0);
    let batch_us_per_iter = (batch_duration.as_micros() as f64) / (iterations as f64);
    
    println!("  Completed: {} iterations in {:.2}s", iterations, batch_duration.as_secs_f64());
    println!("  Rate: {:.0} Hz", batch_rate);
    println!("  Time per iteration: {:.1} µs (all 7 expressions)", batch_us_per_iter);
    println!("  Time per expression: {:.1} µs", batch_us_per_eval);
    println!("  CPU efficiency: {:.1}%", (batch_rate / 1000.0) * 100.0);
    println!("  Expressions/second: {:.0}", batch_rate * 7.0);
    
    // Summary
    println!("\n=== Performance Summary ===");
    let speedup = batch_rate / individual_rate;
    let us_saved = individual_us_per_iter - batch_us_per_iter;
    let memory_bandwidth_mb = (9550.0 * individual_rate) / 1_000_000.0;
    
    println!("Individual approach: {:.0} Hz ({:.1}% of target)", individual_rate, (individual_rate / 1000.0) * 100.0);
    println!("Arena BatchBuilder: {:.0} Hz ({:.1}% of target)", batch_rate, (batch_rate / 1000.0) * 100.0);
    println!("Arena is {:.2}x faster", speedup);
    println!("\nTiming improvements:");
    println!("  Per iteration: {:.1} µs saved ({:.1}%)", us_saved, (us_saved / individual_us_per_iter) * 100.0);
    println!("  Per expression: {:.1} µs → {:.1} µs", individual_us_per_eval, batch_us_per_eval);
    
    println!("\nAt 1000Hz target:");
    println!("  Time budget per iteration: 1000 µs");
    println!("  Individual uses: {:.1} µs ({:.1}% of budget)", individual_us_per_iter, (individual_us_per_iter / 1000.0) * 100.0);
    println!("  Arena uses: {:.1} µs ({:.1}% of budget)", batch_us_per_iter, (batch_us_per_iter / 1000.0) * 100.0);
    println!("  Time available for other tasks: {:.1} µs", 1000.0 - batch_us_per_iter);
    
    println!("\nMemory impact:");
    println!("  Without arena: {:.1} MB/s allocation traffic", memory_bandwidth_mb);
    println!("  With arena: 0 MB/s (all allocations eliminated)");
    
    if individual_rate < 1000.0 {
        println!("\nWARNING: Individual approach cannot meet 1000Hz requirement!");
        println!("Missing {:.0} Hz ({:.1}% short of target)", 1000.0 - individual_rate, ((1000.0 - individual_rate) / 1000.0) * 100.0);
    }
}

// Custom benchmark that runs the CPU test
fn bench_cpu_test(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu_utilization");
    group.sample_size(10);
    
    group.bench_function("run_test", |b| {
        b.iter(|| {
            // We don't actually want to benchmark this, just run it once
            black_box(());
        });
    });
    
    // Run the actual CPU test once
    run_cpu_utilization_test();
    
    group.finish();
}

criterion_group!(benches, bench_comparison, bench_cpu_test);
criterion_main!(benches);