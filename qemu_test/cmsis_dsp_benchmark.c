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

// Number of iterations for the benchmark
#define BENCHMARK_ITERATIONS 10000

// Helper to check approximate equality
static int approx_eq(Real a, Real b, Real eps) {
    return FABS(a - b) < eps;
}

// Benchmark CMSIS-DSP trig functions
test_result_t test_cmsis_dsp_benchmark() {
    qemu_printf("Running CMSIS-DSP benchmark with %s mode...\n", TEST_NAME);
    
    // Test values
    const int num_values = 5;
    Real values[] = {0.0, 0.5, 1.0, 1.5, 2.0};
    
    // Benchmark sin
    qemu_print("Benchmarking sin function...\n");
    uint32_t start = qemu_get_tick_count();
    volatile Real sin_sum = 0.0; // Use volatile to prevent optimization
    
    for (int i = 0; i < BENCHMARK_ITERATIONS; i++) {
        for (int j = 0; j < num_values; j++) {
            sin_sum += ARM_SIN(values[j]);
        }
    }
    
    uint32_t end = qemu_get_tick_count();
    uint32_t sin_duration = end - start;
    qemu_printf("Completed %d sin calls in %u ms (sum = %f)\n", 
               BENCHMARK_ITERATIONS * num_values, sin_duration, sin_sum);
    
    // Benchmark cos
    qemu_print("Benchmarking cos function...\n");
    start = qemu_get_tick_count();
    volatile Real cos_sum = 0.0;
    
    for (int i = 0; i < BENCHMARK_ITERATIONS; i++) {
        for (int j = 0; j < num_values; j++) {
            cos_sum += ARM_COS(values[j]);
        }
    }
    
    end = qemu_get_tick_count();
    uint32_t cos_duration = end - start;
    qemu_printf("Completed %d cos calls in %u ms (sum = %f)\n", 
               BENCHMARK_ITERATIONS * num_values, cos_duration, cos_sum);
    
    // Benchmark sqrt
    qemu_print("Benchmarking sqrt function...\n");
    start = qemu_get_tick_count();
    volatile Real sqrt_sum = 0.0;
    Real sqrt_result;
    
    for (int i = 0; i < BENCHMARK_ITERATIONS; i++) {
        for (int j = 0; j < num_values; j++) {
            if (values[j] > 0) { // Avoid sqrt of negative numbers
                ARM_SQRT(values[j], &sqrt_result);
                sqrt_sum += sqrt_result;
            }
        }
    }
    
    end = qemu_get_tick_count();
    uint32_t sqrt_duration = end - start;
    qemu_printf("Completed %d sqrt calls in %u ms (sum = %f)\n", 
               BENCHMARK_ITERATIONS * (num_values - 1), sqrt_duration, sqrt_sum);
    
    qemu_print("CMSIS-DSP benchmark completed successfully\n");
    return TEST_PASS;
}

// Test case definition
static const test_case_t tests[] = {
    {"cmsis_dsp_benchmark", test_cmsis_dsp_benchmark},
};

int main(void) {
    int failed = run_tests(tests, sizeof(tests) / sizeof(tests[0]));
    qemu_exit(failed ? EXIT_FAILURE : EXIT_SUCCESS);
    return failed ? 1 : 0;
}
