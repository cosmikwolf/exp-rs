//! Integration tests for the exp-rs library
//! These tests demonstrate using the library at various levels of complexity

extern crate alloc;

#[cfg(test)]
use exp_rs::context::EvalContext;
use exp_rs::engine::{interp, parse_expression};
use exp_rs::eval::eval_ast;
use exp_rs::{assert_approx_eq, constants};
use hashbrown::HashMap;
use std::time::Instant;
use std::sync::Mutex;

// Import Real for casting literals
use exp_rs::Real;

use alloc::rc::Rc;

/// Level 1: Basic expression evaluation
#[test]
fn test_basic_expression_evaluation() {
    // Simple arithmetic
    assert_eq!(interp("2 + 3", None).unwrap(), 5.0);
    assert_eq!(interp("2 * 3 + 4", None).unwrap(), 10.0);
    assert_eq!(interp("2 * (3 + 4)", None).unwrap(), 14.0);

    // Built-in functions
    #[cfg(feature = "f32")]
    assert_approx_eq!(
        interp("sin(0.5)", None).unwrap(),
        exp_rs::functions::sin(0.5, 0.0),
        1e-6 as Real // Cast epsilon
    );
    #[cfg(not(feature = "f32"))]
    assert_approx_eq!(
        interp("sin(0.5)", None).unwrap(),
        exp_rs::functions::sin(0.5, 0.0),
        1e-10 as Real // Cast epsilon
    );

    #[cfg(feature = "f32")]
    assert_approx_eq!(
        interp("cos(0.5)", None).unwrap(),
        exp_rs::functions::cos(0.5, 0.0),
        1e-6 as Real // Cast epsilon
    );
    #[cfg(not(feature = "f32"))]
    assert_approx_eq!(
        interp("cos(0.5)", None).unwrap(),
        exp_rs::functions::cos(0.5, 0.0),
        1e-10 as Real // Cast epsilon
    );

    // Constants
    assert_approx_eq!(
        interp("pi", None).unwrap(),
        exp_rs::constants::PI,
        exp_rs::constants::TEST_PRECISION
    );
    assert_approx_eq!(
        interp("e", None).unwrap(),
        exp_rs::constants::E,
        exp_rs::constants::TEST_PRECISION
    );

    // Nested functions
    #[cfg(feature = "f32")]
    assert_approx_eq!(
        interp("sin(cos(0.5))", None).unwrap(),
        exp_rs::functions::sin(exp_rs::functions::cos(0.5, 0.0), 0.0),
        1e-6 as Real // Cast epsilon
    );
    #[cfg(not(feature = "f32"))]
    assert_approx_eq!(
        interp("sin(cos(0.5))", None).unwrap(),
        exp_rs::functions::sin(exp_rs::functions::cos(0.5, 0.0), 0.0),
        1e-10 as Real // Cast epsilon
    );
}

/// Level 2: Using variables in expressions
#[test]
fn test_variable_expressions() {
    let mut ctx = EvalContext::default();

    // Add some variables
    ctx.variables.insert("x".to_string().into(), 5.0);
    ctx.variables.insert("y".to_string().into(), 10.0);

    // Use variables in expressions
    assert_eq!(interp("x + y", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 15.0);
    assert_eq!(interp("x * y", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 50.0);
    assert_eq!(interp("(x + y) / 3", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 5.0);

    // Mix variables with functions
    #[cfg(feature = "f32")]
    assert_approx_eq!(
        interp("sin(x) + cos(y)", Some(std::rc::Rc::new(ctx.clone()))).unwrap(),
        exp_rs::functions::sin(5.0, 0.0) + exp_rs::functions::cos(10.0, 0.0),
        1e-6 as Real // Cast epsilon
    );
    #[cfg(not(feature = "f32"))]
    assert_approx_eq!(
        interp("sin(x) + cos(y)", Some(std::rc::Rc::new(ctx.clone()))).unwrap(),
        exp_rs::functions::sin(5.0, 0.0) + exp_rs::functions::cos(10.0, 0.0),
        1e-10 as Real // Cast epsilon
    );

    // Update variables and re-evaluate
    ctx.variables.insert("x".to_string().into(), 7.0);
    assert_eq!(interp("x + y", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 17.0);
}

/// Level 3: Using arrays in expressions
#[test]
fn test_array_expressions() {
    let mut ctx = EvalContext::default();

    // Add an array
    ctx.arrays
        .insert("data".to_string().into(), vec![10.0, 20.0, 30.0, 40.0, 50.0]);

    // Access array elements
    assert_eq!(interp("data[0]", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 10.0);
    assert_eq!(interp("data[2]", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 30.0);
    assert_eq!(interp("data[4]", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 50.0);

    // Use array elements in expressions
    assert_eq!(interp("data[1] + data[3]", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 60.0);
    assert_eq!(interp("data[2] * 2", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 60.0);

    // Use expressions as array indices
    assert_eq!(interp("data[1+1]", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 30.0);
    assert_eq!(interp("data[floor(1.8)]", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 20.0);

    // Add variables to use as indices
    ctx.variables.insert("i".to_string().into(), 3.0);
    assert_eq!(interp("data[i]", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 40.0);
}

/// Level 4: Using attributes in expressions
#[test]
fn test_attribute_expressions() {
    let mut ctx = EvalContext::default();

    // Add an object with attributes
    let mut point = HashMap::new();
    point.insert("x".to_string().into(), 3.0);
    point.insert("y".to_string().into(), 4.0);
    ctx.attributes.insert("point".to_string().into(), point);

    // Access attributes
    assert_eq!(interp("point.x", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 3.0);
    assert_eq!(interp("point.y", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 4.0);

    // Use attributes in expressions
    assert_eq!(interp("point.x + point.y", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 7.0);
    assert_approx_eq!( // Use approx_eq for sqrt result
        interp("sqrt(point.x^2 + point.y^2)", Some(std::rc::Rc::new(ctx.clone()))).unwrap(),
        5.0 as Real, // Cast expected value
        crate::constants::TEST_PRECISION
    );

    // Add another object
    let mut circle = HashMap::new();
    circle.insert("radius".to_string().into(), 10.0);
    circle.insert("center_x".to_string().into(), 5.0);
    circle.insert("center_y".to_string().into(), 5.0);
    ctx.attributes.insert("circle".to_string().into(), circle);

    // Calculate distance from point to circle center
    let expr = "sqrt((point.x - circle.center_x)^2 + (point.y - circle.center_y)^2)";

    // Add detailed debug prints to see what's happening
    println!("point.x = {}", interp("point.x", Some(std::rc::Rc::new(ctx.clone()))).unwrap());
    println!("point.y = {}", interp("point.y", Some(std::rc::Rc::new(ctx.clone()))).unwrap());
    println!(
        "circle.center_x = {}",
        interp("circle.center_x", Some(std::rc::Rc::new(ctx.clone()))).unwrap()
    );
    println!(
        "circle.center_y = {}",
        interp("circle.center_y", Some(std::rc::Rc::new(ctx.clone()))).unwrap()
    );

    // Debug the subexpressions
    println!(
        "(point.x - circle.center_x) = {}",
        interp("(point.x - circle.center_x)", Some(std::rc::Rc::new(ctx.clone()))).unwrap()
    );
    println!(
        "(point.y - circle.center_y) = {}",
        interp("(point.y - circle.center_y)", Some(std::rc::Rc::new(ctx.clone()))).unwrap()
    );
    println!(
        "(point.x - circle.center_x)^2 = {}",
        interp("(point.x - circle.center_x)^2", Some(std::rc::Rc::new(ctx.clone()))).unwrap()
    );
    println!(
        "(point.y - circle.center_y)^2 = {}",
        interp("(point.y - circle.center_y)^2", Some(std::rc::Rc::new(ctx.clone()))).unwrap()
    );
    println!(
        "(point.x - circle.center_x)^2 + (point.y - circle.center_y)^2 = {}",
        interp(
            "(point.x - circle.center_x)^2 + (point.y - circle.center_y)^2",
            Some(std::rc::Rc::new(ctx.clone()))
        )
        .unwrap()
    );
    println!(
        "Full expression: {} = {}",
        expr,
        interp(expr, Some(std::rc::Rc::new(ctx.clone()))).unwrap()
    );

    // Now run the assertion with the correct expected value
    // The distance between points (3,4) and (5,5) is sqrt(5) ≈ 2.23606797749979
    assert_approx_eq!(
        interp(expr, Some(std::rc::Rc::new(ctx.clone()))).unwrap(),
        2.23606797749979 as Real, // Cast expected value
        crate::constants::TEST_PRECISION
    );

    // Check if point is inside circle
    let inside_expr =
        "sqrt((point.x - circle.center_x)^2 + (point.y - circle.center_y)^2) < circle.radius";
    // We can't directly evaluate boolean expressions, so we'll use a numeric comparison
    let err = interp(inside_expr, Some(std::rc::Rc::new(ctx.clone()))).unwrap_err();
    println!("Error for inside_expr: {}", err);
    assert!(err.to_string().contains("Unknown") || err.to_string().contains("<"));
}

/// Level 5: Custom functions
#[test]
fn test_custom_functions() {
    let mut ctx = EvalContext::new();

    // Register a simple native function that adds all its arguments
    ctx.register_native_function("sum", 3, |args| args.iter().sum());

    // Test the custom function
    assert_eq!(interp("sum(1, 2, 3)", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 6.0);
    assert_eq!(interp("sum(10, 20, 30)", Some(std::rc::Rc::new(ctx.clone()))).unwrap(), 60.0);

    // Register a function that calculates the distance between two points
    ctx.register_native_function("distance", 4, |args| {
        let x1 = args[0];
        let y1 = args[1];
        let x2 = args[2];
        let y2 = args[3];
        exp_rs::functions::sqrt((x2 - x1).powi(2) + (y2 - y1).powi(2), 0.0)
    });

    // Test the distance function
    assert_approx_eq!( // Use approx_eq for sqrt result
        interp("distance(0, 0, 3, 4)", Some(std::rc::Rc::new(ctx.clone()))).unwrap(),
        5.0 as Real, // Cast expected value
        crate::constants::TEST_PRECISION
    );
    assert_approx_eq!(
        interp("distance(1, 1, 4, 5)", Some(std::rc::Rc::new(ctx.clone()))).unwrap(),
        5.0 as Real, // Cast expected value
        1e-10 as Real // Cast epsilon
    );

    // Register a function that calculates the area of a circle
    ctx.register_native_function("circle_area", 1, |args| {
        let radius = args[0];
        exp_rs::constants::PI * radius * radius
    });

    // Test the circle area function
    assert_approx_eq!(
        interp("circle_area(2)", Some(std::rc::Rc::new(ctx.clone()))).unwrap(),
        exp_rs::constants::PI * 4.0,
        exp_rs::constants::TEST_PRECISION
    );

    // Combine custom functions with built-in functions
    assert_approx_eq!(
        interp("circle_area(distance(0, 0, 3, 4) / 2)", Some(std::rc::Rc::new(ctx.clone()))).unwrap(),
        crate::constants::PI * 6.25,
        crate::constants::TEST_PRECISION
    );
}

/// Level 6: Complex expressions with multiple features
#[test]
fn test_complex_expressions() {
    let mut ctx = EvalContext::default();

    // Set up variables
    ctx.variables.insert("t".to_string().into(), 0.5);
    ctx.variables.insert("amplitude".to_string().into(), 10.0);
    ctx.variables.insert("frequency".to_string().into(), 2.0);

    // Set up arrays
    ctx.arrays
        .insert("samples".to_string().into(), vec![1.0, 2.0, 3.0, 4.0, 5.0]);

    // Set up attributes
    let mut wave = HashMap::new();
    wave.insert("phase".to_string().into(), 0.25);
    wave.insert("offset".to_string().into(), 5.0);
    ctx.attributes.insert("wave".to_string().into(), wave);

    // Register native functions
    ctx.register_native_function("interpolate", 3, |args| {
        let a = args[0];
        let b = args[1];
        let t = args[2];
        a * (1.0 - t) + b * t
    });

    // Add debug prints to see what's happening
    println!("t = {}", interp("t", Some(std::rc::Rc::new(ctx.clone()))).unwrap());
    println!(
        "amplitude = {}",
        interp("amplitude", Some(std::rc::Rc::new(ctx.clone()))).unwrap()
    );
    println!(
        "frequency = {}",
        interp("frequency", Some(std::rc::Rc::new(ctx.clone()))).unwrap()
    );
    println!(
        "wave.phase = {}",
        interp("wave.phase", Some(std::rc::Rc::new(ctx.clone()))).unwrap()
    );
    println!(
        "wave.offset = {}",
        interp("wave.offset", Some(std::rc::Rc::new(ctx.clone()))).unwrap()
    );

    // Complex expression combining all features
    let expr = "amplitude * sin(2 * pi * frequency * t + wave.phase) + wave.offset + samples[floor(t * 4)]";
    println!("Evaluating expression: {}", expr);

    // Calculate expected result
    let expected = 10.0
        * exp_rs::functions::sin(2.0 * exp_rs::constants::PI * 2.0 * 0.5 + 0.25, 0.0)
        + 5.0
        + 3.0;
    println!("Expected result: {}", expected);

    let result = interp(expr, Some(std::rc::Rc::new(ctx.clone())));
    match &result {
        Ok(val) => println!("Actual result: {}", val),
        Err(e) => println!("Error: {}", e),
    }

    assert_approx_eq!(
        result.unwrap(),
        expected as Real, // Cast expected value
        exp_rs::constants::TEST_PRECISION
    );

    // Another complex expression with custom function
    let expr2 = "interpolate(samples[1], samples[2], t) * (1 + sin(wave.phase))";

    // Calculate expected result
    let expected2 = (2.0 * (1.0 - 0.5) + 3.0 * 0.5) * (1.0 + exp_rs::functions::sin(0.25, 0.0));

    assert_approx_eq!(
        interp(expr2, Some(std::rc::Rc::new(ctx.clone()))).unwrap(),
        expected2 as Real, // Cast expected value
        exp_rs::constants::TEST_PRECISION
    );
}

/// Level 7: Performance testing
#[test]
fn test_expression_performance() {
    // Create a moderately complex expression
    let expr = "sin(x) * cos(y) + sqrt(z^2 + w^2) / log(u + 5)";

    // Set up context with variables
    let mut ctx = EvalContext::default();
    ctx.variables.insert("x".to_string().into(), 1.0);
    ctx.variables.insert("y".to_string().into(), 2.0);
    ctx.variables.insert("z".to_string().into(), 3.0);
    ctx.variables.insert("w".to_string().into(), 4.0);
    ctx.variables.insert("u".to_string().into(), 5.0);

    // Parse the expression once
    let ast = parse_expression(expr).unwrap();

    // Measure time to evaluate the expression 10,000 times
    let iterations = 10_000;
    let start = Instant::now();

    for i in 0..iterations {
        // Update a variable to ensure we're not just caching the result
        ctx.variables
            .insert("x".to_string().into(), (i % 100) as Real / 100.0);
        let _ = eval_ast(&ast, Some(Rc::new(ctx.clone()))).unwrap();
    }

    let duration = start.elapsed();
    let avg_micros = duration.as_micros() as f64 / iterations as f64;

    println!("Average evaluation time: {:.2} microseconds", avg_micros);

    // On most modern systems, this should be well under 100 microseconds per evaluation
    // which would allow for >10,000 evaluations per second
    assert!(
        avg_micros < 100.0,
        "Expression evaluation is too slow: {:.2} microseconds",
        avg_micros
    );
}

/// Level 8: Error handling
#[test]
fn test_error_handling() {
    // Test syntax errors
    let result = interp("1 +", None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Syntax"));

    // Test unknown variable
    let result = interp("x + 5", None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown variable"));

    // Test unknown function
    let result = interp("foo(5)", None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown function"));

    // Test invalid function arity
    let result = interp("sin(1, 2)", None);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid function call"));

    // Test array index out of bounds
    let mut ctx = EvalContext::new();
    ctx.arrays.insert("arr".to_string().into(), vec![1.0, 2.0, 3.0]);

    let result = interp("arr[5]", Some(std::rc::Rc::new(ctx.clone())));
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    println!("Array index error message: {}", err_msg);
    assert!(err_msg.contains("Array index out of bounds"));

    // Test attribute not found
    let mut obj = HashMap::new();
    obj.insert("x".to_string().into(), 1.0);
    ctx.attributes.insert("obj".to_string().into(), obj);

    let result = interp("obj.y", Some(std::rc::Rc::new(ctx.clone())));
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Attribute not found"));

    // Test custom function errors
    ctx.register_native_function("safe_divide", 2, |args| {
        if args[1] == 0.0 {
            #[cfg(feature = "f32")]
            return f32::NAN;
            #[cfg(not(feature = "f32"))]
            return f64::NAN;
        } else {
            args[0] / args[1]
        }
    });

    // This should return NaN, not an error
    let result = interp("safe_divide(1, 0)", Some(std::rc::Rc::new(ctx.clone()))).unwrap();
    assert!(result.is_nan());

    // Test wrong arity for custom function
    let result = interp("safe_divide(1)", Some(std::rc::Rc::new(ctx.clone())));
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid function call"));
}

/// Level 9: Advanced native function usage
#[test]
fn test_advanced_native_functions() {
    // For the low-pass filter test, we'll use a different approach
    // Instead of trying to modify the context from inside the closure,
    // we'll use a separate variable to track state
    {
        let mut ctx = EvalContext::new();
        let prev_output = Mutex::new(0.0);

        // Register a function that implements a simple low-pass filter
        // y[n] = alpha * x[n] + (1-alpha) * y[n-1]
        ctx.register_native_function("low_pass_filter", 2, {
            // No need to clone the Mutex, just move it into the closure
            move |args| {
                let input = args[0];
                let alpha = args[1];

                let mut prev = prev_output.lock().unwrap();
                let output = alpha * input + (1.0 - alpha) * *prev;
                *prev = output; // Update the state

                output
            }
        });

        // Test the filter with a step input
        let result1 = interp("low_pass_filter(1.0, 0.2)", Some(std::rc::Rc::new(ctx.clone()))).unwrap();
        assert_approx_eq!(result1, 0.2 as Real, 1e-10 as Real); // 0.2 * 1.0 + 0.8 * 0.0

        let result2 = interp("low_pass_filter(1.0, 0.2)", Some(std::rc::Rc::new(ctx.clone()))).unwrap();
        assert_approx_eq!(result2, 0.36 as Real, 1e-10 as Real); // 0.2 * 1.0 + 0.8 * 0.2

        let result3 = interp("low_pass_filter(1.0, 0.2)", Some(std::rc::Rc::new(ctx.clone()))).unwrap();
        // The correct calculation is: 0.2 * 1.0 + 0.8 * 0.36 = 0.2 + 0.288 = 0.488
        // Let's use a slightly larger epsilon to account for floating-point precision
        assert_approx_eq!(result3, 0.488 as Real, constants::TEST_PRECISION); // Use TEST_PRECISION for consistent behavior
    }

    // For the PID controller test, we'll use a similar approach
    {
        let mut ctx = EvalContext::new();
        let integral = Mutex::new(0.0);
        let prev_error = Mutex::new(0.0);

        // Register a function that implements a PID controller
        ctx.register_native_function("pid_controller", 5, {
            // No need to clone the Mutexes, just move them into the closure
            move |args| {
                let setpoint = args[0];
                let process_variable = args[1];
                let kp = args[2];
                let ki = args[3];
                let kd = args[4];

                let error = setpoint - process_variable;

                // Update integral and calculate derivative
                let mut integral_guard = integral.lock().unwrap();
                *integral_guard += error;
                let mut prev_error_guard = prev_error.lock().unwrap();
                let derivative = error - *prev_error_guard;

                // Calculate PID output
                let output = kp * error + ki * *integral_guard + kd * derivative;

                // Update previous error for next call
                *prev_error_guard = error;

                output
            }
        });

        // Test the PID controller
        let result = interp("pid_controller(100, 90, 0.5, 0.1, 0.2)", Some(std::rc::Rc::new(ctx.clone()))).unwrap();
        // error = 10, integral = 10, derivative = 10
        // output = 0.5 * 10 + 0.1 * 10 + 0.2 * 10 = 8.0
        assert_approx_eq!(result, 8.0 as Real, 1e-10 as Real); // Cast expected and epsilon
    }
}

/// Level 10: Expression functions and YAML configuration
#[test]
fn test_expression_functions() {
    // Create a context with some variables
    let mut ctx = EvalContext::new();
    ctx.variables.insert("x".to_string().into(), 5.0);
    ctx.variables.insert("y".to_string().into(), 10.0);

    // Register an expression function that calculates the hypotenuse of a right triangle
    ctx.register_expression_function("hypotenuse", &["a", "b"], "sqrt(a^2 + b^2)")
        .unwrap();

    // Test the expression function with variables
    let result = interp("hypotenuse(x, y)", Some(std::rc::Rc::new(ctx.clone()))).unwrap();
    assert_approx_eq!(
        result, 11.18034 as Real, 1e-5 as Real // Cast expected and epsilon
    );

    // Register an expression function that uses another expression function
    ctx.register_expression_function(
        "distance",
        &["x1", "y1", "x2", "y2"],
        "hypotenuse(x2 - x1, y2 - y1)",
    )
    .unwrap();

    // Test the nested expression function
    let result2 = interp("distance(0, 0, 3, 4)", Some(std::rc::Rc::new(ctx.clone()))).unwrap();
    assert_eq!(result2, 5.0, "distance(0, 0, 3, 4) should be 5.0");

    // Register a more complex expression function
    ctx.register_expression_function("polynomial", &["x"], "x^3 + 2*x^2 + 3*x + 4")
        .unwrap();

    // Test the polynomial function
    let result3 = interp("polynomial(2)", Some(std::rc::Rc::new(ctx.clone()))).unwrap();
    println!("polynomial(2) = {}", result3); // Debug output
                                             // For x=2: 2^3 + 2*2^2 + 3*2 + 4 = 8 + 8 + 6 + 4 = 26
    assert_eq!(result3, 26.0, "polynomial(2) should be 26.0");

    // Test expression function with a complex expression as argument
    let result4 = interp("polynomial(x + y / 2)", Some(std::rc::Rc::new(ctx.clone()))).unwrap();
    let x = 5.0;
    let y = 10.0;
    let arg = x + y / 2.0;
    // For x=5, y=10: arg = 10.0
    // polynomial(10) = 10^3 + 2*10^2 + 3*10 + 4 = 1000 + 200 + 30 + 4 = 1234
    let expected = libm::pow(arg, 3.0) + 2.0 * libm::pow(arg, 2.0)
        + 3.0 * arg
        + 4.0;
    println!("polynomial({}) = {}", arg, result4);
    println!("expected = {}", expected);
    assert!(
        (result4 - expected as Real).abs() < 1e-6,
        "polynomial({}) should be {} (got {})",
        arg,
        expected,
        result4
    );
}

/// Level 11: Parsing and evaluating expressions from a configuration
#[test]
fn test_config_expressions() {
    // Create context with configuration values
    let mut ctx = EvalContext::default();

    // Add constants
    ctx.constants.insert("AMPLITUDE_MIN".to_string().into(), 2.0);
    ctx.constants.insert("AMPLITUDE_MAX".to_string().into(), 75.0);
    ctx.constants.insert("VOLTAGE_MAX".to_string().into(), 5.0);

    // Add data tables
    ctx.arrays.insert(
        "wait_times".to_string().into(),
        vec![64691.0, 64625.0, 64559.0, 64494.0, 64428.0],
    );

    // Add parameters
    ctx.variables.insert("power".to_string().into(), 50.0);
    ctx.variables.insert("speed".to_string().into(), 50.0);
    ctx.variables.insert("t".to_string().into(), 0.5);

    // Evaluate derived parameters
    let pattern_step_expr = "(speed / 100.0) * (9.2 - 0.27) + 0.27";
    let pattern_step = interp(pattern_step_expr, Some(std::rc::Rc::new(ctx.clone()))).unwrap();
    ctx.variables
        .insert("pattern_step".to_string().into(), pattern_step);

    let pattern_index_expr = "t * pattern_step";
    let pattern_index = interp(pattern_index_expr, Some(std::rc::Rc::new(ctx.clone()))).unwrap();
    ctx.variables
        .insert("pattern_index".to_string().into(), pattern_index);

    // Evaluate main function
    let main_function = "((power * (AMPLITUDE_MAX - AMPLITUDE_MIN) + AMPLITUDE_MIN) * VOLTAGE_MAX) / AMPLITUDE_MAX * 1000";
    let result = interp(main_function, Some(std::rc::Rc::new(ctx.clone()))).unwrap();

    // Calculate expected result
    let expected: Real = ((50.0 * (75.0 - 2.0) + 2.0) * 5.0) / 75.0 * 1000.0;

    // Use a relative precision based on the magnitude of the expected value
    #[cfg(feature = "f32")]
    let relative_precision = expected.abs() * 1e-6 as Real; // Cast epsilon
    #[cfg(not(feature = "f32"))]
    let relative_precision = expected.abs() * 1e-10 as Real; // Cast epsilon

    assert_approx_eq!(result, expected, relative_precision);

    // Test with different parameter values
    ctx.variables.insert("power".to_string().into(), 75.0);
    ctx.variables.insert("speed".to_string().into(), 25.0);
    ctx.variables.insert("t".to_string().into(), 1.0);

    // Re-evaluate derived parameters
    let pattern_step = interp(pattern_step_expr, Some(std::rc::Rc::new(ctx.clone()))).unwrap();
    ctx.variables
        .insert("pattern_step".to_string().into(), pattern_step);

    let pattern_index = interp(pattern_index_expr, Some(std::rc::Rc::new(ctx.clone()))).unwrap();
    ctx.variables
        .insert("pattern_index".to_string().into(), pattern_index);

    // Re-evaluate main function
    let result = interp(main_function, Some(std::rc::Rc::new(ctx.clone()))).unwrap();

    // Calculate expected result
    let expected: Real = ((75.0 * (75.0 - 2.0) + 2.0) * 5.0) / 75.0 * 1000.0;

    // Use a relative precision based on the magnitude of the expected value
    #[cfg(feature = "f32")]
    let relative_precision = expected.abs() * 1e-6 as Real; // Cast epsilon
    #[cfg(not(feature = "f32"))]
    let relative_precision = expected.abs() * 1e-10 as Real; // Cast epsilon

    assert_approx_eq!(result, expected, relative_precision);
}
