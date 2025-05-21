use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use std::rc::Rc;
use std::cell::RefCell;

mod test_helpers;
use test_helpers::create_context;

#[test]
fn test_basic_logical_operations() {
    let ctx = Some(Rc::new(create_context()));

    // Test AND operator
    assert_eq!(interp("1 && 1", ctx.clone()).unwrap(), 1.0);
    assert_eq!(interp("1 && 0", ctx.clone()).unwrap(), 0.0);
    assert_eq!(interp("0 && 1", ctx.clone()).unwrap(), 0.0);
    assert_eq!(interp("0 && 0", ctx.clone()).unwrap(), 0.0);
    
    // Test OR operator
    assert_eq!(interp("1 || 1", ctx.clone()).unwrap(), 1.0);
    assert_eq!(interp("1 || 0", ctx.clone()).unwrap(), 1.0);
    assert_eq!(interp("0 || 1", ctx.clone()).unwrap(), 1.0);
    assert_eq!(interp("0 || 0", ctx.clone()).unwrap(), 0.0);
    
    // Test with actual boolean results from comparisons
    assert_eq!(interp("(5 > 3) && (2 < 4)", ctx.clone()).unwrap(), 1.0);
    assert_eq!(interp("(5 < 3) || (2 > 4)", ctx.clone()).unwrap(), 0.0);
    assert_eq!(interp("(5 > 3) || (2 > 4)", ctx.clone()).unwrap(), 1.0);
}

#[test]
fn test_short_circuit_evaluation() {
    // Use RefCell to track function evaluation
    let eval_count = Rc::new(RefCell::new(0));
    
    // Create context with functions that have side effects
    let mut ctx = EvalContext::new();
    
    // Clone Rc for use in closures
    let count1 = Rc::clone(&eval_count);
    ctx.register_native_function("increment_and_return_true", 0, move |_| {
        *count1.borrow_mut() += 1;
        1.0
    });
    
    let count2 = Rc::clone(&eval_count);
    ctx.register_native_function("increment_and_return_false", 0, move |_| {
        *count2.borrow_mut() += 1;
        0.0
    });
    
    let ctx_rc = Rc::new(ctx);
    
    // Test AND short-circuit
    *eval_count.borrow_mut() = 0;
    let result = interp("0 && increment_and_return_true()", Some(ctx_rc.clone())).unwrap();
    assert_eq!(result, 0.0);
    assert_eq!(*eval_count.borrow(), 0, "Right side of AND should not be evaluated when left is false");
    
    // Test OR short-circuit
    *eval_count.borrow_mut() = 0;
    let result = interp("1 || increment_and_return_false()", Some(ctx_rc.clone())).unwrap();
    assert_eq!(result, 1.0);
    assert_eq!(*eval_count.borrow(), 0, "Right side of OR should not be evaluated when left is true");
    
    // Verify non-short-circuit cases
    *eval_count.borrow_mut() = 0;
    let result = interp("1 && increment_and_return_true()", Some(ctx_rc.clone())).unwrap();
    assert_eq!(result, 1.0);
    assert_eq!(*eval_count.borrow(), 1, "Right side of AND should be evaluated when left is true");
    
    *eval_count.borrow_mut() = 0;
    let result = interp("0 || increment_and_return_false()", Some(ctx_rc.clone())).unwrap();
    assert_eq!(result, 0.0);
    assert_eq!(*eval_count.borrow(), 1, "Right side of OR should be evaluated when left is false");
}

#[test]
fn test_operator_precedence() {
    let ctx = Some(Rc::new(create_context()));

    // Verify AND has higher precedence than OR
    assert_eq!(interp("0 && 0 || 1", ctx.clone()).unwrap(), 1.0); // (0 && 0) || 1
    assert_eq!(interp("1 || 0 && 0", ctx.clone()).unwrap(), 1.0); // 1 || (0 && 0)
    
    // Verify comparison operators have higher precedence than logical operators
    assert_eq!(interp("5 > 3 && 2 < 4", ctx.clone()).unwrap(), 1.0); // (5 > 3) && (2 < 4)
    assert_eq!(interp("5 > 3 || 2 > 4", ctx.clone()).unwrap(), 1.0); // (5 > 3) || (2 > 4)
    
    // Test with parentheses to force evaluation order
    assert_eq!(interp("0 && (0 || 1)", ctx.clone()).unwrap(), 0.0);
    assert_eq!(interp("(1 || 0) && 0", ctx.clone()).unwrap(), 0.0);
}

