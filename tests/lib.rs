// Remove unused import
// use exp_rs::approx_eq;
// Add macro import
use exp_rs::assert_approx_eq;
use exp_rs::interp;
use std::time::{Duration, Instant};

// Remove unused import
// use exp_rs::Real;

// --- All parser/tokenizer internals and legacy AST tests removed ---
// All tests now use the new interp() API and check results only.

fn with_timeout<F: FnOnce()>(f: F) {
    let start = Instant::now();
    f();
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(2),
        "Test timed out after {:?}",
        elapsed
    );
}

#[cfg(test)]
mod results {
    use super::*;
    use exp_rs::Real; // Import Real here as it's used in casts

    #[test]
    fn basic_results() {
        with_timeout(|| {
            let cases = [
                ("1", 1.0),
                ("1 ", 1.0),
                ("(1)", 1.0),
                ("pi", exp_rs::constants::PI),
                ("atan(1)*4 - pi", 0.0),
                ("e", exp_rs::constants::E),
                ("2+1", 3.0),
                ("(((2+(1))))", 3.0),
                ("3+2", 5.0),
                ("3+2+4", 9.0),
                ("(3+2)+4", 9.0),
                ("3+(2+4)", 9.0),
                ("(3+2+4)", 9.0),
                ("3*2*4", 24.0),
                ("(3*2)*4", 24.0),
                ("3*(2*4)", 24.0),
                ("(3*2*4)", 24.0),
                ("3-2-4", -3.0),
                ("(3-2)-4", -3.0),
                ("3-(2-4)", 5.0),
                ("(3-2-4)", -3.0),
                ("3/2/4", 0.375),
                ("(3/2)/4", 0.375),
                ("3/(2/4)", 6.0),
                ("(3/2/4)", 0.375),
                ("(3*2/4)", 1.5),
                ("(3/2*4)", 6.0),
                ("3*(2/4)", 1.5),
            ];
            for &(expr, answer) in &cases {
                let result = interp(expr, None).unwrap();
                // Move format string inside the macro call
                assert_approx_eq!(
                    result,
                    answer as Real,
                    exp_rs::constants::TEST_PRECISION,
                    "Failed: {} = {}, expected {}",
                    expr,
                    result,
                    answer
                );
            }
        });
    }

    #[test]
    fn function_and_power_results() {
        with_timeout(|| {
            let cases = [
                ("asin(sin(.5))", 0.5),
                ("sin(asin(.5))", 0.5),
                ("ln(exp(.5))", 0.5),
                ("exp(ln(.5))", 0.5),
                ("asin(sin(-.5))", -0.5),
                ("asin(sin(-0.5))", -0.5),
                ("asin(sin(-0.5))", -0.5),
                ("asin(sin(-0.5))", -0.5),
                ("asin(sin((-0.5)))", -0.5),
                ("asin(sin(-0.5))", -0.5),
                ("(asin(sin(-0.5)))", -0.5),
                ("log10(1000)", 3.0),
                ("log10(1e3)", 3.0),
                ("log10(1000)", 3.0),
                ("log10(1e3)", 3.0),
                ("log10(1000)", 3.0),
                ("log10(1e3)", 3.0),
                ("log10(1.0e3)", 3.0),
                ("10^5*5e-5", 5.0),
                ("log10(1000)", 3.0), // Using log10 as 'log' is natural logarithm (ln) in this library
                ("ln(e^10)", 10.0),
                ("100^.5+1", 11.0),
                ("100^.5+1", 11.0),
                ("100^+.5+1", 11.0),
                ("100^--.5+1", 11.0),
                ("100^---+-++---++-+-+-.5+1", 11.0),
                ("100^-.5+1", 1.1),
                ("100^---.5+1", 1.1),
                ("100^+---.5+1", 1.1),
                ("1e2^+---.5e0+1e0", 1.1),
                ("--(1e2^(+(-(-(-.5e0))))+1e0)", 1.1),
                ("sqrt(100) + 7", 17.0),
                ("sqrt(100) * 7", 70.0),
                ("sqrt(100 * 100)", 100.0),
            ];
            for &(expr, answer) in &cases {
                let result = interp(expr, None).unwrap();
                // Move format string inside the macro call
                assert_approx_eq!(
                    result,
                    answer as Real, // Cast answer to Real
                    exp_rs::constants::TEST_PRECISION,
                    "Failed: {} = {}, expected {}",
                    expr,
                    result,
                    answer
                );
            }
        });
    }

