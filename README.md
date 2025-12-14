# exp-rs

[![Crates.io](https://img.shields.io/crates/v/exp-rs.svg)](https://crates.io/crates/exp-rs)
[![Documentation](https://docs.rs/exp-rs/badge.svg)](https://docs.rs/exp-rs)
[![CI](https://github.com/cosmikwolf/exp-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/cosmikwolf/exp-rs/actions/workflows/rust.yml)
[![Coverage Status](https://coveralls.io/repos/github/cosmikwolf/exp-rs/badge.svg?branch=master)](https://coveralls.io/github/cosmikwolf/exp-rs?branch=master)
![](https://img.shields.io/crates/l/json.svg)
[![no_std](https://img.shields.io/badge/no__std-yes-success)](https://docs.rust-embedded.org/book/intro/no-std.html)

A tiny, `no_std` Pratt expression parser and evaluator for embedded systems.

## Key Features

- **Pratt parser** for minimal stack depth—handles deep nesting on embedded stacks
- **Arena allocation** for bounded memory and zero-allocation evaluation after setup
- **no_std compatible** with configurable f32/f64 precision
- Variables, constants, arrays, attributes, and custom functions
- C FFI with auto-generated headers via cbindgen

## Installation

```toml
[dependencies]
exp-rs = "0.1"

# Or with f32 precision
exp-rs = { version = "0.1", features = ["f32"] }

# Or without libm (provide your own math functions)
exp-rs = { version = "0.1", default-features = false }
```

## Quick Example

```rust
use exp_rs::Expression;
use bumpalo::Bump;

let arena = Bump::new();
let result = Expression::eval_simple("2 + 3 * 4", &arena).unwrap();
assert_eq!(result, 14.0);
```

For the full API including parameters, batch evaluation, custom functions, and more, see the [documentation](https://docs.rs/exp-rs).

## C FFI

A C header is automatically generated during build via cbindgen:

```c
#include "exp_rs.h"

int main() {
    double result = exp_rs_eval("2+2*2");
    printf("%f\n", result); // prints "6.000000"
    return 0;
}
```

The header is generated at `include/exp_rs.h` after running `cargo build`.

## Build Instructions

```bash
cargo build
cargo test
```

### Meson Build

```bash
meson setup build
meson compile -C build
```

### QEMU Tests (ARM Cortex-M)

```bash
# Run QEMU embedded tests
./run_tests.sh --qemu -v

# Run with allocation tracking
./run_tests.sh --qemu -a system --track-allocs -v -c

# Run native C tests
./run_tests.sh --native -v

# See all options
./run_tests.sh --help
```

## Code Coverage

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --workspace
```

## Project History

exp-rs began as a fork of [tinyexpr-rs](https://github.com/kondrak/tinyexpr-rs) by Krzysztof Kondrak, which was a port of [TinyExpr](https://github.com/codeplea/tinyexpr) by Lewis Van Winkle. The grammar is based on [tinyexpr-plusplus](https://github.com/Blake-Madden/tinyexpr-plusplus) by Blake Madden.

Key differences from tinyexpr:
- Pratt parser (vs recursive descent) for shallower call stacks
- Arena allocation for predictable memory usage
- Extended operator set and short-circuit logical operators

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
