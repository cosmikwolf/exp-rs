//! Expression evaluation module for exp-rs
//!
//! This module contains the core evaluation logic for expressions,
//! including AST traversal, variable resolution, function application,
//! and recursion depth tracking.

pub mod ast;
pub mod custom_function;
pub mod evaluator;
pub mod recursion;
pub mod types;
pub mod stack_ops;
pub mod context_stack;
pub mod iterative;

// Re-export the main evaluation functions for backward compatibility
pub use ast::*;
pub use custom_function::*;
pub use evaluator::*;
pub use recursion::*;
pub use types::*;

// Re-export recursion tracking functions
pub use recursion::{
    check_and_increment_recursion_depth, decrement_recursion_depth, get_recursion_depth,
    reset_recursion_depth, set_max_recursion_depth,
};

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeMap;
    use crate::types::{TryIntoHeaplessString, TryIntoFunctionName};

    use super::*;
    use crate::AstExpr;
    use crate::Real;
    use crate::context::EvalContext;
    use crate::context::FunctionRegistry;
    use crate::engine::{interp, parse_expression};
    use crate::error::ExprError;
    use std::sync::atomic::{AtomicUsize, Ordering};
    // Use std HashMap for tests
    use crate::abs;
    use crate::cos;
    use crate::max;
    use crate::min;
    use crate::neg;
    use crate::pow;
    use crate::sin;
    // Removed - using heapless instead
    use std::rc::Rc;

    // Helper functions for tests that need to call eval functions directly
    fn test_eval_variable(name: &str, ctx: Option<Rc<EvalContext>>) -> Result<Real, ExprError> {
        let mut var_cache = BTreeMap::new();
        super::eval_variable(name, ctx, &mut var_cache)
    }

    fn test_eval_function(
        name: &str,
        args: &[AstExpr],
        ctx: Option<Rc<EvalContext>>,
        func_cache: &mut BTreeMap<String, Option<FunctionCacheEntry>>,
    ) -> Result<Real, ExprError> {
        let mut var_cache = BTreeMap::new();
        super::eval_function(name, args, ctx, func_cache, &mut var_cache)
    }

    fn test_eval_array(
        name: &str,
        index: &AstExpr,
        ctx: Option<Rc<EvalContext>>,
        func_cache: &mut BTreeMap<String, Option<FunctionCacheEntry>>,
    ) -> Result<Real, ExprError> {
        let mut var_cache = BTreeMap::new();
        super::eval_array(name, index, ctx, func_cache, &mut var_cache)
    }

    fn test_eval_custom_function<F>(
        name: &str,
        args: &[AstExpr],
        ctx: Option<Rc<EvalContext>>,
        func_cache: &mut BTreeMap<String, Option<FunctionCacheEntry>>,
        func: &F,
    ) -> Result<Real, ExprError>
    where
        F: super::CustomFunction,
    {
        let mut var_cache = BTreeMap::new();
        super::eval_custom_function(name, args, ctx, func_cache, &mut var_cache, func)
    }

    fn test_eval_native_function(
        name: &str,
        args: &[AstExpr],
        ctx: Option<Rc<EvalContext>>,
        func_cache: &mut BTreeMap<String, Option<FunctionCacheEntry>>,
        native_fn: &OwnedNativeFunction,
    ) -> Result<Real, ExprError> {
        let mut var_cache = BTreeMap::new();
        eval_native_function(name, args, ctx, func_cache, &mut var_cache, native_fn)
    }

    #[test]
    fn test_eval_user_function_polynomial() {
        let mut ctx = create_test_context();
        ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4")
            .unwrap();
        let mut func_cache = std::collections::BTreeMap::new();
        let _ast = AstExpr::Function {
            name: "polynomial".to_string(),
            args: vec![AstExpr::Constant(3.0)],
        };
        // Avoid simultaneous mutable and immutable borrow of ctx
        let expr_fn = ctx.get_expression_function("polynomial").unwrap().clone();
        let val = test_eval_custom_function(
            "polynomial",
            &[AstExpr::Constant(3.0)],
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
            &expr_fn,
        )
        .unwrap();
        assert_eq!(val, 58.0); // 3^3 + 2*3^2 + 3*3 + 4 = 27 + 18 + 9 + 4 = 58
    }

    #[test]
    fn test_eval_expression_function_simple() {
        let mut ctx = EvalContext::new();
        ctx.register_expression_function("double", &["x"], "x*2")
            .unwrap();
        let mut func_cache = std::collections::BTreeMap::new();
        // Avoid simultaneous mutable and immutable borrow of ctx
        let expr_fn = ctx.get_expression_function("double").unwrap().clone();
        let val = test_eval_custom_function(
            "double",
            &[AstExpr::Constant(7.0)],
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
            &expr_fn,
        )
        .unwrap();
        assert_eq!(val, 14.0);
    }

    #[test]
    fn test_eval_native_function_simple() {
        let mut ctx = EvalContext::new();
        ctx.register_native_function("triple", 1, |args| args[0] * 3.0);
        let mut func_cache = std::collections::BTreeMap::new();
        // Avoid simultaneous mutable and immutable borrow of ctx by splitting the scope
        let native_fn = {
            // We need to use the string directly as the key
            let nf = ctx
                .function_registry
                .native_functions
                .get(&"triple".try_into_function_name().unwrap())
                .unwrap();
            OwnedNativeFunction {
                arity: nf.arity,
                implementation: nf.implementation.clone(),
                name: nf.name.to_string(), // Convert to String
                description: nf.description.clone(),
            }
        };
        // At this point, the immutable borrow is dropped

        let val = test_eval_native_function(
            "triple",
            &[AstExpr::Constant(4.0)],
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
            &native_fn,
        )
        .unwrap();
        assert_eq!(val, 12.0);
    }

    // Helper to create a context and register defaults IF builtins are enabled
    fn create_test_context<'a>() -> EvalContext {
        let mut ctx = EvalContext::new();
        
        // In tests, we can use stdlib functions even when libm is disabled
        #[cfg(all(test, not(feature = "libm")))]
        {
            // Register basic math functions using stdlib
            let _ = ctx.register_native_function("sin", 1, |args| args[0].sin());
            let _ = ctx.register_native_function("cos", 1, |args| args[0].cos());
            let _ = ctx.register_native_function("tan", 1, |args| args[0].tan());
            let _ = ctx.register_native_function("asin", 1, |args| args[0].asin());
            let _ = ctx.register_native_function("acos", 1, |args| args[0].acos());
            let _ = ctx.register_native_function("atan", 1, |args| args[0].atan());
            let _ = ctx.register_native_function("atan2", 2, |args| args[0].atan2(args[1]));
            let _ = ctx.register_native_function("sinh", 1, |args| args[0].sinh());
            let _ = ctx.register_native_function("cosh", 1, |args| args[0].cosh());
            let _ = ctx.register_native_function("tanh", 1, |args| args[0].tanh());
            let _ = ctx.register_native_function("exp", 1, |args| args[0].exp());
            let _ = ctx.register_native_function("ln", 1, |args| args[0].ln());
            let _ = ctx.register_native_function("log", 1, |args| args[0].ln());
            let _ = ctx.register_native_function("log10", 1, |args| args[0].log10());
            let _ = ctx.register_native_function("log2", 1, |args| args[0].log2());
            let _ = ctx.register_native_function("sqrt", 1, |args| args[0].sqrt());
            let _ = ctx.register_native_function("abs", 1, |args| args[0].abs());
            let _ = ctx.register_native_function("floor", 1, |args| args[0].floor());
            let _ = ctx.register_native_function("ceil", 1, |args| args[0].ceil());
            let _ = ctx.register_native_function("round", 1, |args| args[0].round());
            let _ = ctx.register_native_function("pow", 2, |args| args[0].powf(args[1]));
            let _ = ctx.register_native_function("^", 2, |args| args[0].powf(args[1]));
            let _ = ctx.register_native_function("min", 2, |args| args[0].min(args[1]));
            let _ = ctx.register_native_function("max", 2, |args| args[0].max(args[1]));
            let _ = ctx.register_native_function("neg", 1, |args| -args[0]);
            let _ = ctx.register_native_function("sign", 1, |args| {
                if args[0] > 0.0 { 1.0 } else if args[0] < 0.0 { -1.0 } else { 0.0 }
            });
        }
        
        // Register defaults only if the feature allows it
        #[cfg(feature = "libm")]
        {
            // Manually register built-ins needed for tests if register_defaults doesn't exist
            // or isn't comprehensive enough for test setup.
            ctx.register_native_function("sin", 1, |args| sin(args[0], 0.0));
            ctx.register_native_function("cos", 1, |args| cos(args[0], 0.0));
            ctx.register_native_function("pow", 2, |args| pow(args[0], args[1]));
            ctx.register_native_function("^", 2, |args| pow(args[0], args[1]));
            ctx.register_native_function("min", 2, |args| min(args[0], args[1]));
            ctx.register_native_function("max", 2, |args| max(args[0], args[1]));
            ctx.register_native_function("neg", 1, |args| neg(args[0], 0.0));
            ctx.register_native_function("abs", 1, |args| abs(args[0], 0.0));
            // Add others as needed by tests...
        }
        ctx
    }

    #[test]
    fn test_eval_variable_builtin_constants() {
        // Test pi and e
        #[cfg(feature = "f32")]
        {
            assert!((test_eval_variable("pi", None).unwrap() - std::f32::consts::PI).abs() < 1e-5);
            assert!((test_eval_variable("e", None).unwrap() - std::f32::consts::E).abs() < 1e-5);
        }
        #[cfg(not(feature = "f32"))]
        {
            assert!((test_eval_variable("pi", None).unwrap() - std::f64::consts::PI).abs() < 1e-10);
            assert!((test_eval_variable("e", None).unwrap() - std::f64::consts::E).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_variable_context_lookup() {
        let mut ctx = EvalContext::new();
        ctx.set_parameter("x", 42.0);
        ctx.constants.insert("y".try_into_heapless().unwrap(), crate::constants::PI).expect("Failed to insert constant");
        assert_eq!(
            test_eval_variable("x", Some(Rc::new(ctx.clone()))).unwrap(),
            42.0
        );
        assert_eq!(
            test_eval_variable("y", Some(Rc::new(ctx.clone()))).unwrap(),
            crate::constants::PI
        );
    }

    #[test]
    fn test_eval_variable_unknown_and_function_name() {
        let err = test_eval_variable("nosuchvar", None).unwrap_err();
        assert!(matches!(err, ExprError::UnknownVariable { .. }));
        let err2 = test_eval_variable("sin", None).unwrap_err();
        assert!(matches!(err2, ExprError::Syntax(_)));
    }

    #[test]
    fn test_eval_function_native_and_expression() {
        let mut ctx = create_test_context();
        // Native function
        // Don't use the ast variable if we're not going to use it
        let mut func_cache = std::collections::BTreeMap::new();
        let val = test_eval_function(
            "sin",
            &[AstExpr::Constant(0.0)],
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
        )
        .unwrap();
        assert!((val - 0.0).abs() < 1e-10);

        // Expression function
        ctx.register_expression_function("double", &["x"], "x*2")
            .unwrap();
        // No need for ast2 since we're not using it
        let val2 = test_eval_function(
            "double",
            &[AstExpr::Constant(5.0)],
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
        )
        .unwrap();
        assert_eq!(val2, 10.0);
    }

    #[test]
    fn test_eval_function_user_function() {
        let mut ctx = create_test_context();
        ctx.register_expression_function("inc", &["x"], "x+1")
            .unwrap();
        let mut func_cache = std::collections::BTreeMap::new();
        let val = test_eval_function(
            "inc",
            &[AstExpr::Constant(41.0)],
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
        )
        .unwrap();
        assert_eq!(val, 42.0);
    }

    #[test]
    fn test_eval_function_builtin_fallback() {
        let ctx = create_test_context();
        let mut func_cache = std::collections::BTreeMap::new();
        // Built-in fallback: pow(2,3)
        let val = test_eval_function(
            "pow",
            &[AstExpr::Constant(2.0), AstExpr::Constant(3.0)],
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
        )
        .unwrap();
        assert_eq!(val, 8.0);
        // Built-in fallback: abs(-5)
        let val2 = test_eval_function(
            "abs",
            &[AstExpr::Constant(-5.0)],
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
        )
        .unwrap();
        assert_eq!(val2, 5.0);
    }

    #[test]
    fn test_eval_array_success_and_out_of_bounds() {
        let mut ctx = EvalContext::new();
        ctx.arrays.insert("arr".try_into_heapless().unwrap(), vec![1.0, 2.0, 3.0]).expect("Failed to insert array");

        // Create separate caches for each call to avoid borrowing issues
        let mut func_cache1 = std::collections::BTreeMap::new();
        let mut func_cache2 = std::collections::BTreeMap::new();

        let idx_expr = AstExpr::Constant(1.0);
        let val = test_eval_array(
            "arr",
            &idx_expr,
            Some(Rc::new(ctx.clone())),
            &mut func_cache1,
        )
        .unwrap();
        assert_eq!(val, 2.0);

        // Out of bounds
        let idx_expr2 = AstExpr::Constant(10.0);
        let err = test_eval_array(
            "arr",
            &idx_expr2,
            Some(Rc::new(ctx.clone())),
            &mut func_cache2,
        )
        .unwrap_err();
        assert!(matches!(err, ExprError::ArrayIndexOutOfBounds { .. }));
    }

    #[test]
    fn test_eval_array_unknown() {
        let ctx = EvalContext::new();
        let mut func_cache = std::collections::BTreeMap::new();
        let idx_expr = AstExpr::Constant(0.0);
        let err = test_eval_array(
            "nosucharr",
            &idx_expr,
            Some(Rc::new(ctx.clone())),
            &mut func_cache,
        )
        .unwrap_err();
        assert!(matches!(err, ExprError::UnknownVariable { .. }));
    }

    #[test]
    fn test_eval_attribute_success_and_not_found() {
        let mut ctx = EvalContext::new();
        // Use the helper method to set attributes
        ctx.set_attribute("bar", "foo", 123.0).expect("Failed to set attribute");
        let val = super::eval_attribute("bar", "foo", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val, 123.0);
        let err = super::eval_attribute("bar", "baz", Some(Rc::new(ctx.clone()))).unwrap_err();
        assert!(matches!(err, ExprError::AttributeNotFound { .. }));
    }

    #[test]
    fn test_eval_attribute_unknown_base() {
        let ctx = EvalContext::new();
        let err = super::eval_attribute("nosuch", "foo", Some(Rc::new(ctx.clone()))).unwrap_err();
        assert!(matches!(err, ExprError::AttributeNotFound { .. }));
    }

    #[test]
    fn test_neg_pow_ast() {
        // AST structure test - independent of evaluation context or features
        let ast = parse_expression("-2^2").unwrap_or_else(|e| panic!("Parse error: {}", e));
        // ... (assertions remain the same) ...
        match ast {
            AstExpr::Function { ref name, ref args } if name == "neg" => {
                assert_eq!(args.len(), 1);
                match &args[0] {
                    AstExpr::Function {
                        name: pow_name,
                        args: pow_args,
                    } if pow_name == "^" => {
                        assert_eq!(pow_args.len(), 2);
                        match (&pow_args[0], &pow_args[1]) {
                            (AstExpr::Constant(a), AstExpr::Constant(b)) => {
                                assert_eq!(*a, 2.0);
                                assert_eq!(*b, 2.0);
                            }
                            _ => panic!("Expected constants as pow args"),
                        }
                    }
                    _ => panic!("Expected pow as argument to neg"),
                }
            }
            _ => panic!("Expected neg as top-level function"),
        }
    }

    #[test]
    #[cfg(feature = "libm")] // This test relies on built-in fallback
    fn test_neg_pow_eval() {
        // Test evaluation using built-in functions (no context needed for this specific expr)
        let val = interp("-2^2", None).unwrap();
        assert_eq!(val, -4.0); // Should be -(2^2) = -4
        let val2 = interp("(-2)^2", None).unwrap();
        assert_eq!(val2, 4.0); // Should be 4
    }

    #[test]
    #[cfg(all(not(feature = "libm"), feature = "std"))] // Test behavior when builtins are disabled but std is available
    fn test_neg_pow_eval_no_builtins() {
        // Create a clean context with no auto-registered functions
        let mut ctx = EvalContext {
            variables: Default::default(),
            constants: Default::default(),
            arrays: Default::default(),
            attributes: Default::default(),
            nested_arrays: Default::default(),
            function_registry: Rc::new(FunctionRegistry::default()),
            parent: None,
            ast_cache: None,
        };

        // Manually register just what we need for this test
        ctx.register_native_function("neg", 1, |args| -args[0]);
        ctx.register_native_function("^", 2, |args| args[0].powf(args[1])); // Example using powf

        // Convert to Rc<EvalContext> for interp function
        let ctx_rc = Rc::new(ctx.clone());

        let val = interp("-2^2", Some(ctx_rc.clone())).unwrap();
        assert_eq!(val, -4.0);
        let val2 = interp("(-2)^2", Some(ctx_rc)).unwrap();
        assert_eq!(val2, 4.0);

        // Create another completely empty context for error testing
        let empty_ctx = Rc::new(EvalContext {
            variables: Default::default(),
            constants: Default::default(),
            arrays: Default::default(),
            attributes: Default::default(),
            nested_arrays: Default::default(),
            function_registry: Rc::new(FunctionRegistry::default()),
            parent: None,
            ast_cache: None,
        });

        // Test that it fails with empty context (no functions registered)
        let err = interp("-2^2", Some(empty_ctx)).unwrap_err();
        assert!(matches!(err, ExprError::UnknownFunction { .. }));
    }

    #[test]
    fn test_paren_neg_pow_ast() {
        // AST structure test - independent of evaluation context or features
        let ast = parse_expression("(-2)^2").unwrap_or_else(|e| panic!("Parse error: {}", e));
        // ... (assertions remain the same) ...
        match ast {
            AstExpr::Function { ref name, ref args } if name == "^" => {
                assert_eq!(args.len(), 2);
                match &args[0] {
                    AstExpr::Function {
                        name: neg_name,
                        args: neg_args,
                    } if neg_name == "neg" => {
                        assert_eq!(neg_args.len(), 1);
                        match &neg_args[0] {
                            AstExpr::Constant(a) => assert_eq!(*a, 2.0),
                            _ => panic!("Expected constant as neg arg"),
                        }
                    }
                    _ => panic!("Expected neg as left arg to pow"),
                }
                match &args[1] {
                    AstExpr::Constant(b) => assert_eq!(*b, 2.0),
                    _ => panic!("Expected constant as right arg to pow"),
                }
            }
            _ => panic!("Expected pow as top-level function"),
        }
    }

    #[test]
    fn test_function_application_juxtaposition_ast() {
        // AST structure test - independent of evaluation context or features
        // ... (assertions remain the same) ...
        let sin_x_ast = AstExpr::Function {
            name: "sin".to_string(),
            args: vec![AstExpr::Variable("x".to_string())],
        };

        match sin_x_ast {
            AstExpr::Function { ref name, ref args } if name == "sin" => {
                assert_eq!(args.len(), 1);
                match &args[0] {
                    AstExpr::Variable(var) => assert_eq!(var, "x"),
                    _ => panic!("Expected variable as argument"),
                }
            }
            _ => panic!("Expected function node for sin x"),
        }

        // For "abs -42", we expect abs(neg(42))
        let neg_42_ast = AstExpr::Function {
            name: "neg".to_string(),
            args: vec![AstExpr::Constant(42.0)],
        };

        let abs_neg_42_ast = AstExpr::Function {
            name: "abs".to_string(),
            args: vec![neg_42_ast],
        };

        println!("AST for 'abs -42': {:?}", abs_neg_42_ast);

        match abs_neg_42_ast {
            AstExpr::Function { ref name, ref args } if name == "abs" => {
                assert_eq!(args.len(), 1);
                match &args[0] {
                    AstExpr::Function {
                        name: n2,
                        args: args2,
                    } if n2 == "neg" => {
                        assert_eq!(args2.len(), 1);
                        match &args2[0] {
                            AstExpr::Constant(c) => assert_eq!(*c, 42.0),
                            _ => panic!("Expected constant as neg arg"),
                        }
                    }
                    _ => panic!("Expected neg as argument to abs"),
                }
            }
            _ => panic!("Expected function node for abs -42"),
        }
    }

    #[test]
    fn test_function_application_juxtaposition_eval() {
        // Test evaluation: abs(neg(42)) = 42
        // This requires 'abs' and 'neg' to be available.
        let mut ctx = create_test_context(); // Gets defaults if enabled

        // If builtins disabled, manually add abs and neg
        #[cfg(not(feature = "libm"))]
        {
            ctx.register_native_function("abs", 1, |args| args[0].abs());
            ctx.register_native_function("neg", 1, |args| -args[0]);
        }

        // Manually create AST as parser might handle juxtaposition differently
        let ast = AstExpr::Function {
            name: "abs".to_string(),
            args: vec![AstExpr::Function {
                name: "neg".to_string(),
                args: vec![AstExpr::Constant(42.0)],
            }],
        };

        let val = eval_ast(&ast, Some(Rc::new(ctx))).unwrap();
        assert_eq!(val, 42.0);
    }

    #[test]
    fn test_pow_arity_ast() {
        // AST structure test - independent of evaluation context or features
        // This test assumes the *parser* handles pow(2) -> pow(2, 2) or similar.
        // If the parser produces pow(2), the evaluator handles the default exponent.
        let ast = parse_expression("pow(2)").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast {
            AstExpr::Function { ref name, ref args } if name == "pow" => {
                // The parser might produce 1 or 2 args depending on its logic.
                // The evaluator handles the case where only 1 arg is provided by the AST.
                assert!(args.len() == 1 || args.len() == 2);
                match &args[0] {
                    AstExpr::Constant(c) => assert_eq!(*c, 2.0),
                    _ => panic!("Expected constant as pow arg"),
                }
                // If parser adds default arg:
                if args.len() == 2 {
                    match &args[1] {
                        AstExpr::Constant(c) => assert_eq!(*c, 2.0),
                        _ => panic!("Expected constant as pow second arg"),
                    }
                }
            }
            _ => panic!("Expected function node for pow(2)"),
        }
    }

    #[test]
    #[cfg(feature = "libm")] // Relies on built-in pow fallback logic for default exponent
    fn test_pow_arity_eval() {
        // Test evaluation using built-in pow, which handles the default exponent case
        let result = interp("pow(2)", None).unwrap();
        assert_eq!(result, 4.0); // pow(2) -> pow(2, 2) = 4.0

        let result2 = interp("pow(2, 3)", None).unwrap();
        assert_eq!(result2, 8.0);
    }

    #[test]
    #[cfg(not(feature = "libm"))] // Test with explicit pow needed
    fn test_pow_arity_eval_no_builtins() {
        // Create a minimal context with only what we need
        let mut ctx = EvalContext {
            variables: Default::default(),
            constants: Default::default(),
            arrays: Default::default(),
            attributes: Default::default(),
            nested_arrays: Default::default(),
            function_registry: Rc::new(FunctionRegistry::default()),
            parent: None,
            ast_cache: None,
        };

        // Register a pow function that requires exactly 2 arguments
        ctx.register_native_function("pow", 2, |args| args[0].powf(args[1]));

        // Convert to Rc<EvalContext> for interp function
        let ctx_rc = Rc::new(ctx);

        // Debug output for the parsed expression
        let ast = crate::engine::parse_expression("pow(2)").unwrap();
        println!("Parsed expression: {:?}", ast);

        // The parser now automatically adds a second argument (pow(2) -> pow(2, 2))
        // So we need to expect this to succeed, not fail
        let result = interp("pow(2)", Some(ctx_rc.clone())).unwrap();
        assert_eq!(result, 4.0, "pow(2) should be interpreted as pow(2,2) = 4");

        // Test that pow(2,3) works correctly
        let result2 = interp("pow(2, 3)", Some(ctx_rc)).unwrap();
        assert_eq!(result2, 8.0);
    }

    #[test]
    fn test_unknown_variable_and_function_ast() {
        // AST structure test - independent of evaluation context or features
        let ast = parse_expression("sin").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast {
            AstExpr::Variable(ref name) => assert_eq!(name, "sin"),
            _ => panic!("Expected variable node for sin"),
        }
        let ast2 = parse_expression("abs").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast2 {
            AstExpr::Variable(ref name) => assert_eq!(name, "abs"),
            _ => panic!("Expected variable node for abs"),
        }
    }

    #[test]
    fn test_unknown_variable_and_function_eval() {
        // Test evaluation when a function name is used as a variable
        let mut ctx = create_test_context(); // Gets defaults if enabled

        // If builtins disabled, manually add sin/abs so they are known *potential* functions
        #[cfg(not(feature = "libm"))]
        {
            ctx.register_native_function("sin", 1, |args| args[0].sin());
            ctx.register_native_function("abs", 1, |args| args[0].abs());
        }

        // Create Rc once and reuse with clone
        let ctx_rc = Rc::new(ctx);

        // Evaluate AST for variable "sin"
        let sin_var_ast = AstExpr::Variable("sin".to_string());
        let err = eval_ast(&sin_var_ast, Some(ctx_rc.clone())).unwrap_err();
        match err {
            ExprError::Syntax(msg) => {
                assert!(msg.contains("Function 'sin' used without arguments"));
            }
            _ => panic!("Expected Syntax error, got {:?}", err),
        }

        // Evaluate AST for variable "abs"
        let abs_var_ast = AstExpr::Variable("abs".to_string());
        let err2 = eval_ast(&abs_var_ast, Some(ctx_rc.clone())).unwrap_err();
        match err2 {
            ExprError::Syntax(msg) => {
                assert!(msg.contains("Function 'abs' used without arguments"));
            }
            _ => panic!("Expected Syntax error, got {:?}", err2),
        }

        // Test a truly unknown variable
        let unknown_var_ast = AstExpr::Variable("nosuchvar".to_string());
        let err3 = eval_ast(&unknown_var_ast, Some(ctx_rc)).unwrap_err();
        assert!(matches!(err3, ExprError::UnknownVariable { name } if name == "nosuchvar"));
    }

    #[test]
    fn test_override_builtin_native() {
        let mut ctx = create_test_context(); // Start with defaults (if enabled)

        // Override 'sin'
        ctx.register_native_function("sin", 1, |_args| 100.0);
        // Override 'pow'
        ctx.register_native_function("pow", 2, |args| args[0] + args[1]);
        // Also override '^' if it's treated separately by parser/evaluator
        ctx.register_native_function("^", 2, |args| args[0] + args[1]);

        // Create Rc once and reuse with clone
        let ctx_rc = Rc::new(ctx.clone());

        // Test overridden sin
        let val_sin = interp("sin(0.5)", Some(ctx_rc.clone())).unwrap();
        assert_eq!(val_sin, 100.0, "Native 'sin' override failed");

        // Test overridden pow
        let val_pow = interp("pow(3, 4)", Some(ctx_rc.clone())).unwrap();
        assert_eq!(val_pow, 7.0, "Native 'pow' override failed");

        // Test overridden pow using operator ^
        let val_pow_op = interp("3^4", Some(ctx_rc.clone())).unwrap();
        assert_eq!(val_pow_op, 7.0, "Native '^' override failed");

        // Test a non-overridden function still works (cos)
        // Need to ensure 'cos' is available either via defaults or manual registration
        #[cfg(not(feature = "libm"))]
        {
            ctx.register_native_function("cos", 1, |args| args[0].cos()); // Example impl
            // After registration, we need to update our Rc
            let ctx_rc = Rc::new(ctx.clone());
        }
        // If cos wasn't registered by create_test_context and libm is enabled, this might fail
        if ctx.function_registry.native_functions.contains_key(&"cos".try_into_function_name().unwrap()) || cfg!(feature = "libm") {
            let val_cos = interp("cos(0)", Some(ctx_rc.clone())).unwrap();
            // Use approx eq for floating point results
            let expected_cos = 1.0;
            assert!(
                (val_cos - expected_cos).abs() < 1e-9,
                "Built-in/default 'cos' failed after override. Got {}",
                val_cos
            );
        } else {
            // If cos is unavailable, trying to interp it should fail
            let err = interp("cos(0)", Some(ctx_rc)).unwrap_err();
            assert!(matches!(err, ExprError::UnknownFunction { .. }));
        }
    }

    #[test]
    fn test_override_builtin_expression() {
        let mut ctx = create_test_context(); // Start with defaults (if enabled)

        // Override 'cos' with an expression function
        ctx.register_expression_function("cos", &["x"], "x * 10")
            .unwrap();

        // Create Rc once and clone as needed
        let mut ctx_rc = Rc::new(ctx.clone());

        // Override 'max' with an expression function that uses 'min'
        // Ensure 'min' is available first
        #[cfg(not(feature = "libm"))]
        {
            ctx.register_native_function("min", 2, |args| args[0].min(args[1]));
            // Update Rc after modifying ctx
            ctx_rc = Rc::new(ctx.clone());
        }
        // If min wasn't registered by create_test_context and libm is enabled, this might fail
        if ctx.function_registry.native_functions.contains_key(&"min".try_into_function_name().unwrap()) || cfg!(feature = "libm") {
            ctx.register_expression_function("max", &["a", "b"], "min(a, b)")
                .unwrap();
            // Update Rc after modifying ctx
            ctx_rc = Rc::new(ctx.clone());

            // Test overridden max
            let val_max = interp("max(10, 2)", Some(ctx_rc.clone())).unwrap();
            assert_eq!(val_max, 2.0, "Expression 'max' override failed");
        } else {
            // Cannot register max if min is unavailable
            let reg_err = ctx.register_expression_function("max", &["a", "b"], "min(a, b)");
            // Depending on when parsing/checking happens, this might succeed or fail
            // If it succeeds, evaluation will fail later.
            if reg_err.is_ok() {
                // Update Rc after modifying ctx
                ctx_rc = Rc::new(ctx.clone());
                let eval_err = interp("max(10, 2)", Some(ctx_rc.clone())).unwrap_err();
                assert!(matches!(eval_err, ExprError::UnknownFunction { name } if name == "min"));
            }
        }

        // Test overridden cos
        let val_cos = interp("cos(5)", Some(ctx_rc.clone())).unwrap();
        assert_eq!(val_cos, 50.0, "Expression 'cos' override failed");

        // Test a non-overridden function still works (sin)
        #[cfg(not(feature = "libm"))]
        {
            ctx.register_native_function("sin", 1, |args| args[0].sin());
            // Update Rc after modifying ctx
            ctx_rc = Rc::new(ctx.clone());
        }
        if ctx.function_registry.native_functions.contains_key(&"sin".try_into_function_name().unwrap()) || cfg!(feature = "libm") {
            let val_sin = interp("sin(0)", Some(ctx_rc.clone())).unwrap();
            assert!(
                (val_sin - 0.0).abs() < 1e-9,
                "Built-in/default 'sin' failed after override"
            );
        } else {
            let err = interp("sin(0)", Some(ctx_rc)).unwrap_err();
            assert!(matches!(err, ExprError::UnknownFunction { .. }));
        }
    }

    #[test]
    fn test_expression_function_uses_correct_context() {
        let mut ctx = create_test_context(); // Start with defaults (if enabled)
        ctx.set_parameter("a", 10.0).expect("Failed to set parameter"); // Variable in outer context
        ctx.constants.insert("my_const".try_into_heapless().unwrap(), 100.0).expect("Failed to insert constant"); // Constant in outer context

        // Define func1_const(x) = x + my_const
        // Expression functions inherit constants.
        ctx.register_expression_function("func1_const", &["x"], "x + my_const")
            .unwrap();

        // Create Rc and update after each ctx modification
        let mut ctx_rc = Rc::new(ctx.clone());

        let val1 = interp("func1_const(5)", Some(ctx_rc.clone())).unwrap();
        assert_eq!(val1, 105.0, "func1_const should use constant from context");

        // Define func_uses_outer_var(x) = x + a
        ctx.register_expression_function("func_uses_outer_var", &["x"], "x + a")
            .unwrap();
        ctx_rc = Rc::new(ctx.clone());

        // Add a test to check if 'a' is visible inside the function
        let result = interp("func_uses_outer_var(5)", Some(ctx_rc.clone()));
        match result {
            Ok(val) => {
                assert_eq!(
                    val, 15.0,
                    "func_uses_outer_var should use variable 'a' from context"
                );
            }
            Err(e) => {
                println!("Error evaluating func_uses_outer_var(5): {:?}", e);
                panic!(
                    "Expected Ok(15.0) for func_uses_outer_var(5), got error: {:?}",
                    e
                );
            }
        }

        // Add a test for parameter shadowing
        ctx.register_expression_function("shadow_test", &["a"], "a + 1")
            .unwrap();
        ctx_rc = Rc::new(ctx.clone());

        let val_shadow = interp("shadow_test(7)", Some(ctx_rc.clone())).unwrap();
        assert_eq!(
            val_shadow, 8.0,
            "Parameter 'a' should shadow context variable 'a'"
        );

        // Verify original 'a' in outer context is unchanged
        let val_a = interp("a", Some(ctx_rc)).unwrap();
        assert_eq!(val_a, 10.0, "Context 'a' should remain unchanged");
    }

    // Additional tests for polynomial expression function and related checks

    #[test]
    fn test_polynomial_expression_function_direct() {
        let mut ctx = EvalContext::new();
        ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4")
            .unwrap();

        // Create Rc once
        let ctx_rc = Rc::new(ctx);

        // Test for x = 2
        let ast = crate::engine::parse_expression("polynomial(2)").unwrap();
        let result = crate::eval::eval_ast(&ast, Some(ctx_rc.clone())).unwrap();
        assert!(
            (result - 26.0).abs() < 1e-10,
            "Expected 26.0, got {}",
            result
        );

        // Test for x = 3
        let ast = crate::engine::parse_expression("polynomial(3)").unwrap();
        let result = crate::eval::eval_ast(&ast, Some(ctx_rc)).unwrap();
        assert!(
            (result - 58.0).abs() < 1e-10,
            "Expected 58.0, got {}",
            result
        );
    }

    #[test]
    fn test_polynomial_subexpressions() {
        let mut ctx = EvalContext::new();
        ctx.set_parameter("x", 2.0);

        // Create Rc once
        let ctx_rc = Rc::new(ctx);

        // x^3
        let ast = crate::engine::parse_expression("x^3").unwrap();
        let result = crate::eval::eval_ast(&ast, Some(ctx_rc.clone())).unwrap();
        assert_eq!(result, 8.0);

        // 2*x^2
        let ast = crate::engine::parse_expression("2*x^2").unwrap();
        let result = crate::eval::eval_ast(&ast, Some(ctx_rc.clone())).unwrap();
        assert_eq!(result, 8.0);

        // 3*x
        let ast = crate::engine::parse_expression("3*x").unwrap();
        let result = crate::eval::eval_ast(&ast, Some(ctx_rc.clone())).unwrap();
        assert_eq!(result, 6.0);

        // 4
        let ast = crate::engine::parse_expression("4").unwrap();
        let result = crate::eval::eval_ast(&ast, Some(ctx_rc)).unwrap();
        assert_eq!(result, 4.0);
    }

    #[test]
    fn test_operator_precedence() {
        // Create a context with the necessary operators
        let mut ctx = EvalContext::new();

        // Clear any auto-registered functions for clean test
        ctx.function_registry = Rc::new(FunctionRegistry::default());

        // Register the operators needed for the expression
        ctx.register_native_function("+", 2, |args| args[0] + args[1]);
        ctx.register_native_function("*", 2, |args| args[0] * args[1]);
        ctx.register_native_function("^", 2, |args| args[0].powf(args[1]));

        let ast = crate::engine::parse_expression("2 + 3 * 4 ^ 2").unwrap();
        let result = crate::eval::eval_ast(&ast, Some(Rc::new(ctx))).unwrap();
        assert_eq!(result, 2.0 + 3.0 * 16.0); // 2 + 3*16 = 50
    }

    #[test]
    fn test_polynomial_ast_structure() {
        let ast = crate::engine::parse_expression("x^3 + 2*x^2 + 3*x + 4").unwrap();
        // Print the AST for inspection
        println!("{:?}", ast);
        // Optionally, walk the AST and check node types here if desired
    }

    // New test for debugging polynomial function and body evaluation
    #[test]
    fn test_polynomial_integration_debug() {
        let mut ctx = EvalContext::new();
        ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4")
            .unwrap();

        // Print the AST for the polynomial body
        let body_ast = crate::engine::parse_expression_with_reserved(
            "x^3 + 2*x^2 + 3*x + 4",
            Some(&vec!["x".to_string()]),
        )
        .unwrap();
        println!("AST for polynomial body: {:?}", body_ast);

        // Print the AST for polynomial(2)
        let call_ast = crate::engine::parse_expression("polynomial(2)").unwrap();
        println!("AST for polynomial(2): {:?}", call_ast);

        // Create Rc for ctx
        let ctx_rc = Rc::new(ctx.clone());

        // Evaluate polynomial(2)
        let result = crate::eval::eval_ast(&call_ast, Some(ctx_rc)).unwrap();
        println!("polynomial(2) = {}", result);

        // Evaluate the body directly with x=2
        ctx.set_parameter("x", 2.0);
        let ctx_rc2 = Rc::new(ctx);
        let direct_result = crate::eval::eval_ast(&body_ast, Some(ctx_rc2)).unwrap();
        println!("Direct eval with x=2: {}", direct_result);
    }

    // Test for function argument passing and context mapping in polynomial
    #[test]
    fn test_polynomial_argument_mapping_debug() {
        let mut ctx = EvalContext::new();
        ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4")
            .unwrap();

        // Create Rc and update when ctx changes
        let mut ctx_rc = Rc::new(ctx.clone());

        // Test with a literal
        let ast_lit = crate::engine::parse_expression("polynomial(10)").unwrap();
        let result_lit = crate::eval::eval_ast(&ast_lit, Some(ctx_rc.clone())).unwrap();
        println!("polynomial(10) = {}", result_lit);
        assert_eq!(result_lit, 1234.0);

        // Test with a variable
        ctx.set_parameter("z", 10.0);
        ctx_rc = Rc::new(ctx.clone());

        let ast_var = crate::engine::parse_expression("polynomial(z)").unwrap();
        let result_var = crate::eval::eval_ast(&ast_var, Some(ctx_rc.clone())).unwrap();
        println!("polynomial(z) = {}", result_var);
        assert_eq!(result_var, 1234.0);

        // Test with a subexpression
        ctx.set_parameter("a", 5.0);
        ctx.set_parameter("b", 10.0);
        ctx_rc = Rc::new(ctx.clone());

        let ast_sub = crate::engine::parse_expression("polynomial(a + b / 2)").unwrap();
        let result_sub = crate::eval::eval_ast(&ast_sub, Some(ctx_rc.clone())).unwrap();
        println!("polynomial(a + b / 2) = {}", result_sub);
        assert_eq!(result_sub, 1234.0);

        // Test with a nested polynomial call
        let ast_nested = crate::engine::parse_expression("polynomial(polynomial(2))").unwrap();
        let result_nested = crate::eval::eval_ast(&ast_nested, Some(ctx_rc)).unwrap();
        println!("polynomial(polynomial(2)) = {}", result_nested);
    }
    #[test]
    fn test_polynomial_shadowing_variable() {
        let mut ctx = EvalContext::new();
        ctx.set_parameter("x", 100.0); // Shadowing variable
        ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4")
            .unwrap();

        let ctx_rc = Rc::new(ctx);
        let call_ast = crate::engine::parse_expression("polynomial(2)").unwrap();
        let result = crate::eval::eval_ast(&call_ast, Some(ctx_rc)).unwrap();

        assert!(
            (result - 26.0).abs() < 1e-10,
            "Expected 26.0, got {}",
            result
        );
    }

    //============= Recursion Tracking Tests =============//

    #[test]
    fn test_recursion_depth_tracking_reset() {
        // This test is no longer relevant with the iterative evaluator
        // The iterative evaluator doesn't use a global recursion counter
        // Instead, it uses a context stack with a fixed capacity
        
        // We'll test that simple expressions evaluate correctly
        let ast = AstExpr::Constant(42.0);
        let result = eval_ast(&ast, None);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42.0);
        
        // The iterative evaluator automatically cleans up its stack after evaluation
        // so there's no need to check for reset behavior
    }

    #[test]
    fn test_infinite_recursion_detection() {
        // Reset recursion depth to ensure clean test state
        crate::eval::recursion::reset_recursion_depth();
        
        // Test that infinite recursion is properly detected and halted
        let mut ctx = EvalContext::new();

        // Register a function that calls itself without a base case
        ctx.register_expression_function("infinite_recursion", &["x"], "infinite_recursion(x + 1)")
            .unwrap();

        // Try to evaluate - should fail with capacity exceeded
        let result = interp("infinite_recursion(0)", Some(Rc::new(ctx)));

        // Verify we get the expected error
        assert!(result.is_err(), "Should have failed with capacity exceeded");
        match result.unwrap_err() {
            ExprError::CapacityExceeded(resource) => {
                assert_eq!(
                    resource, "context stack",
                    "Expected context stack overflow, got: {}",
                    resource
                );
            }
            other => panic!("Expected CapacityExceeded error, got: {:?}", other),
        }
        
        // The iterative evaluator automatically cleans up on error,
        // so there's no global state to check
    }

    #[test]
    fn test_nested_function_calls() {
        // Test that nested function calls are properly tracked
        let mut ctx = EvalContext::new();

        // Register some simple functions that call each other
        ctx.register_expression_function("double", &["x"], "x * 2")
            .unwrap();
        ctx.register_expression_function("triple", &["x"], "x * 3")
            .unwrap();
        ctx.register_expression_function("add_10", &["x"], "x + 10")
            .unwrap();

        // Create a nested call without recursion
        let result = interp("double(triple(add_10(5)))", Some(Rc::new(ctx.clone())));

        assert!(
            result.is_ok(),
            "Failed to evaluate nested calls: {:?}",
            result.err()
        );
        assert_eq!(
            result.unwrap(),
            2.0 * 3.0 * (5.0 + 10.0),
            "Incorrect result for nested calls"
        );

        // Verify the counter is reset
        let depth = RECURSION_DEPTH.load(Ordering::Relaxed);
        assert!(
            depth < 10,
            "Recursion depth not properly reset after nested calls, got {}",
            depth
        );
    }

    #[test]
    fn test_recursion_depth_with_non_recursive_expressions() {
        // Test that non-recursive expressions don't accumulate recursion depth

        // Reset the counter
        RECURSION_DEPTH.store(0, Ordering::Relaxed);

        // Create a complex but non-recursive expression
        let expr = "1 + 2 * 3 + 4 * 5 + 6 * 7 + 8 * 9 + 10";

        // Evaluate it
        let result = interp(expr, None);

        // Verify it works
        assert!(
            result.is_ok(),
            "Failed to evaluate non-recursive expression: {:?}",
            result.err()
        );
        assert_eq!(
            result.unwrap(),
            1.0 + 2.0 * 3.0 + 4.0 * 5.0 + 6.0 * 7.0 + 8.0 * 9.0 + 10.0
        );

        // Verify the recursion depth stayed low
        let depth = RECURSION_DEPTH.load(Ordering::Relaxed);
        assert!(
            depth < 10,
            "Unexpectedly high recursion depth for non-recursive expr: {}",
            depth
        );
    }

    #[test]
    fn test_recursion_tracking_function_specific() {
        // Test that our recursion tracking is specific to function calls
        // and doesn't track arithmetic or other AST node evaluation

        // Reset counter
        RECURSION_DEPTH.store(0, Ordering::Relaxed);

        // Create a complex expression with many AST nodes but no function calls
        let expr = "(1 + 2) * (3 + 4) * (5 + 6) * (7 + 8) * (9 + 10) * (11 + 12) * (13 + 14)";

        // Create a context with the necessary operators
        let mut ctx = EvalContext::new();
        
        // Register the necessary operators if libm is not enabled
        #[cfg(not(feature = "libm"))]
        {
            ctx.register_native_function("+", 2, |args| args[0] + args[1]);
            ctx.register_native_function("*", 2, |args| args[0] * args[1]);
        }
        
        // Evaluate it with the context
        let result = interp(expr, Some(Rc::new(ctx)));

        // Verify it works
        assert!(result.is_ok());

        // Verify the recursion depth stayed at zero or very low
        // When running without libm, the depth will be higher because
        // the operators are implemented as function calls
        #[cfg(feature = "libm")]
        {
            let depth = RECURSION_DEPTH.load(Ordering::Relaxed);
            assert!(
                depth < 5,
                "Recursion tracking shouldn't count non-function AST nodes, got depth: {}",
                depth
            );
        }
        
        // When not using libm, the test can't be as strict because the operators
        // are implemented as explicit function calls
        #[cfg(not(feature = "libm"))]
        {
            let depth = RECURSION_DEPTH.load(Ordering::Relaxed);
            println!("Without libm, recursion depth is higher due to operator functions: {}", depth);
        }
    }

    // Test for AST caching effect on polynomial evaluation
    #[test]
    fn test_polynomial_ast_cache_effect() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let mut ctx = EvalContext::new();
        ctx.ast_cache = Some(RefCell::new(
            crate::types::AstCacheMap::new(),
        ));
        ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4")
            .unwrap();

        let expr = "polynomial(2)";

        // Create an Rc for ctx
        let ctx_rc = Rc::new(ctx);

        // First evaluation (should parse and cache)
        let result1 = crate::engine::interp(expr, Some(ctx_rc.clone())).unwrap();
        println!("First eval with cache: {}", result1);

        // Second evaluation (should use cache)
        let result2 = crate::engine::interp(expr, Some(ctx_rc)).unwrap();
        println!("Second eval with cache: {}", result2);

        assert_eq!(result1, result2);
        assert!((result1 - 26.0).abs() < 1e-10);
    }

    // Test for function overriding
    #[test]
    fn test_polynomial_function_overriding() {
        let mut ctx = EvalContext::new();
        ctx.register_expression_function("polynomial", &["x"], "x + 1")
            .unwrap();
        ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4")
            .unwrap();

        let ctx_rc = Rc::new(ctx);
        let call_ast = crate::engine::parse_expression("polynomial(2)").unwrap();
        let result = crate::eval::eval_ast(&call_ast, Some(ctx_rc)).unwrap();

        println!("polynomial(2) after overriding = {}", result);
        assert!((result - 26.0).abs() < 1e-10);
    }

    // Test for built-in function name collision
    #[test]
    fn test_polynomial_name_collision_with_builtin() {
        let mut ctx = EvalContext::new();
        // Register a function named "sin" that overrides built-in
        ctx.register_expression_function("sin", &["x"], "x + 100")
            .unwrap();

        let ctx_rc = Rc::new(ctx);
        let call_ast = crate::engine::parse_expression("sin(2)").unwrap();
        let result = crate::eval::eval_ast(&call_ast, Some(ctx_rc)).unwrap();

        println!("sin(2) with override = {}", result);
        assert_eq!(result, 102.0);
    }
}