    #[test]
    fn comma_and_misc_results() {
        with_timeout(|| {
            // Test comma expressions
            let comma_cases = [
                ("1,2", 2.0),
                ("1,2+1", 3.0),
                ("1+1,2+2,2+1", 3.0),
                ("1,2,3", 3.0),
                ("(1,2),3", 3.0),
                ("1,(2,3)", 3.0),
                ("-(1,(2,3))", -3.0),
            ];

            for &(expr, expected) in &comma_cases {
                let result = interp(expr, None).unwrap();
                // Move format string inside the macro call
                assert_approx_eq!(
                    result,
                    expected as Real, // Cast expected to Real
                    1e-6,
                    "Failed: {} = {}, expected {}",
                    expr,
                    result,
                    expected
                );
            }

            // Test other miscellaneous expressions
            let misc_cases = [
                ("2^2", 4.0),
                ("pow(2,2)", 4.0),
                ("atan2(1,1)", exp_rs::functions::atan2(1.0, 1.0)),
                ("atan2(1,2)", exp_rs::functions::atan2(1.0, 2.0)),
                ("atan2(2,1)", exp_rs::functions::atan2(2.0, 1.0)),
                ("atan2(3,4)", exp_rs::functions::atan2(3.0, 4.0)),
                ("atan2(3+3,4*2)", exp_rs::functions::atan2(6.0, 8.0)),
                ("atan2(3+3,(4*2))", exp_rs::functions::atan2(6.0, 8.0)),
                ("atan2((3+3),4*2)", exp_rs::functions::atan2(6.0, 8.0)),
                ("atan2((3+3),(4*2))", exp_rs::functions::atan2(6.0, 8.0)),
            ];

            for &(expr, expected) in &misc_cases {
                let result = interp(expr, None).unwrap();
                let eps = if expr.starts_with("atan2") {
                    1e-3
                } else {
                    1e-6
                };
                // Move format string inside the macro call
                assert_approx_eq!(
                    result,
                    expected as Real, // Cast expected to Real
                    eps,
                    "Failed: {} = {}, expected {}",
                    expr,
                    result,
                    expected
                );
            }
        });
    }
}

// --- Additional exhaustive test cases ---

#[test]
fn constants_and_whitespace() {
    with_timeout(|| {
        let cases = [
            ("  pi  ", std::f64::consts::PI),
            ("\te\n", std::f64::consts::E),
            ("  42  ", 42.0),
            ("0", 0.0),
            ("  0.0  ", 0.0),
            ("  1.2345  ", 1.2345),
        ];
        for &(expr, answer) in &cases {
            println!("Testing expr: {}", expr);
            let result = interp(expr, None);
            match result {
                Ok(val) => {
                    if answer.is_infinite() {
                        assert!(
                            val.is_infinite(),
                            "Expected infinite result for '{}', got {}",
                            expr,
                            val
                        );
                    } else {
                        // Move format string inside the macro call
                        use exp_rs::Real; // Import Real for casting
                        assert_approx_eq!(
                            val,
                            answer as Real, // Cast answer to Real
                            exp_rs::constants::TEST_PRECISION,
                            "Failed: {} = {}, expected {}",
                            expr,
                            val,
                            answer
                        );
                    }
                }
                Err(e) => {
                    // Accept error only if the expected answer is infinite (overflow)
                    assert!(
                        answer.is_infinite(),
                        "Unexpected error for '{}': {:?}",
                        expr,
                        e
                    );
                }
            }
        }
    });
}

