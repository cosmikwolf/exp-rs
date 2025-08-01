#!/bin/bash

# Compile the arena integration test

set -e  # Exit on error

echo "Compiling arena integration test..."

# Build the Rust library first to ensure exp_rs.h is generated
echo "Building Rust library..."
cargo build --release

# Compile the C test
clang -O2 -Wall -Wextra \
    -I../include \
    -L../target/release \
    -lexp_rs \
    test_arena_integration.c \
    -o test_arena_integration \
    -lm

echo "✓ Compilation successful"
echo "Run with: ./test_arena_integration"