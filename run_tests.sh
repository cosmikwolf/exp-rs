#!/bin/bash

set -e

BUILD_DIR="target/meson"
CLEAN_BUILD=0
VERBOSE=0
TEST_NAME=""
FLOAT_MODE="f64" # Default to f64 mode
LIST_TESTS=0
TEST_TARGET="native"    # Default to native tests
ALLOCATOR_MODE="system" # Default to system allocator for native tests
ALLOC_TRACKING=0        # Default to no detailed allocation tracking

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
	echo "  -a, --allocator MODE  Allocator mode for native tests: system or custom (default: system)"
	echo "                        system = uses standard malloc with Rust tracking"
	echo "                        custom = uses TlsfHeap with C-side tracking"
	echo "  --track-allocs        Enable detailed allocation tracking with caller information"
	echo "                        (useful for debugging memory leaks)"
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
	-a | --allocator)
		if [ -n "$2" ]; then
			if [ "$2" = "system" ] || [ "$2" = "custom" ]; then
				ALLOCATOR_MODE="$2"
				shift 2
			else
				echo "Error: --allocator must be either system or custom"
				exit 1
			fi
		else
			echo "Error: --allocator requires a value (system or custom)"
			exit 1
		fi
		;;
	--track-allocs)
		ALLOC_TRACKING=1
		shift
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
	local current_custom_alloc
	local current_alloc_tracking
	local expected_use_f32
	local expected_test_native
	local expected_qemu_tests
	local expected_custom_alloc
	local expected_alloc_tracking
	local needs_reconfigure=0

	# Get current configuration - meson configure shows values in the second column
	# The output format is like: "  use_f32  true  Enable 32-bit..."
	if ! cd "$BUILD_DIR" 2>/dev/null; then
		echo "Error: Cannot access build directory $BUILD_DIR"
		return 0 # Force reconfigure
	fi

	current_use_f32=$(meson configure 2>/dev/null | grep -E "^\s*use_f32\s" | awk '{print $2}' | head -1)
	current_test_native=$(meson configure 2>/dev/null | grep -E "^\s*test_native\s" | awk '{print $2}' | head -1)
	current_qemu_tests=$(meson configure 2>/dev/null | grep -E "^\s*enable_exprs_qemu_tests\s" | awk '{print $2}' | head -1)
	current_custom_alloc=$(meson configure 2>/dev/null | grep -E "^\s*custom_cbindgen_alloc\s" | awk '{print $2}' | head -1)
	current_alloc_tracking=$(meson configure 2>/dev/null | grep -E "^\s*alloc_tracking\s" | awk '{print $2}' | head -1)

	cd - >/dev/null

	# If any values are empty, force reconfiguration
	if [ -z "$current_use_f32" ] || [ -z "$current_test_native" ] || [ -z "$current_qemu_tests" ] || [ -z "$current_custom_alloc" ] || [ -z "$current_alloc_tracking" ]; then
		echo "Warning: Could not read current meson configuration. Forcing reconfigure."
		return 0 # Force reconfigure
	fi

	# Determine expected values
	if [ "$FLOAT_MODE" = "f32" ]; then
		expected_use_f32="true"
	else
		expected_use_f32="false"
	fi

	if [ "$TEST_TARGET" = "native" ]; then
		expected_test_native="true"
		expected_qemu_tests="false"
		# Set custom allocator based on allocator mode for native tests
		if [ "$ALLOCATOR_MODE" = "custom" ]; then
			expected_custom_alloc="true"
		else
			expected_custom_alloc="false"
		fi
	else
		expected_test_native="false"
		expected_qemu_tests="true"
		# QEMU tests always use custom allocator
		expected_custom_alloc="true"
	fi

	# Set expected alloc_tracking value
	if [ "$ALLOC_TRACKING" -eq 1 ]; then
		expected_alloc_tracking="true"
	else
		expected_alloc_tracking="false"
	fi

	# Debug output for troubleshooting
	if [ "$VERBOSE" -eq 1 ]; then
		echo "DEBUG: current_use_f32='$current_use_f32' expected_use_f32='$expected_use_f32'"
		echo "DEBUG: current_test_native='$current_test_native' expected_test_native='$expected_test_native'"
		echo "DEBUG: current_qemu_tests='$current_qemu_tests' expected_qemu_tests='$expected_qemu_tests'"
		echo "DEBUG: current_custom_alloc='$current_custom_alloc' expected_custom_alloc='$expected_custom_alloc'"
		echo "DEBUG: current_alloc_tracking='$current_alloc_tracking' expected_alloc_tracking='$expected_alloc_tracking'"
		echo "DEBUG: allocator_mode='$ALLOCATOR_MODE'"
	fi

	# Check if reconfiguration is needed
	if [ "$current_use_f32" != "$expected_use_f32" ]; then
		echo "Float mode changed: current='$current_use_f32', expected='$expected_use_f32'"
		needs_reconfigure=1
	fi

	if [ "$current_test_native" != "$expected_test_native" ]; then
		echo "Test target changed: native tests current='$current_test_native', expected='$expected_test_native'"
		needs_reconfigure=1
	fi

	if [ "$current_qemu_tests" != "$expected_qemu_tests" ]; then
		echo "Test target changed: QEMU tests current='$current_qemu_tests', expected='$expected_qemu_tests'"
		needs_reconfigure=1
	fi

	if [ "$current_custom_alloc" != "$expected_custom_alloc" ]; then
		echo "Allocator mode changed: current='$current_custom_alloc', expected='$expected_custom_alloc'"
		needs_reconfigure=1
	fi

	if [ "$current_alloc_tracking" != "$expected_alloc_tracking" ]; then
		echo "Allocation tracking changed: current='$current_alloc_tracking', expected='$expected_alloc_tracking'"
		needs_reconfigure=1
	fi

	if [ "$needs_reconfigure" -eq 1 ]; then
		echo ""
		echo "Meson configuration needs to be updated for your selected options."
		echo "This requires a clean reconfiguration."
		echo "Press Enter to proceed, or Ctrl-C to exit."
		read -r </dev/tty
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
		meson_args+=("-D" "test_native=true")
		meson_args+=("-D" "enable_exprs_qemu_tests=false")
		# Set custom allocator based on allocator mode
		if [ "$ALLOCATOR_MODE" = "custom" ]; then
			meson_args+=("-D" "custom_cbindgen_alloc=true")
		else
			meson_args+=("-D" "custom_cbindgen_alloc=false")
		fi
	else
		meson_args+=("--cross-file=qemu_test/qemu_harness/arm-cortex-m7-qemu.ini")
		meson_args+=("-D" "test_native=false")
		meson_args+=("-D" "custom_cbindgen_alloc=true")
		meson_args+=("-D" "enable_exprs_qemu_tests=true")
	fi

	# Add allocation tracking option for both native and QEMU builds
	if [ "$ALLOC_TRACKING" -eq 1 ]; then
		meson_args+=("-D" "alloc_tracking=true")
	else
		meson_args+=("-D" "alloc_tracking=false")
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
	if [ "$TEST_TARGET" = "native" ]; then
		if [ "$ALLOC_TRACKING" -eq 1 ]; then
			echo "Available tests for $TEST_TARGET target in $FLOAT_MODE mode with $ALLOCATOR_MODE allocator (alloc tracking enabled):"
		else
			echo "Available tests for $TEST_TARGET target in $FLOAT_MODE mode with $ALLOCATOR_MODE allocator:"
		fi
	else
		if [ "$ALLOC_TRACKING" -eq 1 ]; then
			echo "Available tests for $TEST_TARGET target in $FLOAT_MODE mode (alloc tracking enabled):"
		else
			echo "Available tests for $TEST_TARGET target in $FLOAT_MODE mode:"
		fi
	fi
	echo "================================================="
	meson test -C "$BUILD_DIR" --list | while read -r test; do
		echo "  $test"
	done
	exit 0
