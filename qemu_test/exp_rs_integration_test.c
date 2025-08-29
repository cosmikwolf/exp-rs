#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <math.h>
#include "qemu_test_harness.h"

// Include the generated header
#include "exp_rs.h"
#include "register_test_functions.h"

// Define common types and utilities for our tests
#if defined(DEF_USE_F32) || (defined(USE_F32) && !defined(USE_F64))

#define SIN sinf
#define COS cosf
#define SQRT sqrtf
#define FABS fabsf
#define TEST_NAME "F32"
#define FORMAT_SPEC "%.6f"

// Custom CMSIS-DSP function implementations
static inline float custom_arm_sin_f32(float x) {
    return sinf(x);
}
#define ARM_SIN custom_arm_sin_f32

static inline float custom_arm_cos_f32(float x) {
    return cosf(x);
}
#define ARM_COS custom_arm_cos_f32

static inline void custom_arm_sqrt_f32(float in, float *out) {
    *out = sqrtf(in);
}
#define ARM_SQRT(x, result) custom_arm_sqrt_f32(x, result)

#elif defined(DEF_USE_F64) || defined(USE_F64)

#define SIN sin
#define COS cos
#define SQRT sqrt
#define FABS fabs
#define TEST_NAME "F64"
#define FORMAT_SPEC "%.12f"

// Custom CMSIS-DSP function implementations
static inline double custom_arm_sin_f64(double x) {
    return sin(x);
}
#define ARM_SIN custom_arm_sin_f64

static inline double custom_arm_cos_f64(double x) {
    return cos(x);
}
#define ARM_COS custom_arm_cos_f64

static inline void custom_arm_sqrt_f64(double in, double *out) {
    *out = sqrt(in);
}
#define ARM_SQRT(x, result) custom_arm_sqrt_f64(x, result)

#else
#error "Neither USE_F32 nor USE_F64 is defined."
#endif

// Using the EvalResult struct directly

// Helper to check approximate equality
static int approx_eq(Real a, Real b, Real eps) {
    return FABS(a - b) < eps;
}

// We use TEST_PRECISION from the header file for all tests

// Test basic integration of exp-rs with our custom functions
test_result_t test_expression_eval() {
    qemu_printf("Testing basic expression evaluation with %s mode\n", TEST_NAME);
    
    // Create a test context with math functions
    struct EvalContextOpaque* ctx = create_test_context();
    if (!ctx) {
        qemu_print("Failed to create test context\n");
        return TEST_FAIL;
    }
    
    // Test various expressions with our math functions
    struct {
        const char* expr;
        Real expected_result;
    } test_cases[] = {
        {"sin(0.5)", ARM_SIN(0.5)},
        {"cos(0.5)", ARM_COS(0.5)},
        {"sqrt(2)", 1.414213562373095},
        {"sin(0.5) + cos(0.5)", ARM_SIN(0.5) + ARM_COS(0.5)},
        {"sqrt(2) * 5 + 10", 1.414213562373095 * 5 + 10},
        {"2 * sin(pi/4) + cos(0.5) * 3", 2 * ARM_SIN(3.141592653589793/4) + ARM_COS(0.5) * 3}
    };
    
    for (size_t i = 0; i < sizeof(test_cases) / sizeof(test_cases[0]); i++) {
        qemu_printf("Testing expression: %s\n", test_cases[i].expr);
        struct EvalResult result = exp_rs_context_eval(test_cases[i].expr, ctx);
        
        if (result.status != 0) {
            qemu_printf("Error evaluating expression '%s'\n", test_cases[i].expr);
            if (result.error) {
                qemu_printf("Error: %s\n", result.error);
                exp_rs_free_error((char*)result.error);
            }
            exp_rs_context_free(ctx);
            return TEST_FAIL;
        }
        
        qemu_printf("exp_rs_context_eval('%s') = " FORMAT_SPEC " (expected " FORMAT_SPEC ")\n", 
                   test_cases[i].expr, result.value, test_cases[i].expected_result);
                   
        if (!approx_eq(result.value, test_cases[i].expected_result, TEST_PRECISION)) {
            qemu_printf("Test failed: expression mismatch. Expected " FORMAT_SPEC ", got " FORMAT_SPEC "\n", 
                      test_cases[i].expected_result, result.value);
            exp_rs_context_free(ctx);
            return TEST_FAIL;
        }
        
        // Even if the values match within tolerance, show the difference for debugging
        qemu_printf("Precision difference: %e\n", FABS(result.value - test_cases[i].expected_result));
    }
    
    // Clean up context
    exp_rs_context_free(ctx);
    
    qemu_print("Expression evaluation tests passed!\n");
    return TEST_PASS;
}

