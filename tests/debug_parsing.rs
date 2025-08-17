use exp_rs::{context::EvalContext, interp};
use std::rc::Rc;

#[test]
fn debug_sin_parsing() {
    let mut ctx = EvalContext::new();
    ctx.register_native_function("sin", 1, |args| args[0].sin());
    let ctx_rc = Rc::new(ctx);

    println!("Testing 'sin 1':");
    match interp("sin 1", Some(ctx_rc.clone())) {
        Ok(result) => println!("  Result: {}", result),
        Err(e) => println!("  Error: {}", e),
    }

    println!("Testing 'sin 1 + 2':");
    match interp("sin 1 + 2", Some(ctx_rc.clone())) {
        Ok(result) => println!("  Result: {}", result),
        Err(e) => println!("  Error: {}", e),
    }

    println!("Testing 'sin(1) + 2':");
    match interp("sin(1) + 2", Some(ctx_rc.clone())) {
        Ok(result) => println!("  Result: {}", result),
        Err(e) => println!("  Error: {}", e),
    }
}
