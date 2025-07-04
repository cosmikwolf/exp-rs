# exp-rs

## Build Optimization

For reducing binary size when using this library in embedded systems, you can exclude the libm dependency by building without the default features:

```bash
cargo build --release --no-default-features
```

This reduces the flash memory usage by removing the libm mathematical library dependency. When using this configuration in your application, you have two options:

1. Use `ctx.register_default_math_functions()` - This will register all standard math functions using the Rust standard library implementations
2. Register only the specific math functions you need for further binary size optimization

Note that option 1 is only suitable for environments where the Rust standard library is available, and not for `no_std` environments.

[![Crates.io](https://img.shields.io/crates/v/exp-rs.svg)](https://crates.io/crates/exp-rs)
[![Documentation](https://docs.rs/exp-rs/badge.svg)](https://docs.rs/exp-rs)
[![CI](https://github.com/cosmikwolf/exp-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/cosmikwolf/exp-rs/actions/workflows/rust.yml)
[![Coverage Status](https://coveralls.io/repos/github/cosmikwolf/exp-rs/badge.svg?branch=master)](https://coveralls.io/github/cosmikwolf/exp-rs?branch=master)
![](https://img.shields.io/crates/l/json.svg)
[![no_std](https://img.shields.io/badge/no__std-yes-success)](https://docs.rust-embedded.org/book/intro/no-std.html)

exp-rs ([github.com/cosmikwolf/exp-rs](https://github.com/cosmikwolf/exp-rs)) is a tiny recursive descent expression parser, compiler, and evaluation engine for math expressions.

A C header is generated automatically for FFI usage via `cbindgen`.

This project was inspired by [tinyexpr-rs](https://github.com/kondrak/tinyexpr-rs) by Krzysztof Kondrak, which is itself a port of [TinyExpr](https://github.com/codeplea/tinyexpr) by codeplea.

The function grammar of [tinyexpr-plusplus](https://github.com/Blake-Madden/tinyexpr-plusplus) was used to make it a compatible replacement.

**exp-rs is a `no_std` crate** and is designed to be compatible with embedded systems and environments where the Rust standard library is not available.

[Documentation](https://docs.rs/exp-rs)

## Features

- Parse and evaluate mathematical expressions
- Support for variables, constants, and functions
- Array access with `array[index]` syntax
- Attribute access with `object.attribute` syntax
- Custom function registration (both native Rust functions and expression-based functions)
- No external dependencies for core functionality
- No-std compatible
- Configurable precision with f32 (single-precision) or f64 (double-precision) modes

## Grammar

exp-rs supports a superset of the original TinyExpr grammar, closely matching the [tinyexpr++](https://github.com/codeplea/tinyexpr) grammar, including:

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

### Grammar (EBNF-like)

```
<expr>      = <term> { ("," | ";") <term> }
<term>      = <factor> { ("+" | "-") <factor> }
<factor>    = <power> { ("*" | "/" | "%") <power> }
<power>     = <unary> { ("^" | "**") <unary> }
<unary>     = { ("-" | "+" | "~") } <postfix>
<postfix>   = <primary> { ("(" <args> ")" | "[" <expr> "]" | "." <variable>) }
<primary>   = <constant>
            | <variable>
            | <function> "(" <args> ")"
            | <function> <primary>         // Juxtaposition
            | <variable> "[" <expr> "]"
            | <variable> "." <variable>
            | "(" <expr> ")"
<args>      = [ <expr> { ("," | ";") <expr> } ]
```

- Function application by juxtaposition is supported (e.g., `sin x` is equivalent to `sin(x)`).
- Both `,` and `;` are accepted as list separators.
- Multi-character operators are tokenized and parsed correctly.

### Supported Operators

- Arithmetic: `+`, `-`, `*`, `/`, `%`, `^`, `**`
- Comparison: `<`, `>`, `<=`, `>=`, `==`, `!=`, `<>` (native functions only)
- Logical: `&&`, `||` (with short-circuit evaluation)
- Bitwise: `&`, `|`, `~`, `<<`, `>>`, `<<<`, `>>>`
- Comma/semicolon: `,`, `;` (list separator, returns last value)
- Unary: `-`, `+`, `~` (bitwise not)

### Logical Operators and Short-Circuit Evaluation

The logical operators `&&` (AND) and `||` (OR) feature short-circuit evaluation, meaning they only evaluate the right operand when necessary:

- For `&&` (AND): If the left operand evaluates to false (0.0), the right operand is skipped and the result is false (0.0).
- For `||` (OR): If the left operand evaluates to true (non-zero), the right operand is skipped and the result is true (1.0).

This behavior provides several benefits:
1. **Performance**: Avoids unnecessary calculations
2. **Safety**: Can prevent potential errors in the right operand (e.g., division by zero)
3. **Control flow**: Allows conditional execution patterns

Example:
```rust
// If x is zero, the division is never evaluated (avoiding division by zero)
interp("x == 0 || 10 / x > 5", ctx);

// factorial(x) is only called if x > 0
interp("x > 0 && factorial(x) > 100", ctx);
```

Short-circuit logical operators are fully integrated with the recursion detection system to prevent stack overflows in complex expressions with recursive functions.

### Limitations

- Ternary conditional expressions (`condition ? true_expr : false_expr`) are **not** supported.
- Locale-dependent separator (comma/semicolon) is always accepted; locale configuration is not yet implemented.
- Feature flags for optional grammar features are not yet available.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
exp-rs = "0.1"
```

### Floating-Point Precision

By default, `exp-rs` uses 64-bit floating point (double precision) for calculations. You can configure the precision using feature flags:

```toml
# Use default 64-bit precision (double)
exp-rs = "0.1"

# Use 32-bit precision (float)
exp-rs = { version = "0.1", features = ["f32"] }
```

The f64 mode is the default when f32 is not specified.

### Custom Math Implementations

For embedded systems, you can disable the libm dependency to reduce binary size and provide your own math function implementations:

```toml
# Disable libm dependency
exp-rs = { version = "0.1", default-features = false }
```

When using exp-rs without libm, you have three options:

1. **In a standard Rust environment**: Call `ctx.register_default_math_functions()` after creating your `EvalContext` to automatically register all math functions using the Rust standard library implementations:

```rust
let mut ctx = EvalContext::new();
ctx.register_default_math_functions(); // Register all math functions using std
let result = interp("sin(pi/4)", Some(Rc::new(ctx))).unwrap();
```

2. **In a no_std environment**: Register only the specific functions you need for maximum binary size optimization:

```rust
let mut ctx = EvalContext::new();
// Register just the functions your application requires
ctx.register_native_function("sin", 1, |args| my_custom_sin_implementation(args[0]));
ctx.register_native_function("pow", 2, |args| my_custom_pow_implementation(args[0], args[1]));
```

3. **CMSIS-DSP Integration**: For ARM Cortex-M processors, integrate with CMSIS-DSP for optimized implementations:

```rust
let mut ctx = EvalContext::new();
// Register CMSIS-DSP implementations
ctx.register_native_function("sin", 1, |args| cmsis_dsp_sin(args[0]));
ctx.register_native_function("cos", 1, |args| cmsis_dsp_cos(args[0]));
```

The QEMU tests in the repository include examples of integrating with CMSIS-DSP for optimized math functions on ARM Cortex-M processors.

### Basic Example

```rust
use exp_rs::engine::interp;

fn main() {
    // Simple expression evaluation
    let result = interp("2 + 3 * 4", None).unwrap();
    println!("2 + 3 * 4 = {}", result); // Outputs: 2 + 3 * 4 = 14

    // Using built-in functions
    let result = interp("sin(pi/4) + cos(pi/4)", None).unwrap();
    println!("sin(pi/4) + cos(pi/4) = {}", result); // Approximately 1.414
}
```

### Using Variables and Constants

```rust
use exp_rs::engine::interp;
use exp_rs::context::EvalContext;

fn main() {
    let mut ctx = EvalContext::new();

    // Add variables
    ctx.variables.insert("x".to_string(), 5.0);
    ctx.variables.insert("y".to_string(), 10.0);

    // Add constants
    ctx.constants.insert("FACTOR".to_string(), 2.5);

    // Evaluate expression with variables and constants
    let result = interp("x + y * FACTOR", Some(&mut ctx)).unwrap();
    println!("x + y * FACTOR = {}", result); // Outputs: x + y * FACTOR = 30
}
```

### Custom Functions

```rust
use exp_rs::engine::interp;
use exp_rs::context::EvalContext;

fn main() {
    let mut ctx = EvalContext::new();

    // Register a native function
    ctx.register_native_function("sum", 3, |args| {
        args.iter().sum()
    });

    // Register an expression function
    ctx.register_expression_function(
        "hypotenuse",
        &["a", "b"],
        "sqrt(a^2 + b^2)"
    ).unwrap();

    // Use the custom functions
    let result1 = interp("sum(1, 2, 3)", Some(&mut ctx)).unwrap();
    println!("sum(1, 2, 3) = {}", result1); // Outputs: sum(1, 2, 3) = 6

    let result2 = interp("hypotenuse(3, 4)", Some(&mut ctx)).unwrap();
    println!("hypotenuse(3, 4) = {}", result2); // Outputs: hypotenuse(3, 4) = 5
}
```

### Arrays and Attributes

```rust
use exp_rs::engine::interp;
use exp_rs::context::EvalContext;
use std::collections::BTreeMap;

fn main() {
    let mut ctx = EvalContext::new();

    // Add an array
    ctx.arrays.insert("data".to_string(), vec![10.0, 20.0, 30.0, 40.0, 50.0]);

    // Add an object with attributes
    let mut point = BTreeMap::new();
    point.insert("x".to_string(), 3.0);
    point.insert("y".to_string(), 4.0);
    ctx.attributes.insert("point".to_string(), point);

    // Access array elements
    let result1 = interp("data[2]", Some(&mut ctx)).unwrap();
    println!("data[2] = {}", result1); // Outputs: data[2] = 30

    // Access attributes
    let result2 = interp("point.x + point.y", Some(&mut ctx)).unwrap();
    println!("point.x + point.y = {}", result2); // Outputs: point.x + point.y = 7

    // Combine array and attribute access in expressions
    let result3 = interp("sqrt(point.x^2 + point.y^2) + data[0]", Some(&mut ctx)).unwrap();
    println!("sqrt(point.x^2 + point.y^2) + data[0] = {}", result3); // Outputs: 15
}
```

## Supported Operators

- Addition: `+`
- Subtraction: `-`
- Multiplication: `*`
- Division: `/`
- Modulo: `%`
- Power: `^`
- Unary minus: `-`
- Comma operator: `,` (returns the last value)

## Built-in Functions

- Trigonometric: `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`
- Hyperbolic: `sinh`, `cosh`, `tanh`
- Exponential/Logarithmic: `exp`, `log`, `log10`, `ln`
- Power/Root: `sqrt`, `pow`
- Rounding: `ceil`, `floor`
- Comparison: `max`, `min`
- Misc: `abs`, `sign`

### Disabling Built-in Functions

You can disable the built-in math functions by not including the `libm` feature flag (which is enabled by default). This significantly reduces binary size by eliminating the entire libm dependency:

```toml
[dependencies]
exp-rs = { version = "0.1", default-features = false }
```

Benefits of disabling libm:
- Reduced binary size (potentially saves over 1MB in flash usage)
- More control over which math functions are included
- Ability to provide custom implementations optimized for your target platform
- Option to use platform-specific libraries like CMSIS-DSP on ARM Cortex-M

See the [Custom Math Implementations](#custom-math-implementations) section above for details on how to provide your own math function implementations when libm is disabled.

## Constants

- `pi`: 3.14159... (π)
- `e`: 2.71828... (Euler's number)

## C FFI and Header Generation

If you want to use `exp-rs` from C or other languages, a C header file is automatically generated during the build process using [`cbindgen`](https://github.com/mozilla/cbindgen`).
This header exposes a simple C API for evaluating expressions.

After running `cargo build`, the generated header file can be found at:

```
include/exp_rs.h
```

You can copy this header to your C project and link against the generated static or dynamic library.

### Example C usage

```c
#include "exp_rs.h"

int main() {
    double result = exp_rs_eval("2+2*2");
    printf("%f\n", result); // prints "6.000000"
    return 0;
}
```

## Code Coverage

To check test coverage, install [cargo-tarpaulin](https://github.com/xd009642/tarpaulin):

```fish
cargo install cargo-tarpaulin
```

Then run:

```fish
cargo tarpaulin --workspace --all-features
```

## Build instructions

The build script (`build.rs`) will automatically generate a C header file for FFI usage.

### Cargo Build

```bash
cargo build
cargo test
cargo run --example basic
```

### Meson Build

The project can also be integrated into Meson build systems using the provided `meson.build` file:

```bash
# Configure with default options
meson setup build

# Configure for QEMU testing
meson setup build-qemu --cross-file qemu_test/qemu_harness/arm-cortex-m7-qemu.ini -Dbuild_target=qemu_tests

# Build the project
meson compile -C build

# Run tests (when using QEMU configuration)
meson test -C build-qemu
```

### QEMU Tests

The repository includes a script to run tests in QEMU emulation:

```bash
# Run all QEMU tests with default settings (f32 mode)
./run_qemu_tests.sh

# Run with verbose output
./run_qemu_tests.sh --verbose

# Run a specific test
./run_qemu_tests.sh --test test_name

# Run with f64 (double precision) support
./run_qemu_tests.sh --mode f64

# Clean build before running tests
./run_qemu_tests.sh --clean

# Show help
./run_qemu_tests.sh --help
```

## Project History & Attribution

exp-rs began as a fork of [tinyexpr-rs](https://github.com/kondrak/tinyexpr-rs) by Krzysztof Kondrak, which itself was a port of the [TinyExpr](https://github.com/codeplea/tinyexpr) C library by Lewis Van Winkle (codeplea). As the functionality expanded beyond the scope of the original TinyExpr, it evolved into a new project with additional features inspired by [tinyexpr-plusplus](https://github.com/Blake-Madden/tinyexpr-plusplus) by Blake Madden.

The project has grown to include:
- Support for a wider range of operators
- Array and attribute access
- Juxtaposition support for function calls
- Support for ARM Cortex-M with CMSIS-DSP integration
- Configurable precision with f32/f64 modes
- Comprehensive cbindgen generated FFI for C integration

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
