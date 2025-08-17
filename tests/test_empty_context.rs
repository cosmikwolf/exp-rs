use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use std::rc::Rc;

#[test]
fn test_empty_context_has_no_functions() {
    let mut ctx = EvalContext::empty();
    
    // Register basic operators for this test
    ctx.register_native_function("+", 2, |args| args[0] + args[1]).unwrap();
    
    // Basic operators work when manually registered
    let result = interp("2 + 3", Some(Rc::new(ctx.clone())));
    assert_eq!(result.unwrap(), 5.0);
    
    // In test mode without libm feature, sin is provided by std
    // So let's test a function that's never registered
    let result = interp("my_custom_func(1.0)", Some(Rc::new(ctx.clone())));
    assert!(result.is_err());
    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(err_str.contains("Unknown function: 'my_custom_func'"));
        }
        Ok(_) => panic!("Expected error for unknown function"),
    }
    
    // Constants are registered as 0-arity functions, so they require parentheses
    // Without registration, they should fail as unknown variables
    let result = interp("pi()", Some(Rc::new(ctx.clone())));
    assert!(result.is_err());
    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(err_str.contains("Unknown function: 'pi'"));
        }
        Ok(_) => panic!("Expected error for unknown function"),
    }
    
    // Test that e constant also fails
    let result = interp("e()", Some(Rc::new(ctx)));
    assert!(result.is_err());
}

#[test]
fn test_empty_context_with_manual_functions() {
    let mut ctx = EvalContext::empty();
    
    // Manually register basic operators and the functions we need
    ctx.register_native_function("+", 2, |args| args[0] + args[1]).unwrap();
    ctx.register_native_function("*", 2, |args| args[0] * args[1]).unwrap();
    ctx.register_native_function("neg", 1, |args| -args[0]).unwrap(); // For unary negation
    ctx.register_native_function("abs", 1, |args| args[0].abs()).unwrap();
    ctx.register_native_function("max", 2, |args| args[0].max(args[1])).unwrap();
    
    // Basic operators work when manually registered
    let result = interp("2 + 3", Some(Rc::new(ctx.clone())));
    assert_eq!(result.unwrap(), 5.0);
    
    let result = interp("4 * 5", Some(Rc::new(ctx.clone())));
    assert_eq!(result.unwrap(), 20.0);
    
    // Registered functions work
    let result = interp("abs(-5)", Some(Rc::new(ctx.clone())));
    assert_eq!(result.unwrap(), 5.0);
    
    let result = interp("max(3, 7)", Some(Rc::new(ctx.clone())));
    assert_eq!(result.unwrap(), 7.0);
    
    // But unregistered functions should fail
    let result = interp("my_func(1.0)", Some(Rc::new(ctx)));
    assert!(result.is_err());
}

#[test]
fn test_regular_context_has_functions() {
    let ctx = EvalContext::new();
    
    // All basic operators should work out of the box
    let result = interp("2 + 3", Some(Rc::new(ctx.clone())));
    assert_eq!(result.unwrap(), 5.0);
    
    let result = interp("10 - 5", Some(Rc::new(ctx.clone())));
    assert_eq!(result.unwrap(), 5.0);
    
    let result = interp("4 * 5", Some(Rc::new(ctx.clone())));
    assert_eq!(result.unwrap(), 20.0);
    
    let result = interp("10 / 2", Some(Rc::new(ctx)));
    assert_eq!(result.unwrap(), 5.0);
}