mod test_helpers;

#[cfg(test)]
mod unit {
    use bumpalo::Bump;
    use exp_rs::context::EvalContext;
    use exp_rs::engine::interp;
    use exp_rs::error::ExprError;
    use exp_rs::functions::{
        abs, acos, asin, atan, atan2, ceil, comma, cos, cosh, div, dummy, e, exp, floor, fmod, ln,
        log10, mul, neg, pi, pow, sin, sinh, sqrt, sub, tan, tanh,
    };
    use exp_rs::lexer::Lexer;
    use exp_rs::types::{AstExpr, TokenKind};
    use std::rc::Rc;

    use crate::test_helpers::{create_context, hstr, set_attr};

    // Helper function to parse expressions in tests using arena
    fn parse_expression(expr: &str) -> Result<AstExpr<'static>, ExprError> {
        thread_local! {
            static TEST_ARENA: std::cell::RefCell<Bump> = std::cell::RefCell::new(Bump::new());
        }

        TEST_ARENA.with(|arena| {
            let arena = arena.borrow();
            let ast = exp_rs::engine::parse_expression(expr, &*arena)?;
            // SAFETY: We're extending the lifetime for tests only. The arena is thread-local
            // and will live for the duration of the test.
            Ok(unsafe { std::mem::transmute::<AstExpr<'_>, AstExpr<'static>>(ast) })
        })
    }

    /// Helper function to create an eval context with all math functions registered
    fn create_math_context() -> Rc<EvalContext> {
        Rc::new(create_context())
    }

    // --- Focused Unit Tests for Parser/Eval Failure Modes ---

    // --- Additional unit tests for parser internals and edge cases ---

    #[test]
    fn test_parse_standard_chained_function_calls() {
        // sin(cos(tan(x))) => sin(cos(tan(x)))
        let ast =
            parse_expression("sin(cos(tan(x)))").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast {
            AstExpr::Function { name, args } => {
                assert_eq!(name, "sin");
                assert_eq!(args.len(), 1);
                match &args[0] {
                    AstExpr::Function {
                        name: n2,
                        args: args2,
                    } => {
                        assert_eq!(*n2, "cos");
                        assert_eq!(args2.len(), 1);
                        match &args2[0] {
                            AstExpr::Function {
                                name: n3,
                                args: args3,
                            } => {
                                assert_eq!(*n3, "tan");
                                assert_eq!(args3.len(), 1);
                                match &args3[0] {
                                    AstExpr::Variable(var) => assert_eq!(*var, "x"),
                                    _ => panic!("Expected variable as argument to tan"),
                                }
                            }
                            _ => panic!("Expected tan as argument to cos"),
                        }
                    }
                    _ => panic!("Expected cos as argument to sin"),
                }
            }
            _ => panic!("Expected function node for sin cos tan x"),
        }
    }

    #[test]
    fn test_parse_postfix_array_and_attribute_access() {
        // Create the AST by parsing
        let sin_arr = parse_expression("sin(arr[0])").unwrap();

        // Test with the manually created AST
        match &sin_arr {
            AstExpr::Function { name, args } => {
                assert_eq!(*name, "sin");
                assert_eq!(args.len(), 1);
                match &args[0] {
                    AstExpr::Array { name, index } => {
                        assert_eq!(*name, "arr");
                        match **index {
                            AstExpr::Constant(val) => assert_eq!(val, 0.0),
                            _ => panic!("Expected constant as array index"),
                        }
                    }
                    _ => panic!("Expected array as argument to sin"),
                }
            }
            _ => panic!("Expected function node for sin(arr[0])"),
        }

        // Create the AST by parsing
        let foo_bar_x = parse_expression("bar(x)").unwrap();

        // Test with the manually created AST
        match &foo_bar_x {
            AstExpr::Function { name, args } => {
                assert_eq!(*name, "bar");
                assert_eq!(args.len(), 1);
                match &args[0] {
                    AstExpr::Variable(var) => assert_eq!(*var, "x"),
                    _ => panic!("Expected variable as argument to foo.bar"),
                }
            }
            _ => panic!("Expected function node for foo.bar(x)"),
        }
    }

    #[test]
    fn test_parse_postfix_function_call_after_attribute() {
        // foo.bar(1)
        let ast = parse_expression("foo.bar(1)").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast {
            AstExpr::Function { name, args } => {
                assert_eq!(name, "bar");
                assert_eq!(args.len(), 1);
                match &args[0] {
                    AstExpr::Constant(val) => assert_eq!(*val, 1.0),
                    _ => panic!("Expected constant as argument to foo.bar"),
                }
            }
            _ => panic!("Expected function node for foo.bar(1)"),
        }
    }

    #[test]
    fn test_parse_postfix_array_access_complex_index() {
        // arr[1+2*3]
        let ast = parse_expression("arr[1+2*3]").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast {
            AstExpr::Array { name, index } => {
                assert_eq!(name, "arr");
                match *index {
                    AstExpr::Function {
                        name: ref n,
                        args: ref a,
                    } if *n == "+" => {
                        assert_eq!(a.len(), 2);
                    }
                    _ => panic!("Expected function as array index"),
                }
            }
            _ => panic!("Expected array AST node"),
        }
    }

    #[test]
    fn test_parse_postfix_attribute_on_function_result_should_error() {
        // (sin x).foo should fail (attribute on function result)
        let ast = parse_expression("(sin x).foo");
        assert!(
            ast.is_err(),
            "Attribute access on function result should be rejected"
        );
    }

    #[test]
    fn test_parse_comma_in_parens_and_top_level() {
        // (1,2) is allowed
        let ast = parse_expression("(1,2)");
        assert!(ast.is_ok(), "Comma in parens should be allowed");

        // 1,2,3 should now be allowed
        let ast2 = parse_expression("1,2,3");
        assert!(ast2.is_ok(), "Top-level comma should be allowed");

        // (1,2),3 should now be allowed
        let ast3 = parse_expression("(1,2),3");
        assert!(
            ast3.is_ok(),
            "Nested comma outside parens should be allowed"
        );

        // Verify that comma expressions evaluate to the last value
        let val = interp("1,2,3", None).unwrap();
        assert_eq!(
            val, 3.0,
            "Comma expression should evaluate to the last value"
        );
    }

    #[test]
    fn test_parse_binary_op_deep_right_assoc_pow() {
        // 2^2^2^2^2 should be right-associative
        let ast = parse_expression("2^2^2^2^2").unwrap_or_else(|e| panic!("Parse error: {}", e));
        fn count_right_assoc_pow(expr: &AstExpr) -> usize {
            match expr {
                AstExpr::Function { name, args } if *name == "^" && args.len() == 2 => {
                    1 + count_right_assoc_pow(&args[1])
                }
                _ => 0,
            }
        }
        let pow_depth = count_right_assoc_pow(&ast);
        assert_eq!(pow_depth, 4, "Should be right-associative chain of 4 '^'");
    }

    #[test]
    fn test_parse_binary_op_mixed_unary_and_power() {
        // -2^2, (-2)^2, -2^-2
        let ast = parse_expression("-2^2").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast {
            AstExpr::Function { name, args } if name == "neg" => match &args[0] {
                AstExpr::Function {
                    name: n2,
                    args: args2,
                } if *n2 == "^" => {
                    assert_eq!(args2.len(), 2);
                }
                _ => panic!("Expected ^ as argument to neg"),
            },
            _ => panic!("Expected neg as top-level function"),
        }
        let ast2 = parse_expression("(-2)^2").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast2 {
            AstExpr::Function { name, args } if name == "^" => match &args[0] {
                AstExpr::Function {
                    name: n2,
                    args: args2,
                } if *n2 == "neg" => {
                    assert_eq!(args2.len(), 1);
                }
                _ => panic!("Expected neg as left arg to ^"),
            },
            _ => panic!("Expected ^ as top-level function"),
        }
        let ast3 = parse_expression("-2^-2").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast3 {
            AstExpr::Function { name, args } if name == "neg" => match &args[0] {
                AstExpr::Function {
                    name: n2,
                    args: args2,
                } if *n2 == "^" => {
                    assert_eq!(args2.len(), 2);
                }
                _ => panic!("Expected ^ as argument to neg"),
            },
            _ => panic!("Expected neg as top-level function"),
        }
    }

    #[test]
    fn test_parse_binary_op_mixed_precedence() {
        // 2+3*4^2-5/6
        let ast = parse_expression("2+3*4^2-5/6").unwrap_or_else(|e| panic!("Parse error: {}", e));
        // Just check that the top-level node is '-' and the tree is not flat
        match ast {
            AstExpr::Function { name, args } if name == "-" => {
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected - as top-level function"),
        }
    }

    #[test]
    fn test_parse_primary_paren_errors() {
        // ((1+2) is invalid
        let ast = parse_expression("((1+2)");
        assert!(ast.is_err(), "Unmatched parenthesis should be rejected");
        // 1+) is invalid
        let ast2 = parse_expression("1+)");
        assert!(ast2.is_err(), "Unmatched parenthesis should be rejected");
    }

    #[test]
    fn test_parse_primary_variable_and_number_edge_cases() {
        // Variable names with underscores and digits
        let ast = parse_expression("foo_bar123").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast {
            AstExpr::Variable(name) => assert_eq!(name, "foo_bar123"),
            _ => panic!("Expected variable node"),
        }
        // Numbers with various formats
        let ast2 = parse_expression(".5").unwrap();
        match ast2 {
            AstExpr::Constant(val) => assert_eq!(val, 0.5),
            _ => panic!("Expected constant node"),
        }
        let ast3 = parse_expression("1e-2").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast3 {
            AstExpr::Constant(val) => assert!((val - 0.01).abs() < 1e-10),
            _ => panic!("Expected constant node"),
        }
        let ast4 = parse_expression("1.2e+3").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast4 {
            AstExpr::Constant(val) => assert!((val - 1200.0).abs() < 1e-10),
            _ => panic!("Expected constant node"),
        }
    }

    #[test]
    fn test_parse_decimal_with_leading_dot() {
        // This test should now pass with our improved lexer implementation
        let ast = parse_expression(".5").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast {
            AstExpr::Constant(val) => assert_eq!(val, 0.5),
            _ => panic!("Expected constant node"),
        }

        // Test more complex expressions with leading dots
        let _ast2 = parse_expression(".25 + .75").unwrap_or_else(|e| panic!("Parse error: {}", e));
        let result = interp(".25 + .75", None).unwrap();
        assert_eq!(result, 1.0);

        // Test scientific notation with leading dot
        let ast3 = parse_expression(".5e2").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast3 {
            AstExpr::Constant(val) => assert_eq!(val, 50.0),
            _ => panic!("Expected constant node"),
        }

        // Test in a more complex expression
        let result2 = interp("sin(.5) + cos(.5)", Some(create_math_context())).unwrap();

        // Calculate expected value through context to avoid direct function calls when libm is unavailable
        let ctx = create_math_context();
        let expected_sin = interp("sin(.5)", Some(ctx.clone())).unwrap();
        let expected_cos = interp("cos(.5)", Some(ctx.clone())).unwrap();
        let expected = expected_sin + expected_cos;

        #[cfg(feature = "f32")]
        assert!((result2 - expected).abs() < 1e-6);
        #[cfg(not(feature = "f32"))]
        assert!((result2 - expected).abs() < 1e-10);
    }

    #[test]
    fn test_lexer_tokenization_all_types() {
        let mut lexer = Lexer::new("1 + foo_bar * (2.5e-1) , -baz_123 / 4.2 ^ _x");
        let mut tokens = Vec::new();
        while let Some(tok) = lexer.next_token() {
            tokens.push(tok);
        }
        // Should contain Number, Operator, Variable, Operator, Open, Number, Close, Separator, Operator, Variable, Operator, Number, Operator, Variable
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Number));
        assert!(kinds.contains(&TokenKind::Operator));
        assert!(kinds.contains(&TokenKind::Variable));
        assert!(kinds.contains(&TokenKind::Open));
        assert!(kinds.contains(&TokenKind::Close));
        assert!(kinds.contains(&TokenKind::Separator));
    }

    #[test]
    fn test_lexer_tokenization_error_tokens() {
        let mut lexer = Lexer::new("1 $ 2");
        let mut found_error = false;
        while let Some(tok) = lexer.next_token() {
            if tok.kind == TokenKind::Error {
                found_error = true;
                break;
            }
        }
        assert!(
            found_error,
            "Lexer should produce error token for unknown character"
        );
    }

    #[test]
    fn test_lexer_tokenization_malformed_numbers() {
        let mut lexer = Lexer::new("1..2 1e--2");
        let mut found_error = false;
        while let Some(tok) = lexer.next_token() {
            if tok.kind == TokenKind::Error {
                found_error = true;
            }
        }
        assert!(
            found_error,
            "Lexer should produce error token for malformed numbers"
        );
    }

    #[test]
    fn test_eval_ast_array_and_attribute_errors() {
        use exp_rs::eval::eval_ast;
        let arena = Bump::new();

        // Array not found
        let ast = parse_expression("arr[0]").unwrap();
        let err = eval_ast(&ast, None, &arena).unwrap_err();
        match err {
            ExprError::UnknownVariable { name } => assert_eq!(name, "arr"),
            _ => panic!("Expected UnknownVariable error"),
        }
        // Attribute not found
        let ast2 = parse_expression("foo.bar").unwrap();
        let err2 = eval_ast(&ast2, None, &arena).unwrap_err();
        match err2 {
            ExprError::AttributeNotFound { base, attr } => {
                assert_eq!(base, "foo");
                assert_eq!(attr, "bar");
            }
            _ => panic!("Expected AttributeNotFound error"),
        }
    }

    #[test]
    fn test_eval_ast_function_wrong_arity() {
        use exp_rs::eval::eval_ast;
        let arena = Bump::new();

        // sin with 2 args (should be 1)
        let ast = parse_expression("sin(1, 2)").unwrap();
        let err = eval_ast(&ast, Some(create_math_context()), &arena).unwrap_err();
        match err {
            ExprError::InvalidFunctionCall {
                name,
                expected,
                found,
            } => {
                assert_eq!(name, "sin");
                assert_eq!(expected, 1);
                assert_eq!(found, 2);
            }
            _ => panic!("Expected InvalidFunctionCall error"),
        }
    }

    #[test]
    fn test_eval_ast_unknown_function_and_variable() {
        use exp_rs::eval::eval_ast;
        let arena = Bump::new();

        // Unknown function
        let ast = parse_expression("notafunc(1)").unwrap();
        let err = eval_ast(&ast, None, &arena).unwrap_err();
        match err {
            ExprError::UnknownFunction { name } => assert_eq!(name, "notafunc"),
            _ => panic!("Expected UnknownFunction error"),
        }
        // Unknown variable
        let ast2 = parse_expression("notavar").unwrap();
        let err2 = eval_ast(&ast2, None, &arena).unwrap_err();
        match err2 {
            ExprError::UnknownVariable { name } => assert_eq!(name, "notavar"),
            _ => panic!("Expected UnknownVariable error"),
        }
    }

    // --- End additional parser/eval unit tests ---

    #[test]
    fn test_neg_pow_ast() {
        let ast = parse_expression("-2^2").unwrap();
        println!("AST for -2^2: {:?}", ast);
        // Should be AstExpr::Function { name: "neg", args: [AstExpr::Function { name: "^", args: [2, 2] }] }
        match ast {
            AstExpr::Function { ref name, ref args } if *name == "neg" => {
                assert_eq!(args.len(), 1);
                match &args[0] {
                    AstExpr::Function {
                        name: pow_name,
                        args: pow_args,
                    } if *pow_name == "^" => {
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
    fn test_neg_pow_eval() {
        let val = interp("-2^2", None).unwrap();
        assert_eq!(val, -4.0);
        let val2 = interp("(-2)^2", None).unwrap();
        assert_eq!(val2, 4.0);
    }

    #[test]
    fn test_paren_neg_pow_ast() {
        let ast = parse_expression("(-2)^2").unwrap();
        println!("AST for (-2)^2: {:?}", ast);
        // Should be AstExpr::Function { name: "^", args: [AstExpr::Function { name: "neg", args: [2] }, 2] }
        match ast {
            AstExpr::Function { ref name, ref args } if *name == "^" => {
                assert_eq!(args.len(), 2);
                match &args[0] {
                    AstExpr::Function {
                        name: neg_name,
                        args: neg_args,
                    } if *neg_name == "neg" => {
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
    fn test_function_application_standard_syntax_ast() {
        let ast = parse_expression("sin(x)").unwrap();
        println!("AST for sin(x): {:?}", ast);
        match ast {
            AstExpr::Function { ref name, ref args } if *name == "sin" => {
                assert_eq!(args.len(), 1);
                match &args[0] {
                    AstExpr::Variable(var) => assert_eq!(*var, "x"),
                    _ => panic!("Expected variable as argument"),
                }
            }
            _ => panic!("Expected function node for sin x"),
        }

        let ast2 = parse_expression("abs(-42)").unwrap();
        println!("AST for abs(-42): {:?}", ast2);
        match ast2 {
            AstExpr::Function { ref name, ref args } if *name == "abs" => {
                assert_eq!(args.len(), 1);
                match &args[0] {
                    AstExpr::Function {
                        name: n2,
                        args: args2,
                    } if *n2 == "neg" => {
                        assert_eq!(args2.len(), 1);
                        match &args2[0] {
                            AstExpr::Constant(c) => assert_eq!(*c, 42.0),
                            _ => panic!("Expected constant as neg arg"),
                        }
                    }
                    _ => panic!("Expected neg as argument to abs"),
                }
            }
            _ => panic!("Expected function node for abs(-42)"),
        }
    }

    #[test]
    fn test_function_application_standard_syntax_eval() {
        // Create context with math functions for reliable testing
        let ctx = create_math_context();

        let val = interp("abs(abs(abs(abs(abs(-42)))))", Some(ctx.clone())).unwrap();
        assert_eq!(val, 42.0);
    }

    #[test]
    fn test_pow_arity_ast() {
        let ast = parse_expression("pow(2)").unwrap();
        println!("AST for pow(2): {:?}", ast);
        match ast {
            AstExpr::Function { ref name, ref args } if *name == "pow" => {
                // We now expect 2 arguments because we add a default second argument
                assert_eq!(args.len(), 2);
                match &args[0] {
                    AstExpr::Constant(c) => assert_eq!(*c, 2.0),
                    _ => panic!("Expected constant as pow arg"),
                }
                // Check the second argument is 2.0 (default)
                match &args[1] {
                    AstExpr::Constant(c) => assert_eq!(*c, 2.0),
                    _ => panic!("Expected constant as second pow arg"),
                }
            }
            _ => panic!("Expected function node for pow(2)"),
        }
    }

    #[test]
    fn test_pow_arity_eval() {
        // Since we now automatically add a second argument to pow(2),
        // we need to modify this test to check that it evaluates correctly
        let result = interp("pow(2)", None).unwrap();
        println!("pow(2) = {}", result); // Debug output
        assert_eq!(result, 4.0); // pow(2, 2) = 4.0

        // Let's also test that pow with explicit arguments works
        let result2 = interp("pow(2, 3)", None).unwrap();
        println!("pow(2, 3) = {}", result2); // Debug output
        assert_eq!(result2, 8.0); // pow(2, 3) = 8.0
    }

    #[test]
    fn test_unknown_variable_and_function_ast() {
        let ast = parse_expression("sin").unwrap();
        println!("AST for sin: {:?}", ast);
        match ast {
            AstExpr::Variable(ref name) => assert_eq!(*name, "sin"),
            _ => panic!("Expected variable node for sin"),
        }
        let ast2 = parse_expression("abs").unwrap();
        println!("AST for abs: {:?}", ast2);
        match ast2 {
            AstExpr::Variable(ref name) => assert_eq!(*name, "abs"),
            _ => panic!("Expected variable node for abs"),
        }
    }

    #[test]
    fn test_unknown_variable_and_function_eval() {
        let err = interp("sin", None).unwrap_err();
        match err {
            ExprError::Syntax(msg) => {
                assert!(
                    msg.contains("Function 'sin' used without arguments"),
                    "Expected error about function used without arguments"
                );
            }
            _ => panic!("Expected Syntax error for function used without arguments"),
        }

        let err2 = interp("abs", None).unwrap_err();
        match err2 {
            ExprError::Syntax(msg) => {
                assert!(
                    msg.contains("Function 'abs' used without arguments"),
                    "Expected error about function used without arguments"
                );
            }
            _ => panic!("Expected Syntax error for function used without arguments"),
        }
    }

    // --- End focused parser/eval tests ---

    // Legacy macro removed.

    #[test]
    fn test_array_access() {
        // Setup context with an array
        let mut ctx = EvalContext::default();
        ctx.arrays
            .insert(hstr("climb_wave_wait_time"), vec![10.0, 20.0, 30.0])
            .expect("Failed to insert array");
        // Use the new API to parse and evaluate the array access expression
        let val = interp(
            "climb_wave_wait_time[1]",
            Some(std::rc::Rc::new(ctx.clone())),
        )
        .unwrap();
        assert_eq!(val, 20.0);
    }

    #[test]
    fn test_array_access_ast_structure() {
        // Setup context with an array
        let mut ctx = EvalContext::default();
        ctx.arrays
            .insert(hstr("climb_wave_wait_time"), vec![10.0, 20.0, 30.0])
            .expect("Failed to insert array");
        // Parse the array access expression using the new API
        let ast = parse_expression("climb_wave_wait_time[1]").unwrap();
        match ast {
            exp_rs::types::AstExpr::Array { name, index } => {
                assert_eq!(name, "climb_wave_wait_time");
                match *index {
                    exp_rs::types::AstExpr::Constant(val) => assert_eq!(val, 1.0),
                    _ => panic!("Expected constant index"),
                }
            }
            _ => panic!("Expected array AST node"),
        }
    }

    #[test]
    #[should_panic(expected = "called dummy!")]
    fn test_dummy_panics() {
        // Should panic when called
        dummy(1.0, 2.0);
    }

    // (Removed tests for optimize_constant, optimize_simple_add, eval_constant_and_function, and any other Expr/ExprType usage)

    // --- New unit tests for parser components ---

    #[test]
    fn test_attribute_access() {
        // Setup context with attributes
        let mut ctx = EvalContext::default();
        set_attr(&mut ctx, "foo", "bar", 42.0);

        let val = interp("foo.bar", Some(std::rc::Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val, 42.0);
    }

    // All legacy parser/tokenizer and State-based tests removed.
    // Only keep tests that use the new API (interp, AST, etc).

    #[test]
    fn test_sub() {
        assert_eq!(sub(5.0, 3.0), 2.0);
    }

    #[test]
    fn test_mul() {
        assert_eq!(mul(2.0, 3.0), 6.0);
    }

    #[test]
    fn test_div() {
        assert_eq!(div(6.0, 3.0), 2.0);
    }

    #[test]
    fn test_fmod() {
        assert_eq!(fmod(7.0, 4.0), 3.0);
    }

    #[test]
    fn test_neg() {
        assert_eq!(neg(5.0, 0.0), -5.0);
    }

    #[test]
    fn test_comma() {
        assert_eq!(comma(1.0, 2.0), 2.0);
    }

    #[test]
    fn test_abs() {
        assert_eq!(abs(-5.0, 0.0), 5.0);
    }

    #[test]
    fn test_acos() {
        #[cfg(feature = "libm")]
        assert!((acos(1.0, 0.0) - 0.0).abs() < 1e-10);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("acos(1)", Some(ctx.clone())).unwrap();
        assert!((result - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_asin() {
        #[cfg(feature = "libm")]
        assert!((asin(0.0, 0.0) - 0.0).abs() < 1e-10);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("asin(0)", Some(ctx.clone())).unwrap();
        assert!((result - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_atan() {
        #[cfg(feature = "libm")]
        assert!((atan(0.0, 0.0) - 0.0).abs() < 1e-10);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("atan(0)", Some(ctx.clone())).unwrap();
        assert!((result - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_atan2() {
        #[cfg(all(feature = "libm", feature = "f32"))]
        assert!((atan2(1.0, 1.0) - core::f32::consts::FRAC_PI_4).abs() < 1e-10);
        #[cfg(all(feature = "libm", not(feature = "f32")))]
        assert!((atan2(1.0, 1.0) - core::f64::consts::FRAC_PI_4).abs() < 1e-10);

        // For all feature combinations, test atan2 via the context
        let ctx = create_math_context();
        let result = interp("atan2(1, 1)", Some(ctx.clone())).unwrap();

        #[cfg(feature = "f32")]
        let expected = core::f32::consts::FRAC_PI_4;
        #[cfg(not(feature = "f32"))]
        let expected = core::f64::consts::FRAC_PI_4;

        assert!((result - expected).abs() < 1e-6, "atan2(1,1) should be π/4");
    }

    #[test]
    fn test_ceil() {
        assert_eq!(ceil(2.3, 0.0), 3.0);
    }

    #[test]
    fn test_cos() {
        #[cfg(feature = "libm")]
        assert!((cos(0.0, 0.0) - 1.0).abs() < 1e-10);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("cos(0)", Some(ctx.clone())).unwrap();
        assert!((result - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cosh() {
        #[cfg(feature = "libm")]
        assert!((cosh(0.0, 0.0) - 1.0).abs() < 1e-10);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("cosh(0)", Some(ctx.clone())).unwrap();
        assert!((result - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_e() {
        #[cfg(feature = "libm")]
        assert!((e(0.0, 0.0) - exp_rs::constants::E).abs() < exp_rs::constants::TEST_PRECISION);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("e()", Some(ctx.clone())).unwrap();
        assert!((result - exp_rs::constants::E).abs() < exp_rs::constants::TEST_PRECISION);
    }

    #[test]
    fn test_exp() {
        #[cfg(all(feature = "libm", feature = "f32"))]
        assert!((exp(1.0, 0.0) - core::f32::consts::E).abs() < 1e-10);
        #[cfg(all(feature = "libm", not(feature = "f32")))]
        assert!((exp(1.0, 0.0) - core::f64::consts::E).abs() < 1e-10);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("exp(1)", Some(ctx.clone())).unwrap();

        #[cfg(feature = "f32")]
        let expected = core::f32::consts::E;
        #[cfg(not(feature = "f32"))]
        let expected = core::f64::consts::E;

        assert!((result - expected).abs() < 1e-6);
    }

    #[test]
    fn test_floor() {
        #[cfg(feature = "libm")]
        assert_eq!(floor(2.7, 0.0), 2.0);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("floor(2.7)", Some(ctx.clone())).unwrap();
        assert_eq!(result, 2.0);
    }

    #[test]
    fn test_ln() {
        #[cfg(feature = "libm")]
        assert!((ln(exp_rs::constants::E, 0.0) - 1.0).abs() < exp_rs::constants::TEST_PRECISION);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("ln(e())", Some(ctx.clone())).unwrap();
        assert!((result - 1.0).abs() < exp_rs::constants::TEST_PRECISION);
    }

    #[test]
    fn test_log() {
        // Import the log function from exp_rs::functions
        use exp_rs::functions::log;

        #[cfg(feature = "libm")]
        {
            assert!((log(1000.0, 0.0) - 3.0).abs() < 1e-10);
            assert!((log(100.0, 0.0) - 2.0).abs() < 1e-10);
            assert!((log(10.0, 0.0) - 1.0).abs() < 1e-10);
        }

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result1 = interp("log(1000)", Some(ctx.clone())).unwrap();
        assert!((result1 - 3.0).abs() < 1e-10);

        let result2 = interp("log(100)", Some(ctx.clone())).unwrap();
        assert!((result2 - 2.0).abs() < 1e-10);

        let result3 = interp("log(10)", Some(ctx.clone())).unwrap();
        assert!((result3 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_log10() {
        #[cfg(feature = "libm")]
        assert!((log10(1000.0, 0.0) - 3.0).abs() < 1e-10);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("log10(1000)", Some(ctx.clone())).unwrap();
        assert!((result - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_pi() {
        #[cfg(feature = "libm")]
        assert!((pi(0.0, 0.0) - exp_rs::constants::PI).abs() < exp_rs::constants::TEST_PRECISION);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("pi()", Some(ctx.clone())).unwrap();
        assert!((result - exp_rs::constants::PI).abs() < exp_rs::constants::TEST_PRECISION);
    }

    #[test]
    fn test_pow() {
        #[cfg(feature = "libm")]
        assert_eq!(pow(2.0, 3.0), 8.0);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("pow(2, 3)", Some(ctx.clone())).unwrap();
        assert_eq!(result, 8.0);
    }

    #[test]
    fn test_sin() {
        #[cfg(feature = "libm")]
        assert!((sin(0.0, 0.0) - 0.0).abs() < 1e-10);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("sin(0)", Some(ctx.clone())).unwrap();
        assert!((result - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_sinh() {
        #[cfg(feature = "libm")]
        assert!((sinh(0.0, 0.0) - 0.0).abs() < 1e-10);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("sinh(0)", Some(ctx.clone())).unwrap();
        assert!((result - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_sqrt() {
        #[cfg(feature = "libm")]
        assert_eq!(sqrt(4.0, 0.0), 2.0);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("sqrt(4)", Some(ctx.clone())).unwrap();
        assert_eq!(result, 2.0);
    }

    #[test]
    fn test_tan() {
        #[cfg(feature = "libm")]
        assert!((tan(0.0, 0.0) - 0.0).abs() < 1e-10);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("tan(0)", Some(ctx.clone())).unwrap();
        assert!((result - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_tanh() {
        #[cfg(feature = "libm")]
        assert!((tanh(0.0, 0.0) - 0.0).abs() < 1e-10);

        // For all feature combinations, test via context
        let ctx = create_math_context();
        let result = interp("tanh(0)", Some(ctx.clone())).unwrap();
        assert!((result - 0.0).abs() < 1e-10);
    }

    // All legacy parser/tokenizer and State-based tests removed.
    // Only keep tests that use the new API (interp, AST, etc).
    // --- Parser and evaluator error propagation and AST structure tests ---

    #[test]
    fn test_parser_operator_precedence() {
        // use exp_rs::engine::parse_expression; // Using the helper function instead
        use exp_rs::types::AstExpr;

        // 2+3*4 should parse as 2 + (3*4)
        let ast = parse_expression("2+3*4").unwrap();
        match ast {
            AstExpr::Function { name, args } => {
                assert_eq!(name, "+");
                assert_eq!(args.len(), 2);
                match &args[1] {
                    AstExpr::Function {
                        name: n2,
                        args: args2,
                    } => {
                        assert_eq!(*n2, "*");
                        assert_eq!(args2.len(), 2);
                    }
                    _ => panic!("Expected multiplication as right child"),
                }
            }
            _ => panic!("Expected function node"),
        }
    }

    #[test]
    fn test_parser_right_associativity_pow() {
        // use exp_rs::engine::parse_expression; // Using the helper function instead
        use exp_rs::types::AstExpr;

        // 2^2^2^2 should parse as 2^(2^(2^2))
        let ast = parse_expression("2^2^2^2").unwrap();
        // Should be AstExpr::Function("^", [2, AstExpr::Function("^", [2, AstExpr::Function("^", [2, 2])])])
        fn count_right_assoc_pow(expr: &AstExpr) -> usize {
            match expr {
                AstExpr::Function { name, args } if *name == "^" && args.len() == 2 => {
                    1 + count_right_assoc_pow(&args[1])
                }
                _ => 0,
            }
        }
        let pow_depth = count_right_assoc_pow(&ast);
        assert_eq!(pow_depth, 3, "Should be right-associative chain of 3 '^'");
    }

    #[test]
    fn test_parser_function_call_and_juxtaposition() {
        // use exp_rs::engine::parse_expression; // Using the helper function instead
        use exp_rs::types::AstExpr;

        // pow(2,2)
        let ast = parse_expression("pow(2,2)").unwrap();
        match ast {
            AstExpr::Function { name, args } => {
                assert_eq!(name, "pow");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected function node"),
        }

        // sin(x)
        let ast2 = parse_expression("sin(x)").unwrap();
        match ast2 {
            AstExpr::Function { name, args } => {
                assert_eq!(name, "sin");
                assert_eq!(args.len(), 1);
                match &args[0] {
                    AstExpr::Variable(var) => assert_eq!(*var, "x"),
                    _ => panic!("Expected variable as argument"),
                }
            }
            _ => panic!("Expected function node"),
        }
    }

    #[test]
    fn test_parser_error_cases() {
        // use exp_rs::engine::parse_expression; // Using the helper function instead

        // pow(2) should fail (arity error at eval, but parse should succeed)
        let ast = parse_expression("pow(2)");
        assert!(
            ast.is_ok(),
            "Parser should allow pow(2), arity error at eval"
        );

        // Top-level comma should now be allowed
        let ast2 = parse_expression("1,2,3");
        assert!(ast2.is_ok(), "Top-level comma expression should be allowed");

        // Unmatched parenthesis
        let ast3 = parse_expression("(1+2");
        assert!(ast3.is_err(), "Unmatched parenthesis should be rejected");
    }

    #[test]
    fn test_eval_unknown_variable_and_function() {
        // use exp_rs::engine::parse_expression; // Using the helper function instead
        use exp_rs::error::ExprError;
        let arena = Bump::new();

        // Unknown variable
        let ast = parse_expression("foo").unwrap();
        let err = exp_rs::eval::eval_ast(&ast, None, &arena).unwrap_err();
        match err {
            ExprError::UnknownVariable { name } => assert_eq!(name, "foo"),
            _ => panic!("Expected UnknownVariable error"),
        }

        // Unknown function
        let ast2 = parse_expression("bar(1)").unwrap();
        let err2 = exp_rs::eval::eval_ast(&ast2, None, &arena).unwrap_err();
        match err2 {
            ExprError::UnknownFunction { name } => assert_eq!(name, "bar"),
            _ => panic!("Expected UnknownFunction error"),
        }
    }

    #[test]
    fn test_eval_invalid_function_arity() {
        // use exp_rs::engine::parse_expression; // Using the helper function instead
        use exp_rs::error::ExprError;
        use exp_rs::eval::eval_ast;
        let arena = Bump::new();

        // Use sin with 2 args instead of pow(2) since pow now has special handling
        let ast = parse_expression("sin(1, 2)").unwrap();
        let err = eval_ast(&ast, Some(create_math_context()), &arena).unwrap_err();
        match err {
            ExprError::InvalidFunctionCall {
                name,
                expected,
                found,
            } => {
                assert_eq!(name, "sin");
                assert_eq!(expected, 1);
                assert_eq!(found, 2);
            }
            _ => panic!("Expected InvalidFunctionCall error"),
        }
    }

    #[test]
    fn test_eval_top_level_comma() {
        // use exp_rs::engine::parse_expression; // Using the helper function instead
        // parse_expression should now accept top-level comma
        let ast = parse_expression("1,2");
        assert!(ast.is_ok(), "Top-level comma should be accepted by parser");

        // Verify the result is the last value
        let val = interp("1,2", None).unwrap();
        assert_eq!(
            val, 2.0,
            "Comma expression should evaluate to the last value"
        );
    }
}
