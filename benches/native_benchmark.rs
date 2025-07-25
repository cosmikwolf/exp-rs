use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use exp_rs::{interp, EvalContext, BatchBuilder};
use std::rc::Rc;
use std::time::Instant;

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

fn bench_individual_vs_batch_builder(c: &mut Criterion) {
    let mut group = c.benchmark_group("individual_vs_batch_builder");
    
    let expressions = vec![
        "a*sin(b*3.14159/180) + c*cos(d*3.14159/180) + sqrt(e*e + f*f)",
        "exp(g/10) * log(h+1) + pow(i, 0.5) * j",
        "((a > 5) && (b < 10)) * c + ((d >= e) || (f != g)) * h + min(i, j)",
        "sqrt(pow(a-e, 2) + pow(b-f, 2)) + atan2(c-g, d-h) * (i+j)/2",
        "abs(a-b) * sign(c-d) + max(e, f) * min(g, h) + fmod(i*j, 10)",
        "(a+b+c)/3 * sin((d+e+f)*3.14159/6) + log10(g*h+1) - exp(-i*j/100)",
    ];
    
    let param_names = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];
    
    for batch_size in [10, 50, 100].iter() {
        // Generate test data
        let mut param_values = Vec::new();
        for p in 0..10 {
            let mut values = Vec::new();
            for b in 0..*batch_size {
                values.push((p + 1) as f64 * 1.5 + (b + 1) as f64 * 0.1);
            }
            param_values.push(values);
        }
        
        // Benchmark individual evaluation
        group.bench_with_input(
            BenchmarkId::new("individual", batch_size),
            batch_size,
            |b, &size| {
                let ctx = create_test_context();
                
                b.iter(|| {
                    let mut results = Vec::new();
                    for batch in 0..size {
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
            }
        );
        
        // Benchmark using BatchBuilder
        group.bench_with_input(
            BenchmarkId::new("batch_builder", batch_size),
            batch_size,
            |b, &size| {
                use exp_rs::BatchBuilder;
                
                let ctx = create_test_context();
                let mut builder = BatchBuilder::new();
                
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
                    
                    for batch in 0..size {
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
            }
        );
    }
    
    group.finish();
}

// Simple performance test
fn bench_expression_complexity(c: &mut Criterion) {
    let mut group = c.benchmark_group("expression_complexity");
    
    let simple_expr = "a + b * c";
    let medium_expr = "sin(a) * cos(b) + sqrt(c*c + d*d)";
    let complex_expr = "exp(a/10) * log(b+1) + pow(c, 0.5) * d + min(e, max(f, g))";
    
    let ctx = create_test_context();
    
    for (name, expr) in [("simple", simple_expr), ("medium", medium_expr), ("complex", complex_expr)].iter() {
        group.bench_function(*name, |b| {
            let mut ctx_clone = (*ctx).clone();
            
            // Set some parameter values
            for i in 0..10 {
                let param = format!("{}", (b'a' + i as u8) as char);
                ctx_clone.set_parameter(&param, (i + 1) as f64 * 1.5).unwrap();
            }
            
            b.iter(|| {
                let result = interp(expr, Some(Rc::new(ctx_clone.clone()))).unwrap();
                black_box(result);
            });
        });
    }
    
    group.finish();
}

// Real-world timing test
fn timing_test() {
    println!("\n=== Real-world Timing Test ===");
    
    let expressions = vec![
        "a*sin(b*3.14159/180) + c*cos(d*3.14159/180) + sqrt(e*e + f*f)",
        "exp(g/10) * log(h+1) + pow(i, 0.5) * j",
        "((a > 5) && (b < 10)) * c + ((d >= e) || (f != g)) * h + min(i, j)",
        "sqrt(pow(a-e, 2) + pow(b-f, 2)) + atan2(c-g, d-h) * (i+j)/2",
        "abs(a-b) * sign(c-d) + max(e, f) * min(g, h) + fmod(i*j, 10)",
        "(a+b+c)/3 * sin((d+e+f)*3.14159/6) + log10(g*h+1) - exp(-i*j/100)",
    ];
    
    let param_names = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];
    let batch_size = 50;
    let iterations = 100;
    
    // Generate test data
    let mut param_values = Vec::new();
    for p in 0..10 {
        let mut values = Vec::new();
        for b in 0..batch_size {
            values.push((p + 1) as f64 * 1.5 + (b + 1) as f64 * 0.1);
        }
        param_values.push(values);
    }
    
    let ctx = create_test_context();
    
    // Test 1: Individual evaluation
    println!("\nTest 1: Individual evaluation");
    let start = Instant::now();
    
    for _ in 0..iterations {
        for batch in 0..batch_size {
            let mut ctx_clone = (*ctx).clone();
            
            // Set parameters
            for (p, name) in param_names.iter().enumerate() {
                ctx_clone.set_parameter(name, param_values[p][batch]).unwrap();
            }
            
            // Evaluate all expressions
            for expr in &expressions {
                let _result = interp(expr, Some(Rc::new(ctx_clone.clone()))).unwrap();
            }
        }
    }
    
    let individual_duration = start.elapsed();
    println!("  Time: {:?}", individual_duration);
    println!("  Total evaluations: {}", expressions.len() * iterations * batch_size);
    
    // Test 2: BatchBuilder
    println!("\nTest 2: BatchBuilder with engine reuse");
    
    let mut builder = BatchBuilder::new();
    
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
    
    let start = Instant::now();
    
    for _ in 0..iterations {
        for batch in 0..batch_size {
            // Update parameters
            for (p, &idx) in param_indices.iter().enumerate() {
                builder.set_param(idx, param_values[p][batch]).unwrap();
            }
            
            // Evaluate
            builder.eval(&ctx).unwrap();
        }
    }
    
    let batch_duration = start.elapsed();
    println!("  Time: {:?}", batch_duration);
    println!("  Total evaluations: {}", expressions.len() * iterations * batch_size);
    
    let speedup = individual_duration.as_secs_f64() / batch_duration.as_secs_f64();
    let improvement = (individual_duration.as_secs_f64() - batch_duration.as_secs_f64()) / individual_duration.as_secs_f64() * 100.0;
    
    println!("\n=== Performance Results ===");
    println!("Individual: {:?}", individual_duration);
    println!("BatchBuilder: {:?}", batch_duration);
    println!("Speedup: {:.2}x faster", speedup);
    println!("Performance improvement: {:.1}%", improvement);
}

criterion_group!(benches, bench_individual_vs_batch_builder, bench_expression_complexity);
criterion_main!(benches);

#[test]
fn run_timing_test() {
    timing_test();
}