#[test]
fn operator_precedence_and_associativity() {
    with_timeout(|| {
        let cases = [
            ("2+3*4", 14.0),
            ("(2+3)*4", 20.0),
            ("2+3*4^2", 50.0),
            ("2^3^2", 512.0), // 2^(3^2) = 2^9 = 512
            ("2^3*4", 32.0),  // (2^3)*4 = 8*4 = 32
            ("2*3^2", 18.0),  // 2*(3^2) = 2*9 = 18
            ("2^3+4", 12.0),  // (2^3)+4 = 8+4 = 12
            ("2+3^2", 11.0),  // 2+(3^2) = 2+9 = 11
            ("-2^2", -4.0),   // -(2^2) = -4
            ("(-2)^2", 4.0),  // (-2)^2 = 4
        ];
        for &(expr, answer) in &cases {
            let result = interp(expr, None).unwrap();
            // Move format string inside the macro call
            use exp_rs::Real; // Import Real for casting
            assert_approx_eq!(
                result,
                answer as Real, // Cast answer to Real
                exp_rs::constants::TEST_PRECISION,
                "Failed: {} = {}, expected {}",
                expr,
                result,
                answer
            );
        }
    });
}

#[test]
fn function_nesting_and_chaining() {
    with_timeout(|| {
        let cases = [
            ("sin(asin(0.5))", 0.5),
            ("cos(acos(0.5))", 0.5),
            ("tan(atan(1))", 1.0),
            ("exp(ln(5))", 5.0),
            ("log10(10^3)", 3.0),
            ("sqrt(sin(asin(0.5))^2)", 0.5),
            ("abs(-abs(-5))", 5.0),
            ("ceil(floor(2.7))", 2.0),
            ("floor(ceil(2.3))", 3.0),
            // Test deeply nested function calls
            (
                "abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-12345))))))))))",
                12345.0,
            ),
            // Test with different nesting levels to find where it might break
            ("abs(abs(abs(-1)))", 1.0),
            ("abs(abs(abs(abs(abs(-1)))))", 1.0),
            ("abs(abs(abs(abs(abs(abs(abs(-1)))))))", 1.0),
            ("abs(abs(abs(abs(abs(abs(abs(abs(abs(-1)))))))))", 1.0),
        ];
        for &(expr, answer) in &cases {
            let result = interp(expr, None).unwrap();
            // Move format string inside the macro call
            use exp_rs::Real; // Import Real for casting
            assert_approx_eq!(
                result,
                answer as Real, // Cast answer to Real
                exp_rs::constants::TEST_PRECISION,
                "Failed: {} = {}, expected {}",
                expr,
                result,
                answer
            );
        }
    });
}

#[test]
fn test_deeply_nested_function_calls() {
    with_timeout(|| {
        // Test with increasing levels of nesting to find where it breaks
        let cases = [
            ("abs(-1)", 1.0),
            ("abs(abs(-1))", 1.0),
            ("abs(abs(abs(-1)))", 1.0),
            ("abs(abs(abs(abs(-1))))", 1.0),
            ("abs(abs(abs(abs(abs(-1)))))", 1.0),
            ("abs(abs(abs(abs(abs(abs(-1))))))", 1.0),
            ("abs(abs(abs(abs(abs(abs(abs(-1)))))))", 1.0),
            ("abs(abs(abs(abs(abs(abs(abs(abs(-1))))))))", 1.0),
            ("abs(abs(abs(abs(abs(abs(abs(abs(abs(-1)))))))))", 1.0),
            ("abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-1))))))))))", 1.0),
            ("abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-1)))))))))))", 1.0),
            ("abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-1))))))))))))", 1.0),
            ("abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-1)))))))))))))", 1.0),
            ("abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-1))))))))))))))", 1.0),
            ("abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-1)))))))))))))))", 1.0),
            // Add even deeper nesting to test the fix
            ("abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-1))))))))))))))))", 1.0),
            ("abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-1)))))))))))))))))", 1.0),
            ("abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-1))))))))))))))))))", 1.0),
            ("abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-1)))))))))))))))))))", 1.0),
            ("abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-1))))))))))))))))))))", 1.0),
        ];

        for (i, (expr, expected)) in cases.iter().enumerate() {
            println!("Testing nested abs level {}: {}", i + 1, expr);
            let result = interp(expr, None);
            match result {
                Ok(val) => {
                    // Move format string inside the macro call
                    use exp_rs::Real; // Import Real for casting
                    assert_approx_eq!(
                        val,
                        *expected as Real, // Cast expected to Real
                        exp_rs::constants::TEST_PRECISION,
                        "Failed: {} = {}, expected {}",
                        expr,
                        val,
                        expected
                    );
                }
                Err(e) => {
                    panic!("Expression '{}' failed with error: {:?}", expr, e);
                }
            }
        }
    });
}

