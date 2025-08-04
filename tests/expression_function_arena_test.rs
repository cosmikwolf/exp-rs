//! Test expression functions with arena allocation

use exp_rs::{Real, EvalContext};
use exp_rs::batch_builder::ArenaBatchBuilder;
use bumpalo::Bump;
use std::rc::Rc;

#[test]
#[ignore = "Expression functions require arena allocation - not supported in current architecture"]
fn test_expression_function_with_arena() {
    // Create arena
    let arena = Bump::new();
    
    // Create batch builder with arena
    let mut builder = ArenaBatchBuilder::new(&arena);
    
    // Create context and register an expression function
    let mut ctx = EvalContext::new();
    ctx.register_expression_function("double", &["x"], "x * 2").unwrap();
    ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4").unwrap();
    
    // Add expressions that use the functions
    assert_eq!(builder.add_expression("double(5)").unwrap(), 0);
    assert_eq!(builder.add_expression("polynomial(2)").unwrap(), 1);
    assert_eq!(builder.add_expression("double(x) + polynomial(y)").unwrap(), 2);
    
    // Add parameters for the third expression
    assert_eq!(builder.add_parameter("x", 3.0).unwrap(), 0);
    assert_eq!(builder.add_parameter("y", 1.0).unwrap(), 1);
    
    // Evaluate
    let ctx_rc = Rc::new(ctx);
    builder.eval(&ctx_rc).unwrap();
    
    // Check results
    assert_eq!(builder.get_result(0).unwrap(), 10.0); // double(5) = 10
    assert_eq!(builder.get_result(1).unwrap(), 26.0); // 2^3 + 2*2^2 + 3*2 + 4 = 8 + 8 + 6 + 4 = 26
    assert_eq!(builder.get_result(2).unwrap(), 16.0); // double(3) + polynomial(1) = 6 + 10 = 16
}

#[test]
#[ignore = "Expression functions require arena allocation - not supported in current architecture"]
fn test_nested_expression_functions() {
    // Create arena
    let arena = Bump::new();
    
    // Create batch builder
    let mut builder = ArenaBatchBuilder::new(&arena);
    
    // Create context with nested functions
    let mut ctx = EvalContext::new();
    ctx.register_expression_function("add_one", &["x"], "x + 1").unwrap();
    ctx.register_expression_function("double", &["x"], "x * 2").unwrap();
    ctx.register_expression_function("compose", &["x"], "double(add_one(x))").unwrap();
    
    // Test nested function call
    assert_eq!(builder.add_expression("compose(5)").unwrap(), 0);
    
    // Evaluate
    let ctx_rc = Rc::new(ctx);
    builder.eval(&ctx_rc).unwrap();
    
    // compose(5) = double(add_one(5)) = double(6) = 12
    assert_eq!(builder.get_result(0).unwrap(), 12.0);
}

#[test]
#[ignore = "Expression functions require arena allocation - not supported in current architecture"]
fn test_expression_function_zero_allocations() {
    // Create arena
    let arena = Bump::with_capacity(16384);
    
    // Create batch builder
    let mut builder = ArenaBatchBuilder::new(&arena);
    
    // Create context
    let mut ctx = EvalContext::new();
    ctx.register_expression_function("compute", &["x", "y"], "x^2 + y^2 + sin(x*y)").unwrap();
    
    // Add expression
    builder.add_expression("compute(a, b)").unwrap();
    builder.add_parameter("a", 0.0).unwrap();
    builder.add_parameter("b", 0.0).unwrap();
    
    let ctx_rc = Rc::new(ctx);
    
    // Record allocations before evaluation loop
    let allocated_before = arena.allocated_bytes();
    
    // Evaluate many times - should not allocate after first parse
    for i in 0..1000 {
        builder.set_param(0, i as Real * 0.1).unwrap();
        builder.set_param(1, i as Real * 0.05).unwrap();
        builder.eval(&ctx_rc).unwrap();
    }
    
    // Check that no additional allocations occurred
    let allocated_after = arena.allocated_bytes();
    
    // The expression function should be parsed only once and cached
    // No allocations should occur during the 1000 evaluations
    assert_eq!(allocated_before, allocated_after, 
        "Expression functions should not allocate during evaluation");
}

