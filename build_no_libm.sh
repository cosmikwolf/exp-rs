#!/bin/bash
# Build script for exp-rs without libm dependency

set -e

# Build without libm for embedded targets
# This should dramatically reduce binary size

echo "Building exp-rs without libm..."
cargo build --release --no-default-features --features="f64,no-builtin-math"

echo "Build complete!"
echo "To use in the main project, make sure to add --features=no-builtin-math to your compilation flags"