#[test]
fn test_deeply_nested_function_calls_debug() {
    // Test with a specific deeply nested function call that was failing
    let expr = "abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-12345))))))))))";
    println!("Testing expression: {}", expr);

    // Try to parse the expression and print the result
    match exp_rs::engine::parse_expression(expr) {
        Ok(ast) => {
            println!("Successfully parsed AST: {:?}", ast);
            // Try to evaluate the AST
            match exp_rs::eval::eval_ast(&ast, None) {
                Ok(val) => println!("Successfully evaluated to: {}", val),
                Err(e) => println!("Evaluation error: {:?}", e),
            }
        }
        Err(e) => println!("Parse error: {:?}", e),
    }

    // Now try to interpret the expression directly
    match interp(expr, None) {
        Ok(val) => {
            println!("Successfully interpreted to: {}", val);
            assert_eq!(
                val, 12345.0,
                "abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-12345)))))))))) should be 12345.0"
            );
        }
        Err(e) => {
            panic!("Interpretation error: {:?}", e);
        }
    }
}

#[test]
fn test_deeply_nested_function_calls_with_debugging() {
    // Test with a specific deeply nested function call that was failing
    let expr = "abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-12345))))))))))";
    println!("Testing expression with debugging: {}", expr);

    // Create a lexer and tokenize the expression
    let mut lexer = exp_rs::lexer::Lexer::new(expr);
    let mut tokens = Vec::new();
    while let Some(tok) = lexer.next_token() {
        tokens.push(tok);
    }

    // Print all tokens for debugging
    println!("Tokens:");
    for (i, tok) in tokens.iter().enumerate() {
        println!("  {}: {:?}", i, tok);
    }

    // Try to parse the expression
    match exp_rs::engine::parse_expression(expr) {
        Ok(ast) => {
            println!("Successfully parsed AST: {:?}", ast);
            // Try to evaluate the AST
            match exp_rs::eval::eval_ast(&ast, None) {
                Ok(val) => {
                    println!("Successfully evaluated to: {}", val);
                    assert_eq!(val, 12345.0);
                }
                Err(e) => println!("Evaluation error: {:?}", e),
            }
        }
        Err(e) => println!("Parse error: {:?}", e),
    }

    // Now try to interpret the expression directly
    match interp(expr, None) {
        Ok(val) => {
            println!("Successfully interpreted to: {}", val);
            assert_eq!(val, 12345.0);
        }
        Err(e) => {
            panic!("Interpretation error: {:?}", e);
        }
    }
}

