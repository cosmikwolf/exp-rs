#!/bin/sh

set -e

BUILD_DIR="target/meson"

show_help() {
	echo "Usage: $0 [--build] [--test] [--clean] [--test-qemu]"
	echo "  --build       Run meson setup (if needed) and meson compile"
	echo "  --test        Run meson test"
	echo "  --clean       Run meson clean"
	echo "  --test-qemu   Run only the QEMU test (by test name)"
	echo "  --help        Show this help"
}

DO_BUILD=0
DO_TEST=0
DO_CLEAN=0
DO_TEST_QEMU=0

for arg in "$@"; do
	case "$arg" in
	--build) DO_BUILD=1 ;;
	--test) DO_TEST=1 ;;
	--clean) DO_CLEAN=1 ;;
	--test-qemu) DO_TEST_QEMU=1 ;;
	--help)
		show_help
		exit 0
		;;
	*)
		echo "Unknown argument: $arg"
		show_help
		exit 1
		;;
	esac
done

if [ "$DO_CLEAN" -eq 1 ]; then
	if [ -d "$BUILD_DIR" ]; then
		rm -rf "$BUILD_DIR"
		echo "Build directory cleaned."
	else
		echo "No build directory to clean."
	fi
fi

if [ "$DO_BUILD" -eq 1 ]; then
	meson setup "$BUILD_DIR"
	meson compile -C "$BUILD_DIR"
fi

if [ "$DO_TEST" -eq 1 ]; then
	meson test -C "$BUILD_DIR"
fi

if [ "$DO_TEST_QEMU" -eq 1 ]; then
	meson setup "$BUILD_DIR" --cross-file=qemu_test/qemu_harness/arm-cortex-m7-qemu.ini
	meson compile -C "$BUILD_DIR"
	meson test -C "$BUILD_DIR"
fi

if [ "$DO_BUILD" -eq 0 ] && [ "$DO_TEST" -eq 0 ] && [ "$DO_CLEAN" -eq 0 ] && [ "$DO_TEST_QEMU" -eq 0 ]; then
	show_help
	exit 1
fi
