#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <math.h>
#include "qemu_test_harness.h"

// Include the generated header
#include "../include/exp_rs.h"

// Define common types and utilities for our tests
#if defined(DEF_USE_F32) || (defined(USE_F32) && !defined(USE_F64))
typedef float real_t;
#define SIN sinf
#define COS cosf
#define SQRT sqrtf
#define FABS fabsf
#define TEST_NAME "F32"
#define FORMAT_SPEC "%.6f"

#elif defined(DEF_USE_F64) || defined(USE_F64)
typedef double real_t;
#define SIN sin
#define COS cos
#define SQRT sqrt
#define FABS fabs
#define TEST_NAME "F64"
#define FORMAT_SPEC "%.12f"

#else
#error "Neither USE_F32 nor USE_F64 is defined."
#endif

// C implementation of a custom math function
Real custom_add(const Real* args, uintptr_t argc) {
    if (argc != 2) {
        qemu_print("Invalid argument count for custom_add\n");
        return 0.0;
    }
    
    // Add the two arguments and return the result
    Real result = args[0] + args[1];
    qemu_printf("custom_add: %.2f + %.2f = %.2f\n", args[0], args[1], result);
    return result;
}

// C implementation of a custom power function
Real custom_power(const Real* args, uintptr_t argc) {
    if (argc != 2) {
        qemu_print("Invalid argument count for custom_power\n");
        return 0.0;
    }
    
    // First argument is base, second is exponent
    Real base = args[0];
    Real exponent = args[1];
    
    // Handle special cases
    if (exponent == 0.0) {
        return 1.0;
    }
    
    // Simple integer power implementation for demonstration
    Real result = 1.0;
    int exp_int = (int)exponent;
    
    // Only handle positive integer exponents for simplicity
    if (exp_int > 0 && exp_int == exponent) {
        for (int i = 0; i < exp_int; i++) {
            result *= base;
        }
        qemu_printf("custom_power: %.2f^%d = %.2f\n", base, exp_int, result);
        return result;
    }
    
    // For non-integer or negative exponents, print a message and return 0
    qemu_print("custom_power: Only positive integer exponents supported in this example\n");
    return 0.0;
}

// Test using the native function registration
static test_result_t test_native_functions(void) {
    qemu_printf("Testing exp_rs native function registration with %s mode\n", TEST_NAME);
    
    // Create a new evaluation context
    struct EvalContextOpaque* ctx = exp_rs_context_new();
    if (ctx == NULL) {
        qemu_print("Failed to create evaluation context\n");
        return TEST_FAIL;
    }
    
    // Register our custom C functions
    struct EvalResult result = exp_rs_context_register_native_function(ctx, "c_add", 2, custom_add);
    if (result.status != 0) {
        qemu_print("Failed to register c_add function: ");
        if (result.error) {
            qemu_print(result.error);
            exp_rs_free_error((char*)result.error);
        }
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    result = exp_rs_context_register_native_function(ctx, "c_power", 2, custom_power);
    if (result.status != 0) {
        qemu_print("Failed to register c_power function: ");
        if (result.error) {
            qemu_print(result.error);
            exp_rs_free_error((char*)result.error);
        }
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Set some parameters
    exp_rs_context_set_parameter(ctx, "x", 5.0);
    exp_rs_context_set_parameter(ctx, "y", 3.0);
    
    // First test a basic expression to ensure the context is working
    qemu_print("Testing basic expression evaluation with context...\n");
    result = exp_rs_context_eval("x + y", ctx);
    if (result.status != 0) {
        qemu_print("Basic context expression evaluation failed: ");
        if (result.error) {
            qemu_print(result.error);
            exp_rs_free_error((char*)result.error);
        }
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    qemu_printf("Basic expression x + y = %.2f (expected 8.00)\n", result.value);
    if (FABS(result.value - 8.0) > TEST_PRECISION) {
        qemu_print("Basic expression test failed\n");
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Now test our custom native function
    qemu_print("\nTesting custom c_add function...\n");
    result = exp_rs_context_eval("c_add(10, 20)", ctx);
    if (result.status != 0) {
        qemu_print("Native function evaluation failed: ");
        if (result.error) {
            qemu_print(result.error);
            exp_rs_free_error((char*)result.error);
        }
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    qemu_printf("c_add(10, 20) = %.2f (expected 30.00)\n", result.value);
    if (FABS(result.value - 30.0) > TEST_PRECISION) {
        qemu_print("Native function test failed\n");
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Clean up
    exp_rs_context_free(ctx);
    
    qemu_print("Native function test passed!\n");
    return TEST_PASS;
}

static const test_case_t tests[] = {
    {"native_functions", test_native_functions},
};

int main(void) {
    int failed = run_tests(tests, sizeof(tests) / sizeof(tests[0]));
    qemu_exit(failed ? EXIT_FAILURE : EXIT_SUCCESS);
    return failed ? 1 : 0;
}