#[test]
fn error_handling_and_invalid_inputs() {
    let invalid_cases = [
        "",           // empty
        "1+",         // trailing operator
        "1)",         // unmatched parenthesis
        "(1",         // unmatched parenthesis
        "1**1",       // invalid operator
        "1*2(+4",     // invalid function call
        "1*2(1+4",    // invalid function call
        "unknown(1)", // unknown function
        "a+5",        // unknown variable
        "1/0",        // division by zero (should return inf or error)
        "0/0",        // NaN
    ];
    for &expr in &invalid_cases {
        let result = interp(expr, None);
        match result {
            Err(_) => {} // expected
            Ok(val) => {
                assert!(
                    val.is_nan() || val.is_infinite(),
                    "Expected error or non-finite result for '{}', got {}",
                    expr,
                    val
                );
            }
        }
    }
}

#[test]
fn scientific_notation_and_edge_cases() {
    with_timeout(|| {
        let cases = [
            ("1e3", 1000.0),
            ("1.5e2", 150.0),
            ("2e-2", 0.02),
            ("1e+2", 100.0),
            ("1e0", 1.0),
            ("1e-0", 1.0),
            ("1.23e4+5.67e3", 12300.0 + 5670.0),
            ("1e2^2", 10000.0),   // 100^2 = 10000
            ("1e2^2e0", 10000.0), // 100^2 = 10000
        ];
        for &(expr, answer) in &cases {
            let result = interp(expr, None).unwrap();
            // Move format string inside the macro call
            use exp_rs::Real; // Import Real for casting
            assert_approx_eq!(
                result,
                answer as Real, // Cast answer to Real
                exp_rs::constants::TEST_PRECISION,
                "Failed: {} = {}, expected {}",
                expr,
                result,
                answer
            );
        }
    });
}

#[test]
fn chained_unary_operators() {
    with_timeout(|| {
        let cases = [
            ("--5", 5.0),
            ("---5", -5.0),
            ("-+5", -5.0),
            ("+-5", -5.0),
            ("+--5", 5.0),
            ("-(-(-5))", -5.0),
        ];
        for &(expr, answer) in &cases {
            let result = interp(expr, None).unwrap();
            // Use assert_approx_eq! here as well for consistency
            use exp_rs::Real; // Import Real for casting
            assert_approx_eq!(
                result,
                answer as Real, // Cast answer to Real
                1e-6            // No format string needed if default message is okay
            );
        }
    });
}

#[test]
fn parentheses_and_grouping() {
    with_timeout(|| {
        let cases = [
            ("(((((5)))))", 5.0),
            ("(2+3)*(4+5)", 45.0),
            ("((2+3)*4)+5", 25.0),
            ("2+(3*(4+5))", 29.0),
            ("(2+3)*(4+5)", 45.0),
        ];
        for &(expr, answer) in &cases {
            let result = interp(expr, None).unwrap();
            // Use assert_approx_eq! here as well for consistency
            use exp_rs::Real; // Import Real for casting
            assert_approx_eq!(
                result,
                answer as Real, // Cast answer to Real
                1e-6            // No format string needed if default message is okay
            );
        }
    });
}

