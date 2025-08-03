#!/bin/bash

set -e

BUILD_DIR="target/meson"
CLEAN_BUILD=0
VERBOSE=0
TEST_NAME=""
FLOAT_MODE="f64" # Default to f64 mode
LIST_TESTS=0
TEST_TARGET="native" # Default to native tests

show_help() {
	echo "Usage: $0 [options]"
	echo "Run tests for exp-rs"
	echo ""
	echo "Options:"
	echo "  --native          Run native C tests (default)"
	echo "  --qemu            Run QEMU embedded tests"
	echo "  -c, --clean       Clean the build directory before building"
	echo "  -v, --verbose     Run tests with verbose output"
	echo "  -t, --test NAME   Run a specific test by name"
	echo "  -m, --mode MODE   Float mode: f32 or f64 (default: f64)"
	echo "  -l, --list        List all available tests for the selected target"
	echo "  -h, --help        Show this help message"
}

# Parse command-line arguments
while [ "$#" -gt 0 ]; do
	case "$1" in
	--native)
		TEST_TARGET="native"
		shift
		;;
	--qemu)
		TEST_TARGET="qemu"
		shift
		;;
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
	-l | --list)
		LIST_TESTS=1
		shift
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

# Function to check if reconfiguration is needed
check_reconfigure() {
	local current_use_f32
	local current_test_native
	local current_qemu_tests
	local expected_use_f32
	local expected_test_native
	local expected_qemu_tests
	local needs_reconfigure=0
	
	# Get current configuration - meson configure shows values in the second column
	# The output format is like: "  use_f32  true  Enable 32-bit..."
	current_use_f32=$(cd "$BUILD_DIR" && meson configure | grep -E "^\s*use_f32" | awk '{print $2}')
	current_test_native=$(cd "$BUILD_DIR" && meson configure | grep -E "^\s*test_native" | awk '{print $2}')
	current_qemu_tests=$(cd "$BUILD_DIR" && meson configure | grep -E "^\s*enable_exprs_qemu_tests" | awk '{print $2}')
	
	# Determine expected values
	if [ "$FLOAT_MODE" = "f32" ]; then
		expected_use_f32="true"
	else
		expected_use_f32="false"
	fi
	
	if [ "$TEST_TARGET" = "native" ]; then
		expected_test_native="true"
		expected_qemu_tests="false"
	else
		expected_test_native="false"
		expected_qemu_tests="true"
	fi
	
	# Check if reconfiguration is needed
	if [ "$current_use_f32" != "$expected_use_f32" ]; then
		echo "Float mode changed: current=$current_use_f32, expected=$expected_use_f32"
		needs_reconfigure=1
	fi
	
	if [ "$current_test_native" != "$expected_test_native" ]; then
		echo "Test target changed: native tests current=$current_test_native, expected=$expected_test_native"
		needs_reconfigure=1
	fi
	
	if [ "$current_qemu_tests" != "$expected_qemu_tests" ]; then
		echo "Test target changed: QEMU tests current=$current_qemu_tests, expected=$expected_qemu_tests"
		needs_reconfigure=1
	fi
	
	if [ "$needs_reconfigure" -eq 1 ]; then
		echo ""
		echo "Meson configuration needs to be updated for your selected options."
		echo "This requires a clean reconfiguration."
		echo "Press Enter to proceed, or Ctrl-C to exit."
		read -r
		return 0
	else
		return 1
	fi
}

# Clean build if requested
if [ "$CLEAN_BUILD" -eq 1 ]; then
	echo "Cleaning build directory..."
	rm -rf "$BUILD_DIR"
	cargo clean
fi

# Setup appropriate meson configuration
setup_meson() {
	local reconfigure=false
	if [ "$1" = "--reconfigure" ]; then
		reconfigure=true
	fi
	
	local meson_args=()
	
	# Float mode
	if [ "$FLOAT_MODE" = "f32" ]; then
		meson_args+=("-Duse_f32=true")
	else
		meson_args+=("-Duse_f32=false")
	fi
	
	# Test target
	if [ "$TEST_TARGET" = "native" ]; then
		meson_args+=("-Dtest_native=true")
		meson_args+=("-Denable_exprs_qemu_tests=false")
	else
		meson_args+=("--cross-file=qemu_test/qemu_harness/arm-cortex-m7-qemu.ini")
		meson_args+=("-Dtest_native=false")
		meson_args+=("-Denable_exprs_qemu_tests=true")
	fi
	
	if [ "$reconfigure" = true ]; then
		meson setup --reconfigure "$BUILD_DIR" "${meson_args[@]}"
	else
		meson setup "$BUILD_DIR" "${meson_args[@]}"
	fi
}

# Check if build directory exists
if [ ! -d "$BUILD_DIR" ]; then
	echo "Build directory not found. Setting up meson build..."
	setup_meson
else
	# Check if reconfiguration is needed
	if check_reconfigure; then
		echo "Cleaning build directory for reconfiguration..."
		rm -rf "$BUILD_DIR"
		echo "Setting up meson build with new configuration..."
		setup_meson
	fi
fi

# If list tests is requested, show available tests and exit
if [ "$LIST_TESTS" -eq 1 ]; then
	echo "Available tests for $TEST_TARGET target in $FLOAT_MODE mode:"
	echo "================================================="
	meson test -C "$BUILD_DIR" --list | while read -r test; do
		echo "  $test"
	done
	exit 0
fi

# Build the Rust library first
echo "Building Rust library..."
if [ "$FLOAT_MODE" = "f32" ]; then
	cargo build --release --no-default-features --features="f32"
else
	cargo build --release # f64 mode (default)
fi

# Compile the tests
echo "Compiling $TEST_TARGET tests..."
meson compile -C "$BUILD_DIR"

# Run the tests
echo "Running $TEST_TARGET tests in $FLOAT_MODE mode..."
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

echo "Tests completed ($TEST_TARGET target, $FLOAT_MODE mode)"