#[test]
#[ignore = "Expression functions require arena allocation - not supported in current architecture"]
fn test_expression_function_with_all_param_types() {
    let arena = Bump::new();
    let mut builder = ArenaBatchBuilder::new(&arena);
    
    let mut ctx = EvalContext::new();
    
    // Register functions with various parameter counts
    ctx.register_expression_function("zero_params", &[], "42").unwrap();
    ctx.register_expression_function("one_param", &["x"], "x * x").unwrap();
    ctx.register_expression_function("three_params", &["a", "b", "c"], "a + b * c").unwrap();
    
    // Test all functions
    builder.add_expression("zero_params()").unwrap();
    builder.add_expression("one_param(7)").unwrap();
    builder.add_expression("three_params(1, 2, 3)").unwrap();
    
    let ctx_rc = Rc::new(ctx);
    builder.eval(&ctx_rc).unwrap();
    
    assert_eq!(builder.get_result(0).unwrap(), 42.0);
    assert_eq!(builder.get_result(1).unwrap(), 49.0);
    assert_eq!(builder.get_result(2).unwrap(), 7.0); // 1 + 2*3
}

// ============================================================================
// VALIDATION TESTS
// These tests demonstrate what validation happens at registration vs evaluation
// ============================================================================

#[test]
fn test_syntax_validation_at_registration() {
    let mut ctx = EvalContext::new();
    
    // These should fail at registration time due to syntax errors
    
    // Unmatched parenthesis
    let result = ctx.register_expression_function("bad_paren", &["x"], "x + (2 * 3");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), exp_rs::error::ExprError::UnmatchedParenthesis { .. }));
    
    // Invalid operator usage
    let result = ctx.register_expression_function("bad_op", &["x"], "x + * 2");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), exp_rs::error::ExprError::Syntax(_)));
    
    // Empty expression
    let result = ctx.register_expression_function("empty", &["x"], "");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), exp_rs::error::ExprError::Syntax(_)));
    
    // Missing operand
    let result = ctx.register_expression_function("missing_operand", &["x"], "x +");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), exp_rs::error::ExprError::Syntax(_)));
}

#[test]
#[ignore = "Expression functions require arena allocation - not supported in current architecture"]
fn test_semantic_validation_deferred_to_evaluation() {
    // These expressions parse successfully but fail during evaluation
    let arena = Bump::new();
    let mut builder = ArenaBatchBuilder::new(&arena);
    let mut ctx = EvalContext::new();
    
    // 1. Undefined function reference - registers OK
    ctx.register_expression_function("uses_undefined", &["x"], "undefined_func(x)").unwrap();
    
    // 2. Wrong function arity - registers OK
    ctx.register_native_function("sin", 1, |args| args[0].sin()).unwrap();
    ctx.register_expression_function("bad_sin", &["x"], "sin(x, 2, 3)").unwrap();
    
    // 3. Undefined variable reference - registers OK
    ctx.register_expression_function("uses_undefined_var", &["x"], "x + undefined_var").unwrap();
    
    // 4. Function used as variable - registers OK
    ctx.register_expression_function("func_as_var", &["x"], "sin + x").unwrap();
    
    // Now test that they fail during evaluation
    let ctx_rc = Rc::new(ctx);
    
    // Test 1: Undefined function
    builder.add_expression("uses_undefined(5)").unwrap();
    let result = builder.eval(&ctx_rc);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), exp_rs::error::ExprError::UnknownFunction { .. }));
    
    // Reset builder for next test
    let mut builder = ArenaBatchBuilder::new(&arena);
    
    // Test 2: Wrong arity
    builder.add_expression("bad_sin(1.5)").unwrap();
    let result = builder.eval(&ctx_rc);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), exp_rs::error::ExprError::InvalidFunctionCall { .. }));
    
    // Reset for test 3
    let mut builder = ArenaBatchBuilder::new(&arena);
    
    // Test 3: Undefined variable
    builder.add_expression("uses_undefined_var(10)").unwrap();
    let result = builder.eval(&ctx_rc);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), exp_rs::error::ExprError::UnknownVariable { .. }));
    
    // Reset for test 4
    let mut builder = ArenaBatchBuilder::new(&arena);
    
    // Test 4: Function as variable
    builder.add_expression("func_as_var(5)").unwrap();
    let result = builder.eval(&ctx_rc);
    assert!(result.is_err());
    // This specific error depends on evaluation order
}

