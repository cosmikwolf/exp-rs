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

// Number of iterations for the benchmark
#define BENCHMARK_ITERATIONS 10000

// Helper to check approximate equality
static int approx_eq(real_t a, real_t b, real_t eps) {
    return FABS(a - b) < eps;
}

// Simple expressions to benchmark
static const char* expressions[] = {
    "2+2*2",
    "sin(0.5) + cos(0.5)",
    "sqrt(2) * 5 + 10",
    "2 * sin(pi/4) + cos(0.5) * 3"
};

// Test basic benchmarking of expressions
test_result_t test_benchmark() {
    qemu_printf("Running basic expression benchmark with %s mode...\n", TEST_NAME);
    
    for (size_t i = 0; i < sizeof(expressions) / sizeof(expressions[0]); i++) {
        const char* expr = expressions[i];
        qemu_printf("Benchmarking expression: %s\n", expr);
        
        // Start timer
        uint32_t start = qemu_get_tick_count();
        volatile real_t sum = 0.0; // Use volatile to prevent optimization
        
        // Run benchmark
        for (int j = 0; j < BENCHMARK_ITERATIONS; j++) {
            struct EvalResult result = exp_rs_eval(expr);
            
            if (result.status != 0) {
                qemu_printf("Error evaluating expression '%s'\n", expr);
                if (result.error) {
                    qemu_printf("Error: %s\n", result.error);
                    exp_rs_free_error((char*)result.error);
                }
                return TEST_FAIL;
            }
            
            sum += result.value;
        }
        
        // End timer
        uint32_t end = qemu_get_tick_count();
        uint32_t duration = end - start;
        
        qemu_printf("Completed %d iterations of '%s' in %u ms (sum = %f)\n", 
                  BENCHMARK_ITERATIONS, expr, duration, sum);
    }
    
    qemu_print("Benchmark test completed successfully\n");
    return TEST_PASS;
}

// Test case definition
static const test_case_t tests[] = {
    {"benchmark", test_benchmark},
};

int main(void) {
    int failed = run_tests(tests, sizeof(tests) / sizeof(tests[0]));
    qemu_exit(failed ? EXIT_FAILURE : EXIT_SUCCESS);
    return failed ? 1 : 0;
}