fi

# Build the Rust library first
echo "Building Rust library..."
RUST_FEATURES=""
if [ "$FLOAT_MODE" = "f32" ]; then
	RUST_FEATURES="f32"
fi
if [ "$ALLOC_TRACKING" -eq 1 ]; then
	if [ -n "$RUST_FEATURES" ]; then
		RUST_FEATURES="$RUST_FEATURES,alloc_tracking"
	else
		RUST_FEATURES="alloc_tracking"
	fi
fi

# if [ -n "$RUST_FEATURES" ]; then
# 	echo "Building with features: $RUST_FEATURES"
# 	cargo build --release --features="$RUST_FEATURES"
# else
# 	cargo build --release
# fi

# Compile the tests
if [ "$TEST_TARGET" = "native" ]; then
	if [ "$ALLOC_TRACKING" -eq 1 ]; then
		echo "Compiling $TEST_TARGET tests with $ALLOCATOR_MODE allocator (alloc tracking enabled)..."
	else
		echo "Compiling $TEST_TARGET tests with $ALLOCATOR_MODE allocator..."
	fi
else
	if [ "$ALLOC_TRACKING" -eq 1 ]; then
		echo "Compiling $TEST_TARGET tests (alloc tracking enabled)..."
	else
		echo "Compiling $TEST_TARGET tests..."
	fi
fi
meson compile -C "$BUILD_DIR"

# Run the tests
if [ "$TEST_TARGET" = "native" ]; then
	if [ "$ALLOC_TRACKING" -eq 1 ]; then
		echo "Running $TEST_TARGET tests in $FLOAT_MODE mode with $ALLOCATOR_MODE allocator (alloc tracking enabled)..."
	else
		echo "Running $TEST_TARGET tests in $FLOAT_MODE mode with $ALLOCATOR_MODE allocator..."
	fi
else
	if [ "$ALLOC_TRACKING" -eq 1 ]; then
		echo "Running $TEST_TARGET tests in $FLOAT_MODE mode (alloc tracking enabled)..."
	else
		echo "Running $TEST_TARGET tests in $FLOAT_MODE mode..."
	fi
fi
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

if [ "$TEST_TARGET" = "native" ]; then
	if [ "$ALLOC_TRACKING" -eq 1 ]; then
		echo "Tests completed ($TEST_TARGET target, $FLOAT_MODE mode, $ALLOCATOR_MODE allocator, alloc tracking enabled)"
	else
		echo "Tests completed ($TEST_TARGET target, $FLOAT_MODE mode, $ALLOCATOR_MODE allocator)"
	fi
else
	if [ "$ALLOC_TRACKING" -eq 1 ]; then
		echo "Tests completed ($TEST_TARGET target, $FLOAT_MODE mode, alloc tracking enabled)"
	else
		echo "Tests completed ($TEST_TARGET target, $FLOAT_MODE mode)"
	fi
fi
