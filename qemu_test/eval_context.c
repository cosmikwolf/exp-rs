#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <math.h>
#include "qemu_test_harness.h"

// Include the generated header
#include "../include/exp_rs.h"
#include "register_test_functions.h"

// Define common types and utilities for our tests
#if defined(DEF_USE_F32) || (defined(USE_F32) && !defined(USE_F64))

#define SIN sinf
#define COS cosf
#define SQRT sqrtf
#define FABS fabsf
#define TEST_NAME "F32"
#define FORMAT_SPEC "%.6f"

#elif defined(DEF_USE_F64) || defined(USE_F64)

#define SIN sin
#define COS cos
#define SQRT sqrt
#define FABS fabs
#define TEST_NAME "F64"
#define FORMAT_SPEC "%.12f"

#else
#error "Neither USE_F32 nor USE_F64 is defined."
#endif

// Using the ExprResult struct from new batch-based API

// Helper to check approximate equality
static int approx_eq(Real a, Real b, Real eps) {
    return FABS(a - b) < eps;
}

// Test setting and getting variables using batch
test_result_t test_param_set_get() {
    qemu_printf("Testing variable set/get in %s mode...\n", TEST_NAME);
    
    // Create context and batch
    struct ExprContext* ctx = create_test_context();
    if (!ctx) {
        qemu_print("Failed to create context\n");
        return TEST_FAIL;
    }
    
    struct ExprBatch* batch = expr_batch_new(8192);
    if (!batch) {
        qemu_print("Failed to create batch\n");
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Add variables to batch
    Real a_val = 42.0;
    Real b_val = 123.5;
    
    struct ExprResult set_result_a = expr_batch_add_variable(batch, "a", a_val);
    if (set_result_a.status != 0) {
        qemu_print("Error adding variable 'a'\n");
        qemu_printf("Error: %s\n", set_result_a.error);
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    struct ExprResult set_result_b = expr_batch_add_variable(batch, "b", b_val);
    if (set_result_b.status != 0) {
        qemu_print("Error adding variable 'b'\n");
        qemu_printf("Error: %s\n", set_result_b.error);
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Add expressions to evaluate the variables
    struct ExprResult expr_a = expr_batch_add_expression(batch, "a");
    if (expr_a.status != 0) {
        qemu_print("Error adding expression 'a'\n");
        qemu_printf("Error: %s\n", expr_a.error);
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    struct ExprResult expr_b = expr_batch_add_expression(batch, "b");
    if (expr_b.status != 0) {
        qemu_print("Error adding expression 'b'\n");
        qemu_printf("Error: %s\n", expr_b.error);
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Evaluate the batch
    int32_t eval_status = expr_batch_evaluate(batch, ctx);
    if (eval_status != 0) {
        qemu_print("Error evaluating batch\n");
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Get results
    Real result_a_value = expr_batch_get_result(batch, expr_a.index);
    Real result_b_value = expr_batch_get_result(batch, expr_b.index);
    
    // Check values
    qemu_printf("a = " FORMAT_SPEC " (expected " FORMAT_SPEC ")\n", result_a_value, a_val);
    qemu_printf("b = " FORMAT_SPEC " (expected " FORMAT_SPEC ")\n", result_b_value, b_val);
    
    if (!approx_eq(result_a_value, a_val, TEST_PRECISION) || 
        !approx_eq(result_b_value, b_val, TEST_PRECISION)) {
        qemu_print("Variable values don't match\n");
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Clean up
    expr_batch_free(batch);
    expr_context_free(ctx);
    
    qemu_print("Variable set/get test passed\n");
    return TEST_PASS;
}

// Test expression function registration (now batch-specific)
test_result_t test_expression_function() {
    qemu_printf("Testing expression function in %s mode...\n", TEST_NAME);
    
    // Create context and batch
    struct ExprContext* ctx = create_test_context();
    if (!ctx) {
        qemu_print("Failed to create context\n");
        return TEST_FAIL;
    }
    
    struct ExprBatch* batch = expr_batch_new(8192);
    if (!batch) {
        qemu_print("Failed to create batch\n");
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Register an expression function (batch-specific)
    const char* func_name = "my_func";
    const char* params = "x,y";  // Comma-separated parameter names
    const char* expr = "x^2 + y^2 + 2*x*y";
    
    int32_t reg_result = expr_batch_add_expression_function(
        batch, func_name, params, expr);
    
    if (reg_result != 0) {
        qemu_printf("Failed to register function, error code: %d\n", reg_result);
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Add variables to batch for testing
    struct ExprResult set_result_a = expr_batch_add_variable(batch, "a", 3.0);
    if (set_result_a.status != 0) {
        qemu_print("Error adding variable 'a'\n");
        qemu_printf("Error: %s\n", set_result_a.error);
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    struct ExprResult set_result_b = expr_batch_add_variable(batch, "b", 4.0);
    if (set_result_b.status != 0) {
        qemu_print("Error adding variable 'b'\n");
        qemu_printf("Error: %s\n", set_result_b.error);
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Add expression using the function and evaluate
    struct ExprResult expr_result = expr_batch_add_expression(batch, "my_func(a, b)");
    if (expr_result.status != 0) {
        qemu_print("Error adding expression 'my_func(a, b)'\n");
        qemu_printf("Error: %s\n", expr_result.error);
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Evaluate the batch
    int32_t eval_status = expr_batch_evaluate(batch, ctx);
    if (eval_status != 0) {
        qemu_print("Error evaluating batch\n");
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Get the result
    Real result_value = expr_batch_get_result(batch, expr_result.index);
    
    // Expected result: (a^2 + b^2 + 2*a*b) = (3^2 + 4^2 + 2*3*4) = 9 + 16 + 24 = 49
    Real expected = 49.0;
    
    qemu_printf("my_func(3, 4) = " FORMAT_SPEC " (expected " FORMAT_SPEC ")\n", 
                result_value, expected);
    
    if (!approx_eq(result_value, expected, TEST_PRECISION)) {
        qemu_print("Function result doesn't match expected value\n");
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Clean up
    expr_batch_free(batch);
    expr_context_free(ctx);
    
    qemu_print("Expression function test passed\n");
    return TEST_PASS;
}

// Test nested functions (batch-specific)
test_result_t test_nested_functions() {
    qemu_printf("Testing nested functions in %s mode...\n", TEST_NAME);
    
    // Create context and batch
    struct ExprContext* ctx = create_test_context();
    if (!ctx) {
        qemu_print("Failed to create context\n");
        return TEST_FAIL;
    }
    
    struct ExprBatch* batch = expr_batch_new(8192);
    if (!batch) {
        qemu_print("Failed to create batch\n");
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Register first function (batch-specific)
    const char* func1_name = "squared";
    const char* params1 = "x";  // Comma-separated parameter names
    const char* expr1 = "x^2";
    
    int32_t reg_result1 = expr_batch_add_expression_function(
        batch, func1_name, params1, expr1);
    
    if (reg_result1 != 0) {
        qemu_printf("Failed to register function 1, error code: %d\n", reg_result1);
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Register second function that uses the first
    const char* func2_name = "sum_of_squares";
    const char* params2 = "a,b";  // Comma-separated parameter names
    const char* expr2 = "squared(a) + squared(b)";
    
    int32_t reg_result2 = expr_batch_add_expression_function(
        batch, func2_name, params2, expr2);
    
    if (reg_result2 != 0) {
        qemu_printf("Failed to register function 2, error code: %d\n", reg_result2);
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Add variables to batch for testing
    struct ExprResult set_result_x = expr_batch_add_variable(batch, "x", 3.0);
    if (set_result_x.status != 0) {
        qemu_print("Error adding variable 'x'\n");
        qemu_printf("Error: %s\n", set_result_x.error);
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    struct ExprResult set_result_y = expr_batch_add_variable(batch, "y", 4.0);
    if (set_result_y.status != 0) {
        qemu_print("Error adding variable 'y'\n");
        qemu_printf("Error: %s\n", set_result_y.error);
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Add expression using the nested functions and evaluate
    struct ExprResult expr_result = expr_batch_add_expression(batch, "sum_of_squares(x, y)");
    if (expr_result.status != 0) {
        qemu_print("Error adding expression 'sum_of_squares(x, y)'\n");
        qemu_printf("Error: %s\n", expr_result.error);
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Evaluate the batch
    int32_t eval_status = expr_batch_evaluate(batch, ctx);
    if (eval_status != 0) {
        qemu_print("Error evaluating batch\n");
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Get the result
    Real result_value = expr_batch_get_result(batch, expr_result.index);
    
    // Expected result: x^2 + y^2 = 3^2 + 4^2 = 9 + 16 = 25
    Real expected = 25.0;
    
    qemu_printf("sum_of_squares(3, 4) = " FORMAT_SPEC " (expected " FORMAT_SPEC ")\n", 
                result_value, expected);
    
    if (!approx_eq(result_value, expected, TEST_PRECISION)) {
        qemu_print("Nested function result doesn't match expected value\n");
        expr_batch_free(batch);
        expr_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Clean up
    expr_batch_free(batch);
    expr_context_free(ctx);
    
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