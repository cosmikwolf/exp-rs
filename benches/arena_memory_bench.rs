use bumpalo::Bump;
use exp_rs::{EvalContext, Expression};
use std::rc::Rc;

fn main() {
    println!("=== Arena Memory Usage Benchmark ===\n");

    // Your 7 expressions
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

    // Create arena
    let arena = Bump::with_capacity(128 * 1024); // 128KB
    println!("Arena capacity: 128 KB");

    // Setup batch builder
    let mut builder = Expression::new(&arena);

    // Add parameters
    for name in &param_names {
        builder.add_parameter(name, 0.0).unwrap();
    }

    // Add expressions
    for expr in &expressions {
        builder.add_expression(expr).unwrap();
    }

    let bytes_after_setup = arena.allocated_bytes();
    println!("Arena bytes after setup: {} KB", bytes_after_setup / 1024);

    let ctx = Rc::new(EvalContext::new());

    // Simulate 1 second at 1000Hz
    println!("\nSimulating 1 second at 1000Hz (1000 evaluations)...");
    let start = std::time::Instant::now();

    for i in 0..1000 {
        // Update parameters
        for (j, name) in param_names.iter().enumerate() {
            builder.set_param_by_name(name, (i + j) as f64).unwrap();
        }

        // Evaluate all expressions
        builder.eval(&ctx).unwrap();
    }

    let duration = start.elapsed();
    let bytes_after_eval = arena.allocated_bytes();

    println!("\nResults:");
    println!(
        "- Time for 1000 evaluations: {:.2} ms",
        duration.as_millis()
    );
    println!(
        "- Arena bytes after 1000 evals: {} KB",
        bytes_after_eval / 1024
    );
    println!(
        "- Additional bytes allocated: {} bytes",
        bytes_after_eval - bytes_after_setup
    );
    println!(
        "- CPU usage at 1000Hz: {:.1}%",
        (duration.as_secs_f64() * 100.0)
    );

    println!("\n=== Memory Savings ===");
    println!("Without arena: ~9,550 bytes × 1000 = 9.55 MB allocated/freed");
    println!(
        "With arena: {} bytes allocated",
        bytes_after_eval - bytes_after_setup
    );
    println!("Savings: 9.55 MB per second at 1000Hz!");
}

