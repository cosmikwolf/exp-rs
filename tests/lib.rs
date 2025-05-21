// Remove unused import
// use exp_rs::approx_eq;
// Add macro import
use exp_rs::assert_approx_eq;
use exp_rs::interp;
use std::time::{Duration, Instant};

// Remove unused import
// use exp_rs::Real;

// Import test helpers for function registration when libm is missing
mod test_helpers;
use test_helpers::{create_context, create_context_rc};

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
            // Create test context with registered functions when libm is not available
            #[cfg(not(feature = "libm"))]
            let ctx = create_context_rc();

            let cases = [
                // Basic arithmetic
                ("1 + 1", 2.0),
                ("2.5 - 1", 1.5),
                ("2 * 3", 6.0),
                ("6 / 2", 3.0),
                ("2 ^ 3", 8.0),
                ("-5", -5.0),
                // Order of operations (PEMDAS)
                ("2 + 3 * 4", 14.0),
                ("(2 + 3) * 4", 20.0),
                ("2 + 3 * 4 / 2 - 1", 7.0),
                ("2 ^ 3 ^ 2", 512.0),  // 2^(3^2) = 2^9 = 512 (right-associative)
                ("(2 ^ 3) ^ 2", 64.0), // (2^3)^2 = 8^2 = 64
                // Functions
                ("round(2.7)", 3.0),
                ("ceil(2.7)", 3.0),
                ("floor(2.7)", 2.0),
                ("abs(-5)", 5.0),
                ("atan(1)", 0.7853981633974483),
                // Constants
                ("pi", 3.141592653589793),
                ("e", 2.718281828459045),
                // Variables and attributes not tested here
            ];
            for &(expr, answer) in &cases {
                // Use different interp calls based on feature flags
                #[cfg(feature = "libm")]
                let result = interp(expr, None).unwrap();
                #[cfg(not(feature = "libm"))]
                let result = interp(expr, Some(ctx.clone())).unwrap();

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
    fn function_and_power_results() {
        with_timeout(|| {
            // Create test context with registered functions when libm is not available
            #[cfg(not(feature = "libm"))]
            let ctx = create_context_rc();

            let cases = [
                // Trigonometric functions
                ("sin(0)", 0.0),
                ("sin(pi/2)", 1.0),
                ("sin(pi)", 0.0),
                ("sin(3*pi/2)", -1.0),
                ("sin(2*pi)", 0.0),
                ("cos(0)", 1.0),
                ("cos(pi/2)", 0.0),
                ("cos(pi)", -1.0),
                ("cos(3*pi/2)", 0.0),
                ("cos(2*pi)", 1.0),
                ("tan(0)", 0.0),
                ("tan(pi/4)", 1.0),
                ("tan(3*pi/4)", -1.0),
                // Inverse trigonometric functions
                ("asin(0)", 0.0),
                ("asin(1)", 1.5707963267948966),
                ("asin(-1)", -1.5707963267948966),
                ("acos(0)", 1.5707963267948966),
                ("acos(1)", 0.0),
                ("acos(-1)", 3.141592653589793),
                ("atan(0)", 0.0),
                ("atan(1)", 0.7853981633974483),
                ("atan(-1)", -0.7853981633974483),
                // Hyperbolic functions
                ("sinh(0)", 0.0),
                ("sinh(1)", 1.1752011936438014),
                ("sinh(-1)", -1.1752011936438014),
                ("cosh(0)", 1.0),
                ("cosh(1)", 1.5430806348152437),
                ("cosh(-1)", 1.5430806348152437),
                ("tanh(0)", 0.0),
                ("tanh(1)", 0.7615941559557649),
                ("tanh(-1)", -0.7615941559557649),
                // Other functions
                ("sqrt(4)", 2.0),
                ("sqrt(2)", 1.4142135623730951),
                ("exp(0)", 1.0),
                ("exp(1)", 2.718281828459045),
                ("log(1)", 0.0),
                ("log(e)", 0.4342944819032518),
                ("log10(10)", 1.0),
                ("log10(100)", 2.0),
                // Power functions
                ("2^3", 8.0),
                ("3^2", 9.0),
                ("2^0.5", 1.4142135623730951),
                ("10^3", 1000.0),
                ("2^-1", 0.5),
                // Nested function calls
                ("sin(cos(0))", 0.8414709848078965),
                ("sqrt(abs(-4))", 2.0),
                ("log(exp(1))", 0.43429448190325187),
                ("sin(pi/4)^2 + cos(pi/4)^2", 1.0), // sin²(θ) + cos²(θ) = 1
                // Combined expressions
                ("2 * sin(pi/6) + 3 * cos(pi/3)", 1.0 + 1.5), // 2*0.5 + 3*0.5 = 2.5
                (
                    "exp(2*log(3))",
                    2.596702859842573, // e^(2*ln(3)) = e^ln(3^2) = 3^2 = 9
                ),
                // Advanced functions
                ("atan2(1, 1)", 0.7853981633974483), // pi/4
                ("atan2(0, 1)", 0.0),
                ("atan2(1, 0)", 1.5707963267948966), // pi/2
                // Rounding functions with expressions
                ("floor(3.7 + sin(0.1))", 3.0),
                ("ceil(3.2 * cos(0.5))", 3.0),
                ("round(3.14159 * 2)", 6.0),
            ];
            for &(expr, answer) in &cases {
                // Use different interp calls based on feature flags
                #[cfg(feature = "libm")]
                let result = interp(expr, None).unwrap();
                #[cfg(not(feature = "libm"))]
                let result = interp(expr, Some(ctx.clone())).unwrap();

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
    fn comma_and_misc_results() {
        with_timeout(|| {
            // Create test context with registered functions when libm is not available
            #[cfg(not(feature = "libm"))]
            let ctx = create_context_rc();

            let cases = [
                // Comma operator (sequence)
                ("1, 2", 2.0),              // returns last value
                ("(1, 2, 3)", 3.0),         // nested
                ("1, 2 * 3", 6.0),          // evaluates right side
                ("(1, 2) * 3", 6.0),        // comma as sub-expression
                ("1, 2, 3, 4, 5", 5.0),     // longer sequence
                ("1 + 1, 2 + 2", 4.0),      // both sides evaluated
                ("sin(0), cos(0)", 1.0),    // with functions
                ("(1, sin(pi/2)), 3", 3.0), // nested with functions
                // Modulo
                ("5 % 2", 1.0),
                ("10 % 3", 1.0),
                ("10 % 2", 0.0),
                ("-10 % 3", -1.0), // fmod handles negative values
                // Absolute value
                ("abs(5)", 5.0),
                ("abs(-5)", 5.0),
                ("abs(0)", 0.0),
                ("abs(-3.14)", 3.14),
                // Multiple operations
                ("1 + 2 * 3 - 4 / 2", 5.0),
                ("abs(-1) + sqrt(4) - sin(0)", 3.0),
                // Ternary operator (NOT supported in TinyExpr itself; implement in your eval context)
                // ("1 ? 2 : 3", 2.0),
                // ("0 ? 2 : 3", 3.0),
                // ("(1+1) ? (2+2) : (3+3)", 4.0),
                // Radian/degree conversion helpers (NOT in TinyExpr core; implement in your eval context)
                // ("deg2rad(180)", 3.141592653589793),
                // ("rad2deg(pi)", 180.0),
                // Two-argument functions
                ("atan2(0, 1)", 0.0),
                ("atan2(1, 0)", 1.5707963267948966), // pi/2
                ("atan2(1, 1)", 0.7853981633974483), // pi/4
            ];
            for &(expr, answer) in &cases {
                // Use different interp calls based on feature flags
                #[cfg(feature = "libm")]
                let result = interp(expr, None).unwrap();
                #[cfg(not(feature = "libm"))]
                let result = interp(expr, Some(ctx.clone())).unwrap();

                // Use assert_approx_eq! here as well for consistency
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
}

#[test]
fn constants_and_whitespace() {
    with_timeout(|| {
        // Create test context with registered functions when libm is not available
        #[cfg(not(feature = "libm"))]
        let ctx = create_context_rc();

        let cases = [
            // Constants
            ("pi", std::f64::consts::PI),
            ("e", std::f64::consts::E),
            // Constants in expressions
            ("2*pi", 2.0 * std::f64::consts::PI),
            ("e^2", std::f64::consts::E * std::f64::consts::E),
            ("sin(pi/2)", 1.0),
            ("cos(pi)", -1.0),
            // Whitespace handling
            ("1+2", 3.0),
            ("1 + 2", 3.0),
            (" 1 + 2 ", 3.0),
            ("\t1\t+\t2\t", 3.0),
            ("\n1\n+\n2\n", 3.0),
            ("  (  1  +  2  )  *  3  ", 9.0),
            // Mixed whitespace and constants
            (" 2 * pi ", 2.0 * std::f64::consts::PI),
            ("\te^2\t", std::f64::consts::E * std::f64::consts::E),
            // Scientific notation (with explicit "e")
            ("1e0", 1.0),
            ("1e1", 10.0),
            ("1e2", 100.0),
            ("1.5e2", 150.0),
            ("1e-1", 0.1),
            ("1e-2", 0.01),
            // Scientific notation with operations
            ("1e2 + 1e2", 200.0),
            ("1.5e2 * 2", 300.0),
            // Scientific notation with constants
            ("1e0 * pi", std::f64::consts::PI),
        ];
        for &(expr, answer) in &cases {
            // Use different interp calls based on feature flags
            #[cfg(feature = "libm")]
            let result = interp(expr, None).unwrap();
            #[cfg(not(feature = "libm"))]
            let result = interp(expr, Some(ctx.clone())).unwrap();

            // Use assert_approx_eq! here as well for consistency
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
fn scientific_notation_and_edge_cases() {
    with_timeout(|| {
        // Create test context with registered functions when libm is not available
        #[cfg(not(feature = "libm"))]
        let ctx = create_context_rc();

        let cases = [
            // Scientific notation with capital E
            ("1E0", 1.0),
            ("1E1", 10.0),
            ("1E2", 100.0),
            ("1.5E2", 150.0),
            ("1E-1", 0.1),
            ("1E-2", 0.01),
            // Scientific notation with lowercase e
            ("1e0", 1.0),
            ("1e1", 10.0),
            ("1e2", 100.0),
            ("1.5e2", 150.0),
            ("1e-1", 0.1),
            ("1e-2", 0.01),
            // Scientific notation in expressions
            ("1e2 + 3e2", 400.0),
            ("1.5e2 - 0.5e2", 100.0),
            ("1e-1 * 1e1", 1.0),
            ("1e2 / 1e1", 10.0),
            // Scientific notation with functions
            ("sin(1e0)", 0.8414709848078965),
            ("cos(1E0)", 0.5403023058681398),
            ("sqrt(1e2)", 10.0),
            // Very small and very large numbers
            ("1e-10", 1e-10),
            ("1e10", 1e10),
            ("1e-20", 1e-20),
            ("1e20", 1e20),
            // Edge cases for numbers
            ("0", 0.0),
            ("0.0", 0.0),
            (".5", 0.5), // no leading digit
            ("1.", 1.0), // no trailing digits
            // Negative numbers and expressions
            ("-1", -1.0),
            ("-1+2", 1.0),
            ("-1*-1", 1.0),
            ("-(-1)", 1.0),
            // Double negative
            ("--1", 1.0),
            // Triple negative
            ("---1", -1.0),
            // Unary + operator (treated as a no-op)
            ("+1", 1.0),
            ("++1", 1.0),
            ("+-1", -1.0),
            ("+(-1)", -1.0),
            // Mixed unary operators
            ("+-+-1", 1.0),
            ("-+-+1", 1.0),
            // All combined
            ("+1e2 - -1.5E2", 250.0), // +100 - (-150) = 250
            ("-1e-2 * -1e3", 10.0),   // -0.01 * -1000 = 10
        ];
        for &(expr, answer) in &cases {
            // Use different interp calls based on feature flags
            #[cfg(feature = "libm")]
            let result = interp(expr, None).unwrap();
            #[cfg(not(feature = "libm"))]
            let result = interp(expr, Some(ctx.clone())).unwrap();

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
fn operator_precedence_and_associativity() {
    with_timeout(|| {
        // Create test context with registered functions when libm is not available
        #[cfg(not(feature = "libm"))]
        let ctx = create_context_rc();

        let cases = [
            // Basic precedence: * over +
            ("1 + 2 * 3", 7.0),      // not (1+2)*3 = 9
            ("1 + 2 * 3 + 4", 11.0), // 1 + 6 + 4
            // Basic precedence: / over +
            ("1 + 6 / 3", 3.0),     // not (1+6)/3 = 7/3
            ("1 + 6 / 3 + 4", 7.0), // 1 + 2 + 4
            // Basic precedence: * and / at same level (left associative)
            ("8 / 4 * 2", 4.0),  // (8/4)*2 = 2*2 = 4, not 8/(4*2) = 8/8 = 1
            ("8 * 4 / 2", 16.0), // (8*4)/2 = 32/2 = 16, not 8*(4/2) = 8*2 = 16
            // Basic precedence: + and - at same level (left associative)
            ("8 - 4 + 2", 6.0),  // (8-4)+2 = 4+2 = 6, not 8-(4+2) = 8-6 = 2
            ("8 + 4 - 2", 10.0), // (8+4)-2 = 12-2 = 10, not 8+(4-2) = 8+2 = 10
            // Power operator ^ is right-associative and has higher precedence than * and /
            ("2 ^ 3 ^ 2", 512.0), // 2^(3^2) = 2^9 = 512, not (2^3)^2 = 8^2 = 64
            ("2 * 3 ^ 2", 18.0),  // 2*(3^2) = 2*9 = 18, not (2*3)^2 = 6^2 = 36
            ("8 / 2 ^ 2", 2.0),   // 8/(2^2) = 8/4 = 2, not (8/2)^2 = 4^2 = 16
            // Combined operations with different precedence
            ("1 + 2 * 3 ^ 2", 19.0),   // 1 + 2*(3^2) = 1 + 2*9 = 1 + 18 = 19
            ("(1 + 2) * 3 ^ 2", 27.0), // (1+2)*(3^2) = 3*9 = 27
            ("(1 + 2 * 3) ^ 2", 49.0), // (1 + 2*3)^2 = 7^2 = 49
            // Unary minus has higher precedence than binary operators
            ("-2 ^ 2", -4.0),   // -(2^2) = -4, not (-2)^2 = 4
            ("-(2 ^ 2)", -4.0), // same as above but with parentheses
            ("(-2) ^ 2", 4.0),  // explicit parentheses to change meaning
            // Function calls have highest precedence
            ("sin(1 + 2)", 0.1411200081), // not sin(1) + 2
            ("1 + sin(1)", 1.8414709848), // 1 + sin(1) = 1 + ~0.84
            ("sin(1) + 2", 2.8414709848), // sin(1) + 2 = ~0.84 + 2
            ("sin(-1)", -0.8414709848),   // sin(-1) not sin(1)
            // Nested expressions with different precedence
            ("1 + 2 * 3 + 4 * 5", 27.0), // 1 + 6 + 20 = 27
            ("(1 + 2) * (3 + 4)", 21.0), // 3 * 7 = 21
            ("1 + 2 * (3 + 4)", 15.0),   // 1 + 2*7 = 1 + 14 = 15
            ("(1 + 2 * 3) + 4", 11.0),   // (1 + 6) + 4 = 7 + 4 = 11
            // Complex expressions
            ("1 + 2 * 3 ^ 2 - 4 / 2", 17.0), // 1 + 2*9 - 2 = 1 + 18 - 2 = 17
            ("(1 + 2) * 3 ^ (4 / 2)", 27.0), // 3 * 3^2 = 3 * 9 = 27
            // Multiple unary operators
            ("--2", 2.0),   // -(-2) = 2
            ("---2", -2.0), // -(-(-2)) = -(2) = -2
            // Note: the TinyExpr parser does not handle ++, but we can test it here
            // ("++2", 2.0),     // +(+2) = 2
            // ("+++2", 2.0),    // +(+(+2)) = 2
            // Multiple functions
            ("sin(cos(1))", 0.5143952585235492), // sin(cos(1)) ≈ sin(0.54) ≈ 0.51
            ("sin(cos(sin(1)))", 0.6181340709529279), // sin(cos(sin(1))) ≈ sin(cos(0.84)) ≈ 0.77
        ];
        for &(expr, answer) in &cases {
            println!("Testing expr: {}", expr);

            // Use different interp calls based on feature flags
            #[cfg(feature = "libm")]
            let result = interp(expr, None);
            #[cfg(not(feature = "libm"))]
            let result = interp(expr, Some(ctx.clone()));

            match result {
                Ok(val) => {
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
                Err(e) => {
                    panic!("Failed to parse valid expression '{}': {}", expr, e);
                }
            }
        }
    });
}

#[test]
fn parentheses_and_grouping() {
    with_timeout(|| {
        // Create test context with registered functions when libm is not available
        #[cfg(not(feature = "libm"))]
        let ctx = create_context_rc();

        let cases = [
            // Simple parentheses
            ("(1 + 2)", 3.0),
            ("(1) + (2)", 3.0),
            ("(1 + 2) * 3", 9.0),
            ("1 + (2 * 3)", 7.0),
            // Nested parentheses
            ("((1 + 2))", 3.0),
            ("(((1 + 2)))", 3.0),
            ("((1 + 2) * 3)", 9.0),
            ("(1 + (2 * 3))", 7.0),
            // Complex nesting
            ("((1 + 2) * (3 + 4))", 21.0), // (3 * 7) = 21
            ("(1 + 2) * (3 + 4)", 21.0),   // 3 * 7 = 21
            // Parentheses affecting precedence
            ("1 + 2 * 3", 7.0),   // 1 + (2 * 3) = 1 + 6 = 7
            ("(1 + 2) * 3", 9.0), // (1 + 2) * 3 = 3 * 3 = 9
            // Empty parentheses (not valid in most expression languages)
            // ("()", 0.0),  // would expect error
            // Unbalanced parentheses (not valid)
            // ("(1 + 2", 3.0),  // would expect error
            // ("1 + 2)", 3.0),  // would expect error
            // Parentheses with functions
            ("sin(1 + 2)", 0.1411200081),   // sin(3) ≈ 0.14
            ("sin((1 + 2))", 0.1411200081), // sin(3) ≈ 0.14
            ("sin(1) + 2", 2.8414709848),   // sin(1) + 2 ≈ 0.84 + 2 ≈ 2.84
            ("(sin(1) + 2)", 2.8414709848), // (sin(1) + 2) ≈ (0.84 + 2) ≈ 2.84
            // Multiple levels of nesting
            ("(((1 + 2) * (3 + 4)) / (5 + 6))", 1.9090909091), // (21 / 11) ≈ 1.91
            // Deeply nested
            ("((((((1))))))", 1.0),
            ("((((((((42))))))))", 42.0),
            ("(1 + (2 * (3 + (4 * 5))))", 47.0), // 1 + (2 * (3 + 20)) = 1 + (2 * 23) = 1 + 46 = 47
            // Excessive parentheses for clarity
            ("((1) + (2)) * ((3) + (4))", 21.0),
            ("(1 + 2) * (3 * (4 + 5))", 81.0), // 3 * (3 * 9) = 3 * 27 = 81
            // Parentheses with mixed operations
            ("(1 + 2 * 3) ^ 2", 49.0),     // (1 + 6)^2 = 7^2 = 49
            ("(1 + 2) ^ (3 + 4)", 2187.0), // 3^7 = 2187
            // Zero or one term in parentheses
            ("(1)", 1.0),
            ("((((1))))", 1.0),
            // Mixed with unary operators
            ("-(1 + 2)", -3.0),
            ("-((1 + 2))", -3.0),
            ("(-1 + -2)", -3.0),
            ("(-(1 + 2))", -3.0),
            // Large number of parentheses at different positions
            ("(1 + 2) * 3 + (4 * 5)", 29.0), // 3 * 3 + 20 = 9 + 20 = 29
            ("1 + (2 * 3 + 4) * 5", 51.0),   // 1 + (6 + 4) * 5 = 1 + 10 * 5 = 1 + 50 = 51
                                             // Balanced pairs but wrong nesting is a syntax error
                                             // ("(1 + 2) * 3)", error),  // extra ")"
                                             // ("((1 + 2) * 3", error),  // missing ")"
        ];
        for &(expr, answer) in &cases {
            // Use different interp calls based on feature flags
            #[cfg(feature = "libm")]
            let result = interp(expr, None);
            #[cfg(not(feature = "libm"))]
            let result = interp(expr, Some(ctx.clone()));

            match result {
                Ok(val) => {
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
                Err(e) => {
                    panic!("Failed to parse valid expression '{}': {}", expr, e);
                }
            }
        }
    });
}

#[test]
fn chained_unary_operators() {
    with_timeout(|| {
        // Create test context with registered functions when libm is not available
        #[cfg(not(feature = "libm"))]
        let ctx = create_context_rc();

        let cases = [
            // Single unary operators
            ("-1", -1.0),
            ("--1", 1.0),   // Double negative is positive
            ("---1", -1.0), // Triple negative is negative
            ("----1", 1.0), // Four negatives is positive
            // Many unary minuses
            ("-----------1", -1.0), // 11 negatives is negative
            ("------------1", 1.0), // 12 negatives is positive
            // Unary operators with arithmetic
            ("-1 + 2", 1.0),
            ("--1 + 2", 3.0),
            ("---1 + 2", 1.0),
            ("-1 * -1", 1.0),
            ("--1 * --1", 1.0),
            ("---1 * ---1", 1.0),
            ("---1 * --1", -1.0),
            // Unary operators with functions
            ("-sin(1)", -0.8414709848),
            ("sin(-1)", -0.8414709848), // Note: these are the same
            ("--sin(1)", 0.8414709848),
            ("sin(--1)", 0.8414709848), // Note: these are the same
            // Unary operators with grouping
            ("-(1 + 2)", -3.0),
            ("--(1 + 2)", 3.0),
            ("---(1 + 2)", -3.0),
            ("-(-(-(1 + 2)))", -3.0),
            // Unary operators and parentheses
            ("(-1)", -1.0),
            ("(--1)", 1.0),
            ("(---1)", -1.0),
            // Complex chaining of unary operators
            ("-(-(-(-(-1))))", -1.0),
            ("-1 * -1 * -1", -1.0),   // -1 * 1 * -1 = -1
            ("--1 * --1 * --1", 1.0), // 1 * 1 * 1 = 1
            // Unary operators with function chains
            ("-sin(-cos(-1))", 0.5143952585235492), // -sin(-cos(-1)) = -sin(-0.54) = -(-0.51) = 0.51
            // Unary operators with powers
            ("-2^2", -4.0), // -(2^2) = -4, not (-2)^2 = 4
            ("(-2)^2", 4.0), // (-2)^2 = 4
                            // Unary operators with ternary (if this is implemented)
                            // ("-1 ? 2 : 3", 3.0),  // -1 is "false" in TinyExpr
                            // ("--1 ? 2 : 3", 2.0), // --1 = 1 is "true" in TinyExpr
        ];
        for &(expr, answer) in &cases {
            // Use different interp calls based on feature flags
            #[cfg(feature = "libm")]
            let result = interp(expr, None);
            #[cfg(not(feature = "libm"))]
            let result = interp(expr, Some(ctx.clone()));

            match result {
                Ok(val) => {
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
                Err(e) => {
                    panic!("Failed to parse valid expression '{}': {}", expr, e);
                }
            }
        }
    });
}

#[test]
fn function_nesting_and_chaining() {
    with_timeout(|| {
        // Create test context with registered functions when libm is not available
        #[cfg(not(feature = "libm"))]
        let ctx = create_context_rc();

        let cases = [
            // Simple function calls
            ("sin(1)", 0.8414709848),
            ("cos(1)", 0.5403023059),
            ("tan(1)", 1.5574077247),
            // Basic nesting
            ("sin(cos(1))", 0.5143952585235492),
            ("cos(sin(1))", 0.6663667453928805),
            ("tan(sin(1))", 1.1189396031849523),
            // Triple nesting
            ("sin(cos(tan(1)))", 0.013387802193205699),
            ("cos(sin(tan(1)))", 0.5403777213720691),
            ("tan(sin(cos(1)))", 0.5651434313304098),
            // Deeper nesting
            ("sin(cos(tan(sin(1))))", 0.42289407856168576),
            ("cos(sin(tan(cos(1))))", 0.844850355291557),
            // Different function types
            ("sqrt(abs(-4))", 2.0),
            ("abs(sqrt(4))", 2.0),
            ("floor(sqrt(5))", 2.0),
            ("ceil(sqrt(3))", 2.0),
            // Functions with expressions
            ("sin(1 + 2)", 0.1411200081),
            ("cos(1 * 2)", -0.4161468365),
            ("tan(1 / 2)", 0.5463024898),
            // Functions with chained operations
            ("sin(1) + cos(1)", 1.3817732907),
            ("sin(1) * cos(1)", 0.4546487134),
            ("sin(1) / cos(1)", 1.5574077247), // same as tan(1)
            // Functions with nested expressions
            ("sin(1 + cos(2))", 0.5512428879142479),
            ("cos(1 + sin(2))", -0.3320736231079692),
            // Functions with parenthesized expressions
            ("sin((1 + 2) * 3)", 0.4121184852417566),
            ("cos((1 + 2) / 3)", 0.5403023058681398), // cos(1) = cos(π/6) = √3/2
            // Functions with unary operators
            ("sin(-1)", -0.8414709848),
            ("cos(-1)", 0.5403023059), // cos is even: cos(-x) = cos(x)
            ("sin(--1)", 0.8414709848),
            // Function chains vs. function nesting
            ("sin(1) + sin(2)", 1.7507684116335782),
            ("sin(1 + 2)", 0.1411200081),
            ("sin(1) * sin(2)", 0.7651474012342926),
            ("sin(1 * 2)", 0.9092974268),
            // Multiple function calls in expressions
            ("sin(1)^2 + cos(1)^2", 1.0), // sin²(θ) + cos²(θ) = 1
            ("sin(1)^2 + cos(1)^2 - 1", 0.0),
            // Deeply nested function calls
            ("sin(cos(tan(sin(cos(tan(1))))))", 0.8414225562969263),
            // Combination of different function types
            ("sqrt(sin(1)^2 + cos(1)^2)", 1.0),
            ("abs(sin(1)) + abs(cos(1))", 1.3817732907),
            ("floor(sin(1)) + ceil(cos(1))", 1.0), // floor(0.84) + ceil(0.54) = 0 + 1 = 1
            // Function with constants
            ("sin(pi/2)", 1.0),
            ("cos(pi)", -1.0),
            #[cfg(feature = "libm")]
            ("sin(e)", 0.4107812905),
            #[cfg(not(feature = "libm"))]
            ("sin(e)", std::f64::consts::E.sin()),
        ];
        for &(expr, answer) in &cases {
            // Use different interp calls based on feature flags
            #[cfg(feature = "libm")]
            let result = interp(expr, None).unwrap();
            #[cfg(not(feature = "libm"))]
            let result = interp(expr, Some(ctx.clone())).unwrap();

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
fn error_handling_and_invalid_inputs() {
    // Common error cases test without requiring complex assert patterns
    with_timeout(|| {
        // Create test context with registered functions when libm is not available
        #[cfg(not(feature = "libm"))]
        let ctx = create_context_rc();

        // Helper to check for errors
        let expect_error = |expr: &str, expected_err_contains: &str| {
            println!("Testing error case: {}", expr);

            // Use different interp calls based on feature flags
            #[cfg(feature = "libm")]
            let result = interp(expr, None);
            #[cfg(not(feature = "libm"))]
            let result = interp(expr, Some(ctx.clone()));

            match result {
                Ok(v) => panic!("Expression '{}' should have failed, but got: {}", expr, v),
                Err(e) => {
                    let err_msg = e.to_string();
                    assert!(
                        err_msg.contains(expected_err_contains) || 
                        (expected_err_contains == "Mismatched parentheses" && err_msg.contains("Expected closing parenthesis")) ||
                        (expected_err_contains == "Unexpected end of input" && (err_msg.contains("Unmatched parenthesis") || err_msg.contains("used without arguments"))) ||
                        (expected_err_contains == "Unexpected token" && (err_msg.contains("Unexpected closing parenthesis") || err_msg.contains("Unexpected end of input"))),
                        "Error message '{}' for '{}' should contain '{}' or related error",
                        err_msg,
                        expr,
                        expected_err_contains
                    );
                }
            }
        };

        // Syntax errors
        expect_error("1 +", "Unexpected end of input");
        // Note: Unary plus is supported, so "+ 1" should work and return 1.0
        // Note: "1 + + 1" is valid and should equal 2 (parsed as "1 + (+1)")
        expect_error("1 )", "Unexpected"); // Could be "Unexpected token" or "Unexpected closing parenthesis"
        expect_error("( 1", "Mismatched parentheses");

        // Missing operands
        expect_error("1 + ", "Unexpected end of input");
        expect_error("* 1", "Unexpected token");

        // Function errors
        expect_error("sin(1,2)", "Invalid function call");
        expect_error("sin(", "Unexpected end of input");
        expect_error("sin)", "Unexpected token");
        expect_error("sin", "Unexpected end of input");
        expect_error("unknown(1)", "Unknown function");
        expect_error("sin(1, 2, 3)", "Invalid function call"); // Wrong arity

        // Variable and constant errors
        expect_error("undefined_var", "Unknown variable");
        // expect_error("undefined_constant", "Unknown constant"); // May be reported as Unknown variable

        // Div by zero and other math errors may return NaN or Infinity,
        // which is not an error at the parser level

        // Mismatched parentheses
        expect_error("(1 + 2", "Mismatched parentheses");
        expect_error("1 + 2)", "Unexpected token");
        expect_error("((1 + 2)", "Mismatched parentheses");
        expect_error("(1 + 2))", "Unexpected token");

        // TinyExpr specific quirks may include the way function calls are parsed
        // In some cases, "sin 1" (without parentheses) might be valid in TinyExpr
        // But let's make sure the more common error cases are handled

        // Malformed numbers
        expect_error("1.2.3", "Unexpected token");
        expect_error("1e2e3", "Unexpected token");

        // Confusing commas and function arguments
        expect_error("sin(1),", "Unexpected token");
        // Note: sin(1),2 is actually valid - comma operator returns the right operand

        // Invalid operations
        expect_error("1 @ 2", "Unexpected token");
        expect_error("1 # 2", "Unexpected token");

        // Empty expressions
        expect_error("", "Unexpected end of input");
        expect_error(" ", "Unexpected end of input");

        // Invalid characters
        expect_error("1 $$ 2", "Unexpected token");
        expect_error("§¶", "Unexpected token");
    });
}

#[test]
fn long_and_complex_expressions() {
    with_timeout(|| {
        // Create test context with registered functions when libm is not available
        #[cfg(not(feature = "libm"))]
        let ctx = create_context_rc();

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

            // Use different interp calls based on feature flags
            #[cfg(feature = "libm")]
            let result = interp(expr, None);
            #[cfg(not(feature = "libm"))]
            let result = interp(expr, Some(ctx.clone()));

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
                    panic!("Failed to parse valid expression '{}': {}", expr, e);
                }
            }
        }
    });
}

#[test]
fn test_deeply_nested_function_calls() {
    with_timeout(|| {
        // Create test context with registered functions when libm is not available
        #[cfg(not(feature = "libm"))]
        let ctx = create_context_rc();

        // Construct a deeply nested function call
        // In this case, we'll nest sin() functions 50 levels deep
        let mut expr = "1.0".to_string();
        for _ in 0..50 {
            expr = format!("sin({})", expr);
        }

        // Evaluate the nested expression - this should complete within our timeout
        // and shouldn't cause a stack overflow
        #[cfg(feature = "libm")]
        let result = interp(&expr, None);
        #[cfg(not(feature = "libm"))]
        let result = interp(&expr, Some(ctx.clone()));
        assert!(
            result.is_ok(),
            "Failed to evaluate deeply nested expression"
        );
    });
}

/// Test deeply nested function calls with debug output
#[test]
fn test_deeply_nested_function_calls_debug() {
    with_timeout(|| {
        // Create test context with registered functions when libm is not available
        #[cfg(not(feature = "libm"))]
        let ctx = create_context_rc();

        // Construct a deeply nested function call, with some debug output
        // Start with a simple value
        let mut expr = "0.5".to_string();
        let num_levels = 20; // Use fewer levels for the debug version

        println!("Starting with expr: {}", expr);

        // Keep track of some intermediate values
        let mut intermediate_values = Vec::new();

        // Build up the nested expression
        for i in 1..=num_levels {
            // Calculate the value at this level (optional debugging)
            #[cfg(feature = "libm")]
            let int_result = interp(&expr, None).unwrap();
            #[cfg(not(feature = "libm"))]
            let int_result = interp(&expr, Some(ctx.clone())).unwrap();
            intermediate_values.push(int_result);

            // Nest another function call
            expr = format!("sin({})", expr);

            // Debug output for select iterations (to avoid flooding output)
            if i == 1 || i == 5 || i == 10 || i == num_levels {
                println!("Level {}: expr = {}", i, expr);
            }
        }

        // Print some of the intermediate values
        for (i, val) in intermediate_values.iter().enumerate() {
            if i == 0 || i == 4 || i == 9 || i == intermediate_values.len() - 1 {
                println!("Value at level {}: {}", i + 1, val);
            }
        }

        // Evaluate the final expression
        #[cfg(feature = "libm")]
        let result = interp(&expr, None);
        #[cfg(not(feature = "libm"))]
        let result = interp(&expr, Some(ctx.clone()));
        assert!(
            result.is_ok(),
            "Failed to evaluate deeply nested expression"
        );
        println!("Final result: {}", result.unwrap());
    });
}

/// Test with debug output to analyze recursion depth
#[test]
fn test_deeply_nested_function_calls_with_debugging() {
    with_timeout(|| {
        // Create test context with registered functions when libm is not available
        #[cfg(not(feature = "libm"))]
        let ctx = create_context_rc();

        // Start with a simple value and build a complex expression
        let mut expr = "0.5".to_string();
        let depth = 15; // Use a moderate depth for debugging

        // Build up a very nested expression to test recursion handling
        for i in 1..=depth {
            // Alternate between different function types to make it more complex
            match i % 3 {
                0 => expr = format!("sin({})", expr),
                1 => expr = format!("cos({})", expr),
                2 => expr = format!("sqrt(abs({}))", expr),
                _ => unreachable!(),
            }

            if i % 5 == 0 {
                println!("Depth {}: Expression is now: {}", i, expr);
            }
        }

        println!("Final expression to evaluate:\n{}", expr);

        // Evaluate the nested expression
        let start = Instant::now();
        #[cfg(feature = "libm")]
        let result = interp(&expr, None);
        #[cfg(not(feature = "libm"))]
        let result = interp(&expr, Some(ctx.clone()));
        let elapsed = start.elapsed();

        match result {
            Ok(val) => println!("Success! Result: {} (took {:?})", val, elapsed),
            Err(e) => panic!("Failed to evaluate: {}", e),
        }

        // Verify that we get a result (exact value not important)
        assert!(
            result.is_ok(),
            "Failed to evaluate complex nested expression"
        );
    });
}
