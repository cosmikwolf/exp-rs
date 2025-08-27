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
#define FABS fabsf
#define TEST_NAME "F32"
#define FORMAT_SPEC "%.6f"
#elif defined(DEF_USE_F64) || defined(USE_F64)
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
    qemu_printf("custom_add: " FORMAT_SPEC " + " FORMAT_SPEC " = " FORMAT_SPEC "\n", args[0], args[1], result);
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
        qemu_printf("custom_power: " FORMAT_SPEC "^%d = " FORMAT_SPEC "\n", base, exp_int, result);
        return result;
    }
    
    // For non-integer or negative exponents, print a message and return 0
    qemu_print("custom_power: Only positive integer exponents supported in this example\n");
    return 0.0;
}

// Test using the native function registration
static test_result_t test_native_functions(void) {
    qemu_printf("Testing native function registration with %s mode\n", TEST_NAME);
    
    // Create a new evaluation context
    struct ExprContext* ctx = expr_context_new();
    if (ctx == NULL) {
        qemu_print("Failed to create evaluation context\n");
        return TEST_FAIL;
    }
    
    // Create batch
    struct ExprBatch* batch = expr_batch_new(8192);
    if (!batch) {
        qemu_print("Failed to create batch\n");
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Register our custom C functions
    int32_t result = expr_context_add_function(ctx, "c_add", 2, custom_add);
    if (result != 0) {
        qemu_printf("Failed to register c_add function, error code: %d\n", result);
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    result = expr_context_add_function(ctx, "c_power", 2, custom_power);
    if (result != 0) {
        qemu_printf("Failed to register c_power function, error code: %d\n", result);
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Add variables to batch
    struct ExprResult var_x = expr_batch_add_variable(batch, "x", 5.0);
    if (var_x.status != 0) {
        qemu_print("Failed to add variable x\n");
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    struct ExprResult var_y = expr_batch_add_variable(batch, "y", 3.0);
    if (var_y.status != 0) {
        qemu_print("Failed to add variable y\n");
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Test custom add function
    qemu_print("Testing custom add function c_add(x, y)...\n");
    struct ExprResult add_expr = expr_batch_add_expression(batch, "c_add(x, y)");
    if (add_expr.status != 0) {
        qemu_print("Failed to add c_add expression\n");
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Test custom power function  
    qemu_print("Testing custom power function c_power(x, y)...\n");
    struct ExprResult power_expr = expr_batch_add_expression(batch, "c_power(x, y)");
    if (power_expr.status != 0) {
        qemu_print("Failed to add c_power expression\n");
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Evaluate all expressions
    int32_t eval_status = expr_batch_evaluate(batch, ctx);
    if (eval_status != 0) {
        qemu_print("Failed to evaluate batch\n");
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Get results
    Real add_result = expr_batch_get_result(batch, add_expr.index);
    Real power_result = expr_batch_get_result(batch, power_expr.index);
    
    qemu_printf("c_add(5, 3) = " FORMAT_SPEC " (expected 8.0)\n", add_result);
    qemu_printf("c_power(5, 3) = " FORMAT_SPEC " (expected 125.0)\n", power_result);
    
    // Check results
    if (FABS(add_result - 8.0) > TEST_PRECISION) {
        qemu_print("Custom add function test failed\n");
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    if (FABS(power_result - 125.0) > TEST_PRECISION) {
        qemu_print("Custom power function test failed\n");
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    qemu_print("Native function tests passed\n");
    
    // Clean up
    expr_batch_free(batch);
    expr_context_free(ctx);
    
    return TEST_PASS;
}

test_result_t test_native_function(void) {
    qemu_printf("Testing native functions with %s precision\n\n", TEST_NAME);

    test_result_t native_result = test_native_functions();
    if (native_result != TEST_PASS) {
        return native_result;
    }

    qemu_print("\nAll native function tests passed!\n");
    return TEST_PASS;
}

// Test case definition
static const test_case_t tests[] = {
    {"native_function", test_native_function},
};

int main(void) {
    int failed = run_tests(tests, sizeof(tests) / sizeof(tests[0]));
    qemu_exit(failed ? EXIT_FAILURE : EXIT_SUCCESS);
    return failed ? 1 : 0;
}