// Test expression context with our custom functions
test_result_t test_context_integration() {
    qemu_printf("Testing context integration with %s mode\n", TEST_NAME);
    
    // Create a test context with math functions
    struct EvalContextOpaque* ctx = create_test_context();
    if (!ctx) {
        qemu_print("Failed to create context\n");
        return TEST_FAIL;
    }
    
    // Set parameters
    struct EvalResult set_result_x = exp_rs_context_set_parameter(ctx, "x", 0.5);
    if (set_result_x.status != 0) {
        qemu_print("Error setting parameter 'x'\n");
        if (set_result_x.error) {
            qemu_printf("Error: %s\n", set_result_x.error);
            exp_rs_free_error((char*)set_result_x.error);
        }
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    struct EvalResult set_result_y = exp_rs_context_set_parameter(ctx, "y", 2.0);
    if (set_result_y.status != 0) {
        qemu_print("Error setting parameter 'y'\n");
        if (set_result_y.error) {
            qemu_printf("Error: %s\n", set_result_y.error);
            exp_rs_free_error((char*)set_result_y.error);
        }
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Register a function that uses our math functions
    const char* func_name = "my_math_func";
    const char* param1_name = "a";
    const char* param2_name = "b";
    const char* params[] = {param1_name, param2_name};
    const char* expr = "sin(a) + cos(b) + sqrt(a*b)";
    
    struct EvalResult reg_result = exp_rs_context_register_expression_function(
        ctx, func_name, (const char**)params, 2, expr);
    
    if (reg_result.status != 0) {
        qemu_printf("Failed to register function\n");
        if (reg_result.error) {
            qemu_printf("Error: %s\n", reg_result.error);
            exp_rs_free_error((char*)reg_result.error);
        }
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Test function with our parameters
    struct EvalResult result = exp_rs_context_eval("my_math_func(x, y)", ctx);
    if (result.status != 0) {
        qemu_print("Error evaluating 'my_math_func(x, y)'\n");
        if (result.error) {
            qemu_printf("Error: %s\n", result.error);
            exp_rs_free_error((char*)result.error);
        }
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Calculate expected result manually
    Real a = 0.5;
    Real b = 2.0;
    Real sqrt_result;
    ARM_SQRT(a*b, &sqrt_result);
    Real expected = ARM_SIN(a) + ARM_COS(b) + sqrt_result;
    
    qemu_printf("my_math_func(0.5, 2.0) = " FORMAT_SPEC " (expected " FORMAT_SPEC ")\n", 
               result.value, expected);
    
    if (!approx_eq(result.value, expected, TEST_PRECISION)) {
        qemu_print("Context function result doesn't match expected value\n");
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Even if the values match within tolerance, show the difference for debugging
    qemu_printf("Precision difference: %e\n", FABS(result.value - expected));
    
    // Clean up
    exp_rs_context_free(ctx);
    
    qemu_print("Context integration tests passed!\n");
    return TEST_PASS;
}

// Test case definition
static const test_case_t tests[] = {
    {"expression_eval", test_expression_eval},
    {"context_integration", test_context_integration},
};

int main(void) {
    int failed = run_tests(tests, sizeof(tests) / sizeof(tests[0]));
    qemu_exit(failed ? EXIT_FAILURE : EXIT_SUCCESS);
    return failed ? 1 : 0;
}
