//! Criterion benchmark for `Expression::eval`, the batch evaluation path.
//!
//! Two groups:
//! - `expression_eval`: one expression, one parameter; each iteration does
//!   `set_param` + `eval` + `get_result`. This is the in-repo replacement for
//!   the temporary microbenchmark behind the per-expression table in
//!   PERFORMANCE_PLAN.md.
//! - `native_baseline`: the same formulas as Rust closures, for the ratio.
//! - `batch_eval`: 10 parameter updates + one `eval` of 7 expressions, the
//!   shape of one consumer-firmware tick.

use bumpalo::Bump;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use exp_rs::{EvalContext, Expression, Real};
use std::rc::Rc;

const EXPRESSIONS: &[(&str, &str)] = &[
    ("add_const", "a+5"),
    ("mul_paren", "(a+5)*2"),
    ("quadratic", "a*a + 2*a + 1"),
    ("trig_sqrt", "sin(a)*cos(a) + sqrt(a+1)"),
    ("rational", "1/(a+1)+2/(a+2)+3/(a+3)"),
];

const BATCH_EXPRESSIONS: &[&str] = &[
    "a*sin(b*3.14159/180) + c*cos(d*3.14159/180) + sqrt(e*e + f*f)",
    "exp(g/10) * log(h+1) + pow(i, 0.5) * j",
    "((a > 5) && (b < 10)) * c + ((d >= e) || (f != g)) * h + min(i, j)",
    "sqrt(pow(a-e, 2) + pow(b-f, 2)) + atan2(c-g, d-h) * (i+j)/2",
    "abs(a-b) * sign(c-d) + max(e, f) * min(g, h) + fmod(i*j, 10)",
    "(a+b+c)/3 * sin((d+e+f)*3.14159/6) + log10(g*h+1) - exp(-i*j/100)",
    "a + b * c - d / (e + 0.001) + pow(f, g) * h - i + j",
];

const PARAM_NAMES: &[&str] = &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];

fn bench_expression_eval(c: &mut Criterion) {
    // EvalContext::new() registers the default math functions.
    let ctx = Rc::new(EvalContext::new());
    let mut group = c.benchmark_group("expression_eval");

    for (name, text) in EXPRESSIONS {
        let arena = Bump::new();
        let mut expr = Expression::new(&arena);
        let a = expr.add_parameter("a", 0.5).unwrap();
        expr.add_expression(text).unwrap();

        let mut x: Real = 0.5;
        group.bench_function(*name, |b| {
            b.iter(|| {
                x += 0.001;
                if x > 100.0 {
                    x = 0.5;
                }
                expr.set_param(a, black_box(x)).unwrap();
                expr.eval(&ctx).unwrap();
                black_box(expr.get_result(0).unwrap())
            })
        });
    }
    group.finish();
}

fn bench_native_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("native_baseline");
    let mut x: Real = 0.5;
    let step = |x: &mut Real| {
        *x += 0.001;
        if *x > 100.0 {
            *x = 0.5;
        }
        black_box(*x)
    };

    group.bench_function("add_const", |b| {
        b.iter(|| {
            let a = step(&mut x);
            black_box(a + 5.0)
        })
    });
    group.bench_function("mul_paren", |b| {
        b.iter(|| {
            let a = step(&mut x);
            black_box((a + 5.0) * 2.0)
        })
    });
    group.bench_function("quadratic", |b| {
        b.iter(|| {
            let a = step(&mut x);
            black_box(a * a + 2.0 * a + 1.0)
        })
    });
    group.bench_function("trig_sqrt", |b| {
        b.iter(|| {
            let a = step(&mut x);
            black_box(a.sin() * a.cos() + (a + 1.0).sqrt())
        })
    });
    group.bench_function("rational", |b| {
        b.iter(|| {
            let a = step(&mut x);
            black_box(1.0 / (a + 1.0) + 2.0 / (a + 2.0) + 3.0 / (a + 3.0))
        })
    });
    group.finish();
}

fn bench_batch_eval(c: &mut Criterion) {
    let ctx = Rc::new(EvalContext::new());
    let mut group = c.benchmark_group("batch_eval");

    let arena = Bump::new();
    let mut expr = Expression::new(&arena);
    let mut param_indices = Vec::new();
    for (p, name) in PARAM_NAMES.iter().enumerate() {
        param_indices.push(expr.add_parameter(name, (p + 1) as Real * 1.5).unwrap());
    }
    for text in BATCH_EXPRESSIONS {
        expr.add_expression(text).unwrap();
    }

    let mut tick: Real = 0.0;
    group.bench_function("update10_eval7", |b| {
        b.iter(|| {
            tick += 0.001;
            if tick > 100.0 {
                tick = 0.0;
            }
            for (p, &idx) in param_indices.iter().enumerate() {
                expr.set_param(idx, (p + 1) as Real * 1.5 + black_box(tick))
                    .unwrap();
            }
            expr.eval(&ctx).unwrap();
            for i in 0..BATCH_EXPRESSIONS.len() {
                black_box(expr.get_result(i).unwrap());
            }
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_expression_eval,
    bench_native_baseline,
    bench_batch_eval
);
criterion_main!(benches);