#[test]
fn test_complex_logical_expressions() {
    let base_ctx = Some(Rc::new(create_context()));

    // Test nested expressions
    assert_eq!(interp("(1 && 1) && (1 && 1)", base_ctx.clone()).unwrap(), 1.0);
    assert_eq!(interp("(1 || 0) && (0 || 1)", base_ctx.clone()).unwrap(), 1.0);
    assert_eq!(interp("(0 || 0) || (0 || 0)", base_ctx.clone()).unwrap(), 0.0);
    
    // Test with variables
    let mut ctx = create_context();
    ctx.set_parameter("x", 5.0);
    ctx.set_parameter("y", -3.0);
    
    let ctx_rc = Rc::new(ctx);
    
    // Test complex expressions with variables
    assert_eq!(interp("x > 0 && y < 0", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("x < 0 || y < 0", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("(x > 0 && y > 0) || (x < 0 && y < 0)", Some(ctx_rc.clone())).unwrap(), 0.0);
    
    // Test in custom functions
    let mut ctx2 = EvalContext::new();
    ctx2.register_expression_function("is_between", &["x", "min", "max"], "x >= min && x <= max").unwrap();
    
    let ctx2_rc = Rc::new(ctx2);
    
    assert_eq!(interp("is_between(5, 1, 10)", Some(ctx2_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("is_between(0, 1, 10)", Some(ctx2_rc.clone())).unwrap(), 0.0);
    assert_eq!(interp("is_between(5, 10, 20)", Some(ctx2_rc.clone())).unwrap(), 0.0);
}

#[test]
fn test_simplified_logical_operators() {
    // Create a clean context
    let mut ctx = EvalContext::new();
    
    // Register a tracking function to count calls
    let count_calls = Rc::new(RefCell::new(0));
    
    let call_counter1 = Rc::clone(&count_calls);
    ctx.register_native_function("tracked_true", 0, move |_| {
        *call_counter1.borrow_mut() += 1;
        1.0 // Always returns true (1.0)
    });
    
    let call_counter2 = Rc::clone(&count_calls);
    ctx.register_native_function("tracked_false", 0, move |_| {
        *call_counter2.borrow_mut() += 1;
        0.0 // Always returns false (0.0)
    });
    
    let ctx_rc = Rc::new(ctx);
    
    // Test 1: Basic AND operator behavior
    assert_eq!(interp("1 && 1", None).unwrap(), 1.0, "1 && 1 should be 1");
    assert_eq!(interp("1 && 0", None).unwrap(), 0.0, "1 && 0 should be 0");
    assert_eq!(interp("0 && 1", None).unwrap(), 0.0, "0 && 1 should be 0");
    assert_eq!(interp("0 && 0", None).unwrap(), 0.0, "0 && 0 should be 0");
    
    // Test 2: Basic OR operator behavior
    assert_eq!(interp("1 || 1", None).unwrap(), 1.0, "1 || 1 should be 1");
    assert_eq!(interp("1 || 0", None).unwrap(), 1.0, "1 || 0 should be 1");
    assert_eq!(interp("0 || 1", None).unwrap(), 1.0, "0 || 1 should be 1");
    assert_eq!(interp("0 || 0", None).unwrap(), 0.0, "0 || 0 should be 0");
    
    // Test 3: AND short-circuit behavior
    *count_calls.borrow_mut() = 0;
    let result = interp("0 && tracked_true()", Some(ctx_rc.clone())).unwrap();
    assert_eq!(result, 0.0, "0 && tracked_true() should be 0");
    assert_eq!(*count_calls.borrow(), 0, "tracked_true() should not be called due to short-circuit");
    
    *count_calls.borrow_mut() = 0;
    let result = interp("1 && tracked_true()", Some(ctx_rc.clone())).unwrap();
    assert_eq!(result, 1.0, "1 && tracked_true() should be 1");
    assert_eq!(*count_calls.borrow(), 1, "tracked_true() should be called");
    
    // Test 4: OR short-circuit behavior
    *count_calls.borrow_mut() = 0;
    let result = interp("1 || tracked_true()", Some(ctx_rc.clone())).unwrap();
    assert_eq!(result, 1.0, "1 || tracked_true() should be 1");
    assert_eq!(*count_calls.borrow(), 0, "tracked_true() should not be called due to short-circuit");
    
    *count_calls.borrow_mut() = 0;
    let result = interp("0 || tracked_true()", Some(ctx_rc.clone())).unwrap();
    assert_eq!(result, 1.0, "0 || tracked_true() should be 1");
    assert_eq!(*count_calls.borrow(), 1, "tracked_true() should be called");
    
    // Test 5: Combining AND and OR with correct precedence
    assert_eq!(interp("1 && 0 || 1", None).unwrap(), 1.0, "(1 && 0) || 1 should be 1");
    assert_eq!(interp("0 || 1 && 0", None).unwrap(), 0.0, "0 || (1 && 0) should be 0");
    assert_eq!(interp("1 && (0 || 1)", None).unwrap(), 1.0, "1 && (0 || 1) should be 1");
}

#[test]
fn test_mixed_operators_and_logical() {
    let ctx = Some(Rc::new(create_context()));

    // Test mixing arithmetic, comparison, and logical operators
    assert_eq!(interp("3 + 4 > 6 && 10 - 5 <= 5", ctx.clone()).unwrap(), 1.0);
    assert_eq!(interp("2 * 3 == 6 || 10 / 2 != 5", ctx.clone()).unwrap(), 1.0);
    assert_eq!(interp("2^3 > 8 || 3^2 < 9", ctx.clone()).unwrap(), 0.0);
    
    // Test with complex expressions
    assert_eq!(interp("(3 + 4 * 2) > (2 * 5) && (10 - 2) <= 8", ctx.clone()).unwrap(), 1.0);
    assert_eq!(interp("sqrt(4) == 2 && sin(0) == 0", ctx.clone()).unwrap(), 1.0);
    
    // Test with more complex nested expressions
    assert_eq!(interp("(2 > 1 && 3 > 2) || (4 < 3 && 5 < 4)", ctx.clone()).unwrap(), 1.0);
    assert_eq!(interp("(2 < 1 || 3 < 2) && (4 > 3 || 5 > 4)", ctx.clone()).unwrap(), 0.0);
}

#[test]
fn test_logical_operators_with_functions() {
    let mut ctx = EvalContext::new();
    
    // Register some test functions
    ctx.register_native_function("is_positive", 1, |args| if args[0] > 0.0 { 1.0 } else { 0.0 });
    ctx.register_native_function("is_negative", 1, |args| if args[0] < 0.0 { 1.0 } else { 0.0 });
    
    let ctx_rc = Rc::new(ctx);
    
    // Test using functions in logical expressions
    assert_eq!(interp("is_positive(5) && is_positive(10)", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("is_negative(-3) || is_negative(0)", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("is_positive(5) && is_negative(5)", Some(ctx_rc.clone())).unwrap(), 0.0);
    assert_eq!(interp("is_positive(-5) || is_negative(-5)", Some(ctx_rc.clone())).unwrap(), 1.0);
    
    // Test short-circuiting with functions
    assert_eq!(interp("is_negative(5) && is_positive(0)", Some(ctx_rc.clone())).unwrap(), 0.0);
    assert_eq!(interp("is_positive(5) || is_negative(0)", Some(ctx_rc.clone())).unwrap(), 1.0);
}