#[test]
fn test_parameter_validation() {
    let mut ctx = EvalContext::new();
    
    // Test parameter name length limit (32 characters)
    let long_param = "a".repeat(33);
    let result = ctx.register_expression_function("long_param", &[&long_param], "x");
    // Currently this might succeed because parameter names are stored as String, not HString
    // This test documents current behavior
    
    // Test many parameters (no explicit limit in current implementation)
    let many_params: Vec<&str> = (0..20).map(|i| Box::leak(format!("p{}", i).into_boxed_str()) as &str).collect();
    let result = ctx.register_expression_function("many_params", &many_params, "p0");
    assert!(result.is_ok()); // Should succeed - no hard limit on parameter count
    
    // Test duplicate parameter names - currently NOT validated
    let result = ctx.register_expression_function("dup_params", &["x", "x"], "x + x");
    assert!(result.is_ok()); // Currently succeeds - no duplicate check
    
    // Test empty parameter name
    let result = ctx.register_expression_function("empty_param", &[""], "42");
    assert!(result.is_ok()); // Currently succeeds - empty names allowed
}

#[test]
fn test_function_name_validation() {
    let mut ctx = EvalContext::new();
    
    // Test function name length limit (32 characters)
    let long_name = "f".repeat(33);
    let result = ctx.register_expression_function(&long_name, &["x"], "x");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), exp_rs::error::ExprError::StringTooLong));
    
    // Test maximum valid length (32 characters)
    let max_name = "f".repeat(32);
    let result = ctx.register_expression_function(&max_name, &["x"], "x");
    assert!(result.is_ok());
    
    // Test empty function name
    let result = ctx.register_expression_function("", &["x"], "x");
    assert!(result.is_ok()); // Currently succeeds - empty names allowed
}

#[test]
fn test_expression_function_capacity() {
    let mut ctx = EvalContext::new();
    
    // Register functions up to the limit (8 expression functions)
    for i in 0..8 {
        let name = format!("func{}", i);
        let result = ctx.register_expression_function(&name, &["x"], "x + 1");
        assert!(result.is_ok(), "Failed to register function {}", i);
    }
    
    // The 9th function should fail with capacity exceeded
    let result = ctx.register_expression_function("func8", &["x"], "x + 1");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), exp_rs::error::ExprError::CapacityExceeded(_)));
}

#[test]
#[ignore = "Expression functions require arena allocation - not supported in current architecture"]
fn test_recursive_expression_functions() {
    let mut ctx = EvalContext::new();
    
    // Direct recursion - registers OK but would fail at runtime
    ctx.register_expression_function("factorial", &["n"], "n * factorial(n-1)").unwrap();
    
    // Mutual recursion - both register OK
    ctx.register_expression_function("even", &["n"], "n == 0 || odd(n-1)").unwrap();
    ctx.register_expression_function("odd", &["n"], "n != 0 && even(n-1)").unwrap();
    
    // These would need proper base cases and recursion depth limiting to work
}

#[test]
#[ignore = "Expression functions require arena allocation - not supported in current architecture"]
fn test_expression_with_context_variables() {
    let arena = Bump::new();
    let mut builder = ArenaBatchBuilder::new(&arena);
    let mut ctx = EvalContext::new();
    
    // Set a global variable
    ctx.set_parameter("global_scale", 10.0);
    
    // Expression function that references the global - registers OK
    ctx.register_expression_function("scale", &["x"], "x * global_scale").unwrap();
    
    builder.add_expression("scale(5)").unwrap();
    
    let ctx_rc = Rc::new(ctx);
    builder.eval(&ctx_rc).unwrap();
    
    // Should work - global_scale is available in context
    assert_eq!(builder.get_result(0).unwrap(), 50.0);
}

