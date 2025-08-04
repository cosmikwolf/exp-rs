use bumpalo::Bump;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use exp_rs::{EvalContext, Expression};
use std::rc::Rc;

fn bench_arena_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("arena_performance");

    // Test expressions from your use case
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

    group.bench_function("arena_batch_1000hz", |b| {
        // Setup once per benchmark iteration
        let arena = Bump::with_capacity(128 * 1024); // 128KB arena
        let mut builder = Expression::new(&arena);

        // Add parameters
        for name in &param_names {
            builder.add_parameter(name, 0.0).unwrap();
        }

        // Add expressions
        for expr in &expressions {
            builder.add_expression(expr).unwrap();
        }

        let ctx = Rc::new(EvalContext::new());

        b.iter(|| {
            // Simulate 100ms at 1000Hz (100 evaluations)
            for i in 0..100 {
                // Update parameters
                for (j, name) in param_names.iter().enumerate() {
                    builder.set_param_by_name(name, (i + j) as f64).unwrap();
                }

                // Evaluate all expressions
                builder.eval(&ctx).unwrap();

                // Use results to prevent optimization
                black_box(builder.get_all_results());
            }
        });
    });

    // Benchmark parsing into arena
    group.bench_function("parse_7_expressions_arena", |b| {
        b.iter(|| {
            let arena = Bump::new();
            let mut parsed = Vec::new();

            for expr in &expressions {
                let mut expr_builder = Expression::parse(expr, &arena).unwrap();
                parsed.push(expr_builder);
            }

            black_box(parsed);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_arena_performance);
criterion_main!(benches);