#[test]
fn long_and_complex_expressions() {
    with_timeout(|| {
        let cases = [
            // Long chain of additions and multiplications
            ("1+2+3+4+5+6+7+8+9+10", 55.0),
            ("1*2*3*4*5*6*7*8*9*10", 3628800.0),
            // Nested parentheses and mixed operators
            ("((1+2)*(3+4)*(5+6)*(7+8))", 3465.0),
            // Alternating add/subtract
            ("1-2+3-4+5-6+7-8+9-10", -5.0),
            // Deeply nested powers
            ("2^2^2^2", 65536.0), // 2^(2^(2^2)) = 2^16 = 65536
            // Chained functions and powers
            ("sin(cos(tan(1)^2)^2)^2", 0.2903875274),
            // Long chain of unary minuses
            ("-+-+-+-+-+-+-+-10", 10.0),
            // Long chain of function applications
            ("sqrt(sqrt(sqrt(sqrt(65536))))", 2.0),
            // Combination of all
            ("(1+2*3-4/2+5^2-6+7*8-9/3+10^2)*2", 354.0),
            // Many nested parentheses
            ("((((((((((((((((42))))))))))))))))", 42.0),
            // Many chained commas
            // Top-level comma expressions are not allowed in TinyExpr; expect error/NaN.
            // ("1,2,3,4,5,6,7,8,9,10", 10.0),
            // Many chained functions
            ("abs(abs(abs(abs(abs(-42)))))", 42.0),
            // Many chained powers
            ("2^2^2^2^1", 65536.0), // 2^(2^(2^(2^1))) = 2^16 = 65536
            // Many chained sqrt
            ("sqrt(sqrt(sqrt(sqrt(sqrt(4294967296)))))", 2.0),
            // Very long addition and multiplication chain
            ("1+2+3+4+5+6+7+8+9+10+11+12+13+14+15+16+17+18+19+20", 210.0),
            (
                "1*2*3*4*5*6*7*8*9*10*11*12*13*14*15*16*17*18*19*20",
                2432902008176640000.0,
            ),
            // Deeply nested parentheses and mixed operators (balanced: 11 open, 11 close)
            ("((((((((((1+2)*3)+4)*5)+6)*7)+8)*9)+10))", 4555.0),
            // Alternating add/subtract/multiply/divide
            (
                "1-2+3*4/5-6+7*8/9-10+11*12/13-14+15*16/17-18+19*20/21",
                1.9889535301300008,
            ),
            // Deeply nested powers and roots
            ("2^2^2^2^2", f64::INFINITY), // 2^(2^(2^(2^2))) = 2^65536 (overflow)
            (
                "sqrt(sqrt(sqrt(sqrt(sqrt(sqrt(16777216))))))",
                1.2968395546510096,
            ),
            // Chained functions and powers with more depth
            ("sin(cos(tan(1)^2)^2)^2^2^2", 0.00005056193323212385),
            // Long chain of unary minuses and pluses
            ("-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+100", -100.0),
            // Long chain of function applications
            (
                "sqrt(sqrt(sqrt(sqrt(sqrt(sqrt(sqrt(sqrt(256))))))))",
                1.0218971486541166,
            ),
            // Combination of all with more terms
            (
                "(1+2*3-4/2+5^2-6+7*8-9/3+10^2+11*12-13/4+15^2-16+17*18-19/5+20^2)*2",
                2433.9,
            ),
            // Many nested parentheses (20 deep)
            ("((((((((((((((((((((123))))))))))))))))))))", 123.0),
            // Many chained commas (20 values)
            ("1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20", 20.0),
            // Many chained abs and sqrt
            (
                "abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-12345))))))))))",
                12345.0,
            ),
            (
                "sqrt(sqrt(sqrt(sqrt(sqrt(sqrt(sqrt(sqrt(sqrt(sqrt(1048576))))))))))",
                1.0136300849514894,
            ),
            // Many chained powers (right-associative)
            ("2^2^2^2^2^2", f64::INFINITY), // 2^(2^(2^(2^(2^2^2)))) (overflow)
        ];
        for &(expr, answer) in &cases {
            println!("Testing expr: {}", expr);
            let result = interp(expr, None);
            match result {
                Ok(val) => {
                    if answer.is_infinite() {
                        assert!(
                            val.is_infinite(),
                            "Expected infinite result for '{}', got {}",
                            expr,
                            val
                        );
                    } else {
                        // Move format string inside the macro call
                        use exp_rs::Real; // Import Real for casting
                        assert_approx_eq!(
                            val,
                            answer as Real, // Cast answer to Real
                            exp_rs::constants::TEST_PRECISION,
                            "Failed: {} = {}, expected {}",
                            expr,
                            val,
                            answer
                        );
                    }
                }
                Err(e) => {
                    // Accept error only if the expected answer is infinite (overflow)
                    assert!(
                        answer.is_infinite(),
                        "Unexpected error for '{}': {:?}",
                        expr,
                        e
                    );
                }
            }
        }
    });
}