#[test]
#[ignore = "Expression functions require arena allocation - not supported in current architecture"]
fn test_unused_parameters_in_expression() {
    let arena = Bump::new();
    let mut builder = ArenaBatchBuilder::new(&arena);
    let mut ctx = EvalContext::new();
    
    // Register function with unused parameter - currently no warning
    ctx.register_expression_function("ignores_y", &["x", "y"], "x * 2").unwrap();
    
    builder.add_expression("ignores_y(5, 100)").unwrap();
    
    let ctx_rc = Rc::new(ctx);
    builder.eval(&ctx_rc).unwrap();
    
    // Works fine - y is simply ignored
    assert_eq!(builder.get_result(0).unwrap(), 10.0);
}

#[test]
fn test_very_long_expression() {
    let mut ctx = EvalContext::new();
    
    // Build a very long but valid expression
    let mut expr = "x".to_string();
    for _ in 0..100 {
        expr.push_str(" + 1");
    }
    
    // Should register successfully - no explicit length limit on expressions
    let result = ctx.register_expression_function("long_expr", &["x"], &expr);
    assert!(result.is_ok());
    
    // Test extremely long expression that might hit memory limits
    let huge_expr = "x + ".repeat(10000) + "x";
    let result = ctx.register_expression_function("huge_expr", &["x"], &huge_expr);
    // This might fail due to stack overflow during parsing or memory limits
    // The actual limit depends on the parser implementation
}

// ============================================================================
// VALIDATED REGISTRATION TESTS
// Tests for the new register_expression_function_validated method
// ============================================================================

#[test]
fn test_validated_registration_syntax_errors() {
    let mut ctx = EvalContext::new();
    
    // Test syntax error detection
    let report = ctx.register_expression_function_validated(
        "bad_syntax",
        &["x"],
        "x + (2 * 3",
        true
    ).unwrap();
    
    assert!(!report.syntax_valid);
    assert!(report.syntax_error.is_some());
    assert!(!report.semantic_validated); // Semantic validation not performed on syntax errors
}

#[test]
fn test_validated_registration_undefined_functions() {
    let mut ctx = EvalContext::new();
    
    // Register with undefined function
    let report = ctx.register_expression_function_validated(
        "uses_undefined",
        &["x"],
        "undefined_func(x) + another_undefined(x, 2)",
        true
    ).unwrap();
    
    assert!(report.syntax_valid);
    assert!(report.semantic_validated);
    assert_eq!(report.undefined_functions.len(), 2);
    assert!(report.undefined_functions.contains(&"undefined_func".to_string()));
    assert!(report.undefined_functions.contains(&"another_undefined".to_string()));
    assert!(!report.is_valid());
}

#[test]
fn test_validated_registration_arity_warnings() {
    let mut ctx = EvalContext::new();
    
    // Register native function for testing
    ctx.register_native_function("sin", 1, |args| args[0].sin()).unwrap();
    ctx.register_native_function("pow", 2, |args| args[0].powf(args[1])).unwrap();
    
    // Register with wrong arity
    let report = ctx.register_expression_function_validated(
        "bad_arity",
        &["x"],
        "sin(x, 2, 3) + pow(x)",
        true
    ).unwrap();
    
    assert!(report.syntax_valid);
    assert_eq!(report.arity_warnings.len(), 2);
    
    // Check sin warning
    let sin_warning = report.arity_warnings.iter()
        .find(|(name, _, _)| name == "sin")
        .unwrap();
    assert_eq!(sin_warning.1, 3); // Used with 3 args
    assert_eq!(sin_warning.2, Some(1)); // Expected 1 arg
    
    // Check pow warning
    let pow_warning = report.arity_warnings.iter()
        .find(|(name, _, _)| name == "pow")
        .unwrap();
    assert_eq!(pow_warning.1, 1); // Used with 1 arg
    assert_eq!(pow_warning.2, Some(2)); // Expected 2 args
}

