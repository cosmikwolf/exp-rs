extern crate exp_rs;

use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use std::rc::Rc;
use std::env;

fn main() {
    // Create a test context
    let mut ctx = EvalContext::new();
    
    // Add a simple function to log values
    ctx.register_native_function("log", 1, |args| {
        println!("log() called with: {}", args[0]);
        args[0] // Return the input
    });
    
    // Add factorial implementations
    ctx.register_expression_function(
        "factorial",
        &["n"],
        "n <= 1 ? 1 : n * factorial(n-1)",
    ).unwrap();
    
    // Test with comma/semicolon operators
    let result1 = interp("log(5), 10", Some(Rc::new(ctx.clone())));
    println!("'log(5), 10' evaluates to: {:?}", result1);
    
    let result2 = interp("log(5); 10", Some(Rc::new(ctx.clone())));
    println!("'log(5); 10' evaluates to: {:?}", result2);
    
    // Test with logging + factorial 
    let result3 = interp("log(4); factorial(4)", Some(Rc::new(ctx.clone())));
    println!("'log(4); factorial(4)' evaluates to: {:?}", result3);
    
    // Get factorial directly to check its result
    let result4 = interp("factorial(4)", Some(Rc::new(ctx.clone())));
    println!("'factorial(4)' evaluates to: {:?}", result4);
}