#![cfg_attr(all(not(test), target_arch = "arm"), no_std)]
#![doc = r#"
# exp-rs

A minimal, extensible, no_std-friendly math expression parser and evaluator for Rust.

## Overview

exp-rs is a math expression parser and evaluator library designed to be simple, extensible, and compatible with no_std environments. It was inspired by [TinyExpr](https://github.com/codeplea/tinyexpr) and [TinyExpr++](https://github.com/Blake-Madden/tinyexpr-plusplus), but with additional features and Rust-native design.

Key features:
- Configurable floating-point precision (f32/f64)
- Support for user-defined variables, constants, arrays, attributes, and functions
- Built-in math functions (sin, cos, pow, etc.) that can be enabled/disabled
- Ability to override any built-in function at runtime
- Array access with `array[index]` syntax
- Object attributes with `object.attribute` syntax
- Function application by juxtaposition (`sin x` is equivalent to `sin(x)`)
- Comprehensive error handling
- No_std compatibility for embedded systems
- Integration with CMSIS-DSP for ARM Cortex-M

## Quick Start

Here's a basic example of evaluating a math expression:

```rust
use exp_rs::engine::interp;

fn main() {
    // Simple expression evaluation
    let result = interp("2 + 3 * 4", None).unwrap();
    assert_eq!(result, 14.0); // 2 + (3 * 4) = 14
    
    // Using built-in functions and constants
    let result = interp("sin(pi/4) + cos(pi/4)", None).unwrap();
    assert!(result - 1.414 < 0.001); // Approximately √2
}
```

## Using Variables and Constants

```rust
use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use std::rc::Rc;

fn main() {
    let mut ctx = EvalContext::new();
    
    // Add variables
    ctx.set_parameter("x", 5.0);
    ctx.set_parameter("y", 10.0);
    
    // Add constants - these won't change once set
    ctx.constants.insert("FACTOR".to_string(), 2.5);
    
    // Evaluate expression with variables and constants
    let result = interp("x + y * FACTOR", Some(Rc::new(ctx))).unwrap();
    assert_eq!(result, 30.0); // 5 + (10 * 2.5) = 30
}
```

## Arrays and Object Attributes

```rust
use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use std::collections::HashMap;
use std::rc::Rc;

fn main() {
    let mut ctx = EvalContext::new();
    
    // Add an array
    ctx.arrays.insert("data".to_string(), vec![10.0, 20.0, 30.0, 40.0, 50.0]);
    
    // Add an object with attributes
    let mut point = HashMap::new();
    point.insert("x".to_string(), 3.0);
    point.insert("y".to_string(), 4.0);
    ctx.attributes.insert("point".to_string(), point);
    
    // Access array elements
    let result = interp("data[2]", Some(Rc::new(ctx.clone()))).unwrap();
    assert_eq!(result, 30.0);
    
    // Access attributes
    let result = interp("point.x + point.y", Some(Rc::new(ctx.clone()))).unwrap();
    assert_eq!(result, 7.0);
    
    // Combine array and attribute access in expressions
    let result = interp("sqrt(point.x^2 + point.y^2) + data[0]", Some(Rc::new(ctx))).unwrap();
    assert_eq!(result, 15.0); // sqrt(3^2 + 4^2) + 10 = 5 + 10 = 15
}
```

## Custom Functions

### Native Functions

```rust
use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use std::rc::Rc;

fn main() {
    let mut ctx = EvalContext::new();
    
    // Register a native function that sums all arguments
    ctx.register_native_function("sum", 3, |args| {
        args.iter().sum()
    });
    
    // Register a native function with variable number of arguments
    ctx.register_native_function("average", 0, |args| {
        if args.is_empty() {
            0.0
        } else {
            args.iter().sum::<f64>() / args.len() as f64
        }
    });
    
    // Use the custom functions
    let result = interp("sum(1, 2, 3)", Some(Rc::new(ctx.clone()))).unwrap();
    assert_eq!(result, 6.0);
    
    let result = interp("average(10, 20, 30, 40)", Some(Rc::new(ctx))).unwrap();
    assert_eq!(result, 25.0);
}
```

### Expression Functions

```rust
use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use std::rc::Rc;

fn main() {
    let mut ctx = EvalContext::new();
    
    // Register an expression function
    ctx.register_expression_function(
        "hypotenuse",
        &["a", "b"],
        "sqrt(a^2 + b^2)"
    ).unwrap();
    
    // Register a recursive expression function
    ctx.register_expression_function(
        "factorial",
        &["n"],
        "n <= 1 ? 1 : n * factorial(n - 1)"
    ).unwrap();
    
    // Use the custom functions
    let result = interp("hypotenuse(3, 4)", Some(Rc::new(ctx.clone()))).unwrap();
    assert_eq!(result, 5.0);
    
    let result = interp("factorial(5)", Some(Rc::new(ctx))).unwrap();
    assert_eq!(result, 120.0); // 5! = 5 * 4 * 3 * 2 * 1 = 120
}
```

## Performance Optimization with AST Caching

For repeated evaluations of the same expression with different variables:

```rust
use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use std::rc::Rc;

fn main() {
    let mut ctx = EvalContext::new();
    ctx.enable_ast_cache(); // Enable AST caching
    
    // First evaluation will parse and cache the AST
    ctx.set_parameter("x", 1.0);
    let result1 = interp("x^2 + 2*x + 1", Some(Rc::new(ctx.clone()))).unwrap();
    assert_eq!(result1, 4.0); // 1^2 + 2*1 + 1 = 4
    
    // Subsequent evaluations with the same expression will reuse the cached AST
    ctx.set_parameter("x", 2.0);
    let result2 = interp("x^2 + 2*x + 1", Some(Rc::new(ctx.clone()))).unwrap();
    assert_eq!(result2, 9.0); // 2^2 + 2*2 + 1 = 9
    
    // This is much faster for repeated evaluations
    ctx.set_parameter("x", 3.0);
    let result3 = interp("x^2 + 2*x + 1", Some(Rc::new(ctx))).unwrap();
    assert_eq!(result3, 16.0); // 3^2 + 2*3 + 1 = 16
}
```

## Using on Embedded Systems (no_std)

exp-rs is designed to work in no_std environments with the alloc crate:

```rust
#![no_std]
extern crate alloc;

use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use alloc::rc::Rc;

#[no_mangle]
pub extern "C" fn evaluate_expression(x: f32, y: f32) -> f32 {
    let mut ctx = EvalContext::new();
    ctx.set_parameter("x", x as f64);
    ctx.set_parameter("y", y as f64);
    
    let result = interp("sqrt(x^2 + y^2)", Some(Rc::new(ctx))).unwrap();
    result as f32
}
```

## Disabling Built-in Math Functions

For embedded systems where you want to provide your own math implementations:

```rust
// In Cargo.toml:
// exp-rs = { version = "0.1", default-features = false, features = ["f32", "no-builtin-math"] }

use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use std::rc::Rc;

fn main() {
    let mut ctx = EvalContext::new();
    
    // Register custom math functions
    ctx.register_native_function("sin", 1, |args| libm::sinf(args[0]));
    ctx.register_native_function("cos", 1, |args| libm::cosf(args[0]));
    ctx.register_native_function("sqrt", 1, |args| libm::sqrtf(args[0]));
    
    // Now you can use these functions
    let result = interp("sin(0.5) + cos(0.5)", Some(Rc::new(ctx))).unwrap();
    println!("Result: {}", result);
}
```

## Error Handling

Comprehensive error handling is provided:

```rust
use exp_rs::context::EvalContext;
use exp_rs::engine::interp;
use exp_rs::error::ExprError;
use std::rc::Rc;

fn main() {
    let ctx = EvalContext::new();
    
    // Handle syntax errors
    match interp("2 + * 3", Some(Rc::new(ctx.clone()))) {
        Ok(_) => println!("Unexpected success"),
        Err(ExprError::Syntax(msg)) => println!("Syntax error: {}", msg),
        Err(e) => println!("Unexpected error: {:?}", e),
    }
    
    // Handle unknown variables
    match interp("x + 5", Some(Rc::new(ctx.clone()))) {
        Ok(_) => println!("Unexpected success"),
        Err(ExprError::UnknownVariable { name }) => println!("Unknown variable: {}", name),
        Err(e) => println!("Unexpected error: {:?}", e),
    }
    
    // Handle division by zero
    match interp("1 / 0", Some(Rc::new(ctx))) {
        Ok(result) => {
            if result.is_infinite() {
                println!("Division by zero correctly returned infinity")
            } else {
                println!("Unexpected result: {}", result)
            }
        },
        Err(e) => println!("Unexpected error: {:?}", e),
    }
}
```

## Supported Grammar

exp-rs supports a superset of the original TinyExpr grammar, closely matching the [tinyexpr++](https://github.com/Blake-Madden/tinyexpr-plusplus) grammar, including:

- Multi-character operators: `&&`, `||`, `==`, `!=`, `<=`, `>=`, `<<`, `>>`, `<<<`, `>>>`, `**`, `<>`
- Logical, comparison, bitwise, and exponentiation operators with correct precedence and associativity
- List expressions and both comma and semicolon as separators
- Function call syntax supporting both parentheses and juxtaposition
- Array and attribute access
- Right-associative exponentiation

### Operator Precedence and Associativity

From lowest to highest precedence:

| Precedence | Operators                                 | Associativity      |
|------------|-------------------------------------------|--------------------|
| 1          | `,` `;`                                   | Left               |
| 2          | `||`                                      | Left               |
| 3          | `&&`                                      | Left               |
| 4          | `|`                                       | Left (bitwise OR)  |
| 6          | `&`                                       | Left (bitwise AND) |
| 7          | `==` `!=` `<` `>` `<=` `>=` `<>`          | Left (comparison)  |
| 8          | `<<` `>>` `<<<` `>>>`                     | Left (bit shifts)  |
| 9          | `+` `-`                                   | Left               |
| 10         | `*` `/` `%`                               | Left               |
| 14         | unary `+` `-` `~`                         | Right (unary)      |
| 15         | `^`                                       | Right              |
| 16         | `**`                                      | Right              |

### Built-in Functions

The following functions are available by default (unless `no-builtin-math` is enabled):

- Trigonometric: `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`
- Hyperbolic: `sinh`, `cosh`, `tanh`
- Exponential/Logarithmic: `exp`, `log`, `log10`, `ln`
- Power/Root: `sqrt`, `pow`
- Rounding: `ceil`, `floor`
- Comparison: `max`, `min`
- Misc: `abs`, `sign`

### Built-in Constants

- `pi`: 3.14159... (π)
- `e`: 2.71828... (Euler's number)

## Feature Flags

- `no-builtin-math`: Disables all built-in math functions. You must register your own.
- `f32`: Use 32-bit floating point (single precision) for calculations
- `f64`: Use 64-bit floating point (double precision) for calculations (default)

Only one of `f32` or `f64` can be enabled at a time.

## Embedded Systems Support

exp-rs provides extensive support for embedded systems:

- `no_std` compatible with the `alloc` crate
- Configurable precision with `f32`/`f64` options
- Option to disable built-in math functions and provide custom implementations
- Integration with CMSIS-DSP for ARM Cortex-M processors
- Meson build system integration for cross-compilation
- QEMU test harness for validating on ARM hardware
- Optional C FFI for calling from non-Rust code

## Attribution

exp-rs began as a fork of [tinyexpr-rs](https://github.com/kondrak/tinyexpr-rs) by Krzysztof Kondrak, which itself was a port of the [TinyExpr](https://github.com/codeplea/tinyexpr) C library by Lewis Van Winkle (codeplea). As the functionality expanded beyond the scope of the original TinyExpr, it evolved into a new project with additional features inspired by [tinyexpr-plusplus](https://github.com/Blake-Madden/tinyexpr-plusplus).

"#]

// Re-export alloc for no_std compatibility
#[cfg(all(not(test), target_arch = "arm"))]
extern crate alloc;
#[cfg(all(not(test), target_arch = "arm"))]
pub use alloc::boxed::Box;
#[cfg(all(not(test), target_arch = "arm"))]
pub use alloc::string::{String, ToString};
#[cfg(all(not(test), target_arch = "arm"))]
pub use alloc::vec::Vec;

// For non-ARM targets, keep the original behavior
#[cfg(not(all(not(test), target_arch = "arm")))]
#[cfg(not(test))]
extern crate alloc;
#[cfg(not(all(not(test), target_arch = "arm")))]
#[cfg(not(test))]
pub use alloc::boxed::Box;
#[cfg(not(all(not(test), target_arch = "arm")))]
#[cfg(not(test))]
pub use alloc::string::{String, ToString};
#[cfg(not(all(not(test), target_arch = "arm")))]
#[cfg(not(test))]
pub use alloc::vec::Vec;

// Ensure core::result::Result, core::result::Result::Ok, and core::result::Result::Err are in scope for no_std/serde

pub mod context;
pub mod engine;
pub mod error;
pub mod eval;
pub mod expression_functions;
pub mod ffi;
pub mod functions;
pub mod lexer;
pub mod types;

pub use context::*;
pub use engine::*;
pub use functions::*;
pub use types::*;

pub use ffi::*;

// Compile-time check: only one of f32 or f64 can be enabled
#[cfg(all(feature = "f32", feature = "f64"))]
compile_error!("You must enable only one of the features: 'f32' or 'f64', not both.");

/// Define the floating-point type based on feature flags
#[cfg(feature = "f32")]
pub type Real = f32;

#[cfg(feature = "f64")]
pub type Real = f64;

pub mod constants {
    use super::Real;

    #[cfg(feature = "f32")]
    pub const PI: Real = core::f32::consts::PI;
    #[cfg(feature = "f32")]
    pub const E: Real = core::f32::consts::E;
    #[cfg(feature = "f32")]
    pub const TEST_PRECISION: Real = 1e-6;

    #[cfg(feature = "f64")]
    pub const PI: Real = core::f64::consts::PI;
    #[cfg(feature = "f64")]
    pub const E: Real = core::f64::consts::E;
    #[cfg(feature = "f64")]
    pub const TEST_PRECISION: Real = 1e-10;
}

/// Utility macro to check if two floating point values are approximately equal
/// within a specified epsilon. Supports optional format arguments like assert_eq!.
#[macro_export]
macro_rules! assert_approx_eq {
    // Case 1: assert_approx_eq!(left, right) -> use default epsilon
    ($left:expr, $right:expr $(,)?) => {
        $crate::assert_approx_eq!($left, $right, $crate::constants::TEST_PRECISION)
    };
    // Case 2: assert_approx_eq!(left, right, epsilon) -> use specified epsilon
    ($left:expr, $right:expr, $epsilon:expr $(,)?) => {{
        let left_val = $left;
        let right_val = $right;
        let eps = $epsilon;

        // Use a default message if none is provided
        let message = format!(
            "assertion failed: `(left ≈ right)` \
             (left: `{}`, right: `{}`, epsilon: `{}`)",
            left_val, right_val, eps
        );

        if left_val.is_nan() && right_val.is_nan() {
            // NaN == NaN for our purposes
        } else if left_val.is_infinite()
            && right_val.is_infinite()
            && left_val.signum() == right_val.signum()
        {
            // Same-signed infinities are equal
        } else {
            assert!((left_val - right_val).abs() < eps, "{}", message);
        }
    }};
    // Case 3: assert_approx_eq!(left, right, epsilon, "format message") -> use specified epsilon and message
    ($left:expr, $right:expr, $epsilon:expr, $msg:literal $(,)?) => {{
        let left_val = $left;
        let right_val = $right;
        let eps = $epsilon;

        if left_val.is_nan() && right_val.is_nan() {
            // NaN == NaN for our purposes
        } else if left_val.is_infinite()
            && right_val.is_infinite()
            && left_val.signum() == right_val.signum()
        {
            // Same-signed infinities are equal
        } else {
            assert!((left_val - right_val).abs() < eps, $msg);
        }
    }};
    // Case 4: assert_approx_eq!(left, right, epsilon, "format message with args", args...) -> use specified epsilon and formatted message
    ($left:expr, $right:expr, $epsilon:expr, $fmt:expr, $($arg:tt)+) => {{
        let left_val = $left;
        let right_val = $right;
        let eps = $epsilon;

        if left_val.is_nan() && right_val.is_nan() {
            // NaN == NaN for our purposes
        } else if left_val.is_infinite()
            && right_val.is_infinite()
            && left_val.signum() == right_val.signum()
        {
            // Same-signed infinities are equal
        } else {
            assert!((left_val - right_val).abs() < eps, $fmt, $($arg)+);
        }
    }};
}