#[test]
fn test_validated_registration_undefined_variables() {
    let mut ctx = EvalContext::new();
    
    // Set a global variable
    ctx.set_parameter("global_var", 10.0);
    
    // Register with undefined variables
    let report = ctx.register_expression_function_validated(
        "uses_undefined_vars",
        &["x", "y"],
        "x + y + undefined_var + another_undefined - global_var",
        true
    ).unwrap();
    
    assert!(report.syntax_valid);
    assert_eq!(report.undefined_variables.len(), 2);
    assert!(report.undefined_variables.contains(&"undefined_var".to_string()));
    assert!(report.undefined_variables.contains(&"another_undefined".to_string()));
    // global_var should NOT be in undefined list since it exists
    assert!(!report.undefined_variables.contains(&"global_var".to_string()));
}

#[test]
fn test_validated_registration_unused_parameters() {
    let mut ctx = EvalContext::new();
    
    // Register with unused parameters
    let report = ctx.register_expression_function_validated(
        "ignores_params",
        &["x", "y", "z"],
        "x + 2", // Only uses x
        true
    ).unwrap();
    
    assert!(report.syntax_valid);
    assert!(report.has_only_warnings());
    assert_eq!(report.unused_parameters.len(), 2);
    assert!(report.unused_parameters.contains(&"y".to_string()));
    assert!(report.unused_parameters.contains(&"z".to_string()));
    assert!(!report.unused_parameters.contains(&"x".to_string()));
}

#[test]
fn test_validated_registration_all_valid() {
    let mut ctx = EvalContext::new();
    
    // Register supporting functions
    ctx.register_native_function("sin", 1, |args| args[0].sin()).unwrap();
    ctx.set_parameter("scale", 2.0);
    
    // Register a fully valid function
    let report = ctx.register_expression_function_validated(
        "compute",
        &["x", "y"],
        "sin(x) * y + scale",
        true
    ).unwrap();
    
    assert!(report.syntax_valid);
    assert!(report.semantic_validated);
    assert!(report.is_valid());
    assert!(report.undefined_functions.is_empty());
    assert!(report.arity_warnings.is_empty());
    assert!(report.undefined_variables.is_empty());
    assert!(report.unused_parameters.is_empty());
}

#[test]
fn test_validated_registration_without_semantic_validation() {
    let mut ctx = EvalContext::new();
    
    // Register without semantic validation
    let report = ctx.register_expression_function_validated(
        "no_semantic",
        &["x"],
        "undefined_func(x)",
        false // Don't validate semantics
    ).unwrap();
    
    assert!(report.syntax_valid);
    assert!(!report.semantic_validated);
    // No semantic errors should be detected
    assert!(report.undefined_functions.is_empty());
    assert!(report.is_valid());
}

#[test]
#[cfg(feature = "libm")]
fn test_validated_registration_builtin_functions() {
    let mut ctx = EvalContext::new();
    
    // Test that built-in functions are recognized
    let report = ctx.register_expression_function_validated(
        "uses_builtins",
        &["x"],
        "sin(x) + cos(x) + sqrt(x) + pow(x, 2) + pi + e",
        true
    ).unwrap();
    
    assert!(report.syntax_valid);
    assert!(report.is_valid());
    // Built-in functions should not be in undefined list
    assert!(report.undefined_functions.is_empty());
    assert!(report.arity_warnings.is_empty());
}

#[test]
fn test_validated_registration_complex_expression() {
    let mut ctx = EvalContext::new();
    
    // Register some functions
    ctx.register_expression_function("helper", &["a"], "a * 2").unwrap();
    ctx.register_native_function("min", 2, |args| args[0].min(args[1])).unwrap();
    
    // Complex expression with multiple issues
    let report = ctx.register_expression_function_validated(
        "complex",
        &["x", "y", "z", "unused"],
        "helper(x) + min(y) + undefined(z) + global_missing",
        true
    ).unwrap();
    
    assert!(report.syntax_valid);
    assert!(!report.is_valid());
    
    // Check each type of issue
    assert_eq!(report.undefined_functions.len(), 1); // undefined
    assert_eq!(report.arity_warnings.len(), 1); // min(y) should have 2 args
    assert_eq!(report.undefined_variables.len(), 1); // global_missing
    assert_eq!(report.unused_parameters.len(), 1); // unused
}