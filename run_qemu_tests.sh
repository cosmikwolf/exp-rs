#\!/bin/bash

set -e

BUILD_DIR="target/meson"
CLEAN_BUILD=0
VERBOSE=0
TEST_NAME=""
FLOAT_MODE="f32" # Default to f32 mode

show_help() {
	echo "Usage: $0 [options]"
	echo "Run QEMU tests for exp-rs without cleaning the build by default"
	echo ""
	echo "Options:"
	echo "  -c, --clean       Clean the build directory before building"
	echo "  -v, --verbose     Run tests with verbose output"
	echo "  -t, --test NAME   Run a specific test by name"
	echo "  -m, --mode MODE   Float mode: f32 or f64 (default: f32)"
	echo "  -h, --help        Show this help message"
}

# Parse command-line arguments
while [ "$#" -gt 0 ]; do
	case "$1" in
	-c | --clean)
		CLEAN_BUILD=1
		shift
		;;
	-v | --verbose)
		VERBOSE=1
		shift
		;;
	-t | --test)
		if [ -n "$2" ]; then
			TEST_NAME="$2"
			shift 2
		else
			echo "Error: --test requires a test name"
			exit 1
		fi
		;;
	-m | --mode)
		if [ -n "$2" ]; then
			if [ "$2" = "f32" ] || [ "$2" = "f64" ]; then
				FLOAT_MODE="$2"
				shift 2
			else
				echo "Error: --mode must be either f32 or f64"
				exit 1
			fi
		else
			echo "Error: --mode requires a value (f32 or f64)"
			exit 1
		fi
		;;
	-h | --help)
		show_help
		exit 0
		;;
	*)
		echo "Unknown option: $1"
		show_help
		exit 1
		;;
	esac
done

# Clean build if requested
if [ "$CLEAN_BUILD" -eq 1 ]; then
	echo "Cleaning build directory..."
	rm -rf "$BUILD_DIR"
	cargo clean
fi

# Make sure Rust library is built with the correct float mode
echo "Building Rust library in $FLOAT_MODE mode..."
if [ "$FLOAT_MODE" = "f32" ]; then
	cargo build --no-default-features --features="f32"
else
	cargo build # f64 mode (default)
fi

# Setup Meson build for QEMU tests
echo "Setting up QEMU test build with $FLOAT_MODE mode..."
if [ "$FLOAT_MODE" = "f32" ]; then
	meson setup "$BUILD_DIR" --cross-file=qemu_test/qemu_harness/arm-cortex-m7-qemu.ini -Denable_f64=false -Denable_exprs_qemu_tests=true
else
	meson setup "$BUILD_DIR" --cross-file=qemu_test/qemu_harness/arm-cortex-m7-qemu.ini -Denable_f64=true -Denable_exprs_qemu_tests=true
fi

# Compile the tests
echo "Compiling QEMU tests..."
meson compile -C "$BUILD_DIR"

# Run the tests
echo "Running QEMU tests..."
if [ -n "$TEST_NAME" ]; then
	# Run specific test if name provided
	echo "Running test: $TEST_NAME"
	if [ "$VERBOSE" -eq 1 ]; then
		meson test -C "$BUILD_DIR" "$TEST_NAME" -v
	else
		meson test -C "$BUILD_DIR" "$TEST_NAME"
	fi
else
	# Run all tests
	if [ "$VERBOSE" -eq 1 ]; then
		meson test -C "$BUILD_DIR" -v
	else
		meson test -C "$BUILD_DIR"
	fi
fi

echo "QEMU tests completed in $FLOAT_MODE mode"
