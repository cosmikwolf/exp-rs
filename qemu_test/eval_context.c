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

// Using the EvalResult struct directly

// Helper to check approximate equality
static int approx_eq(real_t a, real_t b, real_t eps) {
    return FABS(a - b) < eps;
}

// Test setting and getting parameters
test_result_t test_param_set_get() {
    qemu_printf("Testing parameter set/get in %s mode...\n", TEST_NAME);
    
    // Create a new context
    struct EvalContextOpaque* ctx = exp_rs_context_new();
    if (!ctx) {
        qemu_print("Failed to create context\n");
        return TEST_FAIL;
    }
    
    // Set parameters
    real_t a_val = 42.0;
    real_t b_val = 123.5;
    
    exp_rs_context_set_parameter(ctx, "a", a_val);
    exp_rs_context_set_parameter(ctx, "b", b_val);
    
    // Test getting parameters by using them in expressions
    struct EvalResult result_a = exp_rs_context_eval("a", ctx);
    if (result_a.status != 0) {
        qemu_print("Error evaluating 'a'\n");
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    struct EvalResult result_b = exp_rs_context_eval("b", ctx);
    if (result_b.status != 0) {
        qemu_print("Error evaluating 'b'\n");
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Check values
    qemu_printf("a = " FORMAT_SPEC " (expected " FORMAT_SPEC ")\n", result_a.value, a_val);
    qemu_printf("b = " FORMAT_SPEC " (expected " FORMAT_SPEC ")\n", result_b.value, b_val);
    
    if (!approx_eq(result_a.value, a_val, TEST_PRECISION) || 
        !approx_eq(result_b.value, b_val, TEST_PRECISION)) {
        qemu_print("Parameter values don't match\n");
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Clean up
    exp_rs_context_free(ctx);
    
    qemu_print("Parameter set/get test passed\n");
    return TEST_PASS;
}

// Test expression function registration
test_result_t test_expression_function() {
    qemu_printf("Testing expression function in %s mode...\n", TEST_NAME);
    
    // Create a new context
    struct EvalContextOpaque* ctx = exp_rs_context_new();
    if (!ctx) {
        qemu_print("Failed to create context\n");
        return TEST_FAIL;
    }
    
    // Register an expression function
    const char* func_name = "my_func";
    const char* param1_name = "x";
    const char* param2_name = "y";
    const char* params[] = {param1_name, param2_name};
    const char* expr = "x^2 + y^2 + 2*x*y";
    
    int status = exp_rs_context_register_expression_function(
        ctx, func_name, (const char**)params, 2, expr);
    
    if (status != 0) {
        qemu_printf("Failed to register function, status=%d\n", status);
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Set parameters for testing
    exp_rs_context_set_parameter(ctx, "a", 3.0);
    exp_rs_context_set_parameter(ctx, "b", 4.0);
    
    // Test using the function
    struct EvalResult result = exp_rs_context_eval("my_func(a, b)", ctx);
    if (result.status != 0) {
        qemu_print("Error evaluating 'my_func(a, b)'\n");
        if (result.error) {
            qemu_printf("Error: %s\n", result.error);
            exp_rs_free_error((char*)result.error);
        }
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Expected result: (a^2 + b^2 + 2*a*b) = (3^2 + 4^2 + 2*3*4) = 9 + 16 + 24 = 49
    real_t expected = 49.0;
    
    qemu_printf("my_func(3, 4) = " FORMAT_SPEC " (expected " FORMAT_SPEC ")\n", 
                result.value, expected);
    
    if (!approx_eq(result.value, expected, TEST_PRECISION)) {
        qemu_print("Function result doesn't match expected value\n");
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Clean up
    exp_rs_context_free(ctx);
    
    qemu_print("Expression function test passed\n");
    return TEST_PASS;
}

// Test nested functions
test_result_t test_nested_functions() {
    qemu_printf("Testing nested functions in %s mode...\n", TEST_NAME);
    
    // Create a new context
    struct EvalContextOpaque* ctx = exp_rs_context_new();
    if (!ctx) {
        qemu_print("Failed to create context\n");
        return TEST_FAIL;
    }
    
    // Register first function
    const char* func1_name = "squared";
    const char* param1_name = "x";
    const char* params1[] = {param1_name};
    const char* expr1 = "x^2";
    
    int status = exp_rs_context_register_expression_function(
        ctx, func1_name, (const char**)params1, 1, expr1);
    
    if (status != 0) {
        qemu_printf("Failed to register function 1, status=%d\n", status);
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Register second function that uses the first
    const char* func2_name = "sum_of_squares";
    const char* param2a_name = "a";
    const char* param2b_name = "b";
    const char* params2[] = {param2a_name, param2b_name};
    const char* expr2 = "squared(a) + squared(b)";
    
    status = exp_rs_context_register_expression_function(
        ctx, func2_name, (const char**)params2, 2, expr2);
    
    if (status != 0) {
        qemu_printf("Failed to register function 2, status=%d\n", status);
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Set parameters for testing
    exp_rs_context_set_parameter(ctx, "x", 3.0);
    exp_rs_context_set_parameter(ctx, "y", 4.0);
    
    // Test using the nested functions
    struct EvalResult result = exp_rs_context_eval("sum_of_squares(x, y)", ctx);
    if (result.status != 0) {
        qemu_print("Error evaluating 'sum_of_squares(x, y)'\n");
        if (result.error) {
            qemu_printf("Error: %s\n", result.error);
            exp_rs_free_error((char*)result.error);
        }
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Expected result: x^2 + y^2 = 3^2 + 4^2 = 9 + 16 = 25
    real_t expected = 25.0;
    
    qemu_printf("sum_of_squares(3, 4) = " FORMAT_SPEC " (expected " FORMAT_SPEC ")\n", 
                result.value, expected);
    
    if (!approx_eq(result.value, expected, TEST_PRECISION)) {
        qemu_print("Nested function result doesn't match expected value\n");
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Clean up
    exp_rs_context_free(ctx);
    
    qemu_print("Nested functions test passed\n");
    return TEST_PASS;
}

// Main test function
test_result_t test_eval_context() {
    qemu_printf("Testing EvalContext with %s precision\n\n", TEST_NAME);
    
    // Run individual tests
    test_result_t param_result = test_param_set_get();
    if (param_result != TEST_PASS) {
        return param_result;
    }
    
    test_result_t func_result = test_expression_function();
    if (func_result != TEST_PASS) {
        return func_result;
    }
    
    test_result_t nested_result = test_nested_functions();
    if (nested_result != TEST_PASS) {
        return nested_result;
    }
    
    qemu_print("\nAll EvalContext tests passed!\n");
    return TEST_PASS;
}

// Test case definition
static const test_case_t tests[] = {
    {"eval_context", test_eval_context},
};

int main(void) {
    int failed = run_tests(tests, sizeof(tests) / sizeof(tests[0]));
    qemu_exit(failed ? EXIT_FAILURE : EXIT_SUCCESS);
    return failed ? 1 : 0;
}