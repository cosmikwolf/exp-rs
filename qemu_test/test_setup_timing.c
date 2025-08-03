#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <math.h>
#include "qemu_test_harness.h"
#include "register_test_functions.h"

// Include the generated header
#include "../include/exp_rs.h"

// Number of iterations for setup timing
#define SETUP_ITERATIONS 100
#define EVAL_ITERATIONS 10000

// Native function for sign (not in standard math functions)
Real native_sign(const Real* args, uintptr_t nargs) { 
    (void)nargs;
    return (args[0] > 0) ? 1.0 : (args[0] < 0) ? -1.0 : 0.0;
}

// Use the proper benchmark functions instead of raw timer reads
typedef struct {
    uint32_t ticks;
    int valid;
} timing_result_t;

static timing_result_t measure_operation(void (*operation)(void)) {
    timing_result_t result = {0, 0};
    
    // Use the benchmark functions which handle timer properly
    benchmark_start();
    operation();
    result.ticks = benchmark_stop();
    
    // Sanity check
    if (result.ticks < 0xF0000000) {
        result.valid = 1;
    } else {
        // Track invalid timing warnings
        increment_invalid_timing_warning();
    }
    
    return result;
}

// Global variables for test operations
static const char** g_expressions;
static const char** g_param_names;
static double* g_param_values;
static BatchBuilderOpaque* g_eval_builder = NULL;
static EvalContextOpaque* g_ctx = NULL;

// Operation wrappers for timing
static void op_complete_setup(void) {
    EvalContextOpaque* ctx = create_test_context();
    BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
    
    for (int p = 0; p < 10; p++) {
        exp_rs_batch_builder_add_parameter(builder, g_param_names[p], g_param_values[p]);
    }
    
    for (int e = 0; e < 7; e++) {
        exp_rs_batch_builder_add_expression(builder, g_expressions[e]);
    }
    
    exp_rs_batch_builder_eval(builder, ctx);
    
    exp_rs_batch_builder_free(builder);
    exp_rs_context_free(ctx);
}

static void op_create_context(void) {
    EvalContextOpaque* ctx = create_test_context();
    exp_rs_context_free(ctx);
}

static void op_create_builder(void) {
    BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
    exp_rs_batch_builder_free(builder);
}

static void op_parse_expressions(void) {
    BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
    
    for (int p = 0; p < 10; p++) {
        exp_rs_batch_builder_add_parameter(builder, g_param_names[p], g_param_values[p]);
    }
    
    for (int e = 0; e < 7; e++) {
        exp_rs_batch_builder_add_expression(builder, g_expressions[e]);
    }
    
    exp_rs_batch_builder_free(builder);
}

static void op_evaluate(void) {
    exp_rs_batch_builder_eval(g_eval_builder, g_ctx);
}

static void op_full_cycle(void) {
    static int counter = 0;
    
    for (int p = 0; p < 10; p++) {
        exp_rs_batch_builder_set_param(g_eval_builder, p, g_param_values[p] + (counter % 100) * 0.01);
    }
    
    exp_rs_batch_builder_eval(g_eval_builder, g_ctx);
    counter++;
}

// Helper to run multiple iterations and get average
static uint32_t measure_average(void (*operation)(void), int iterations) {
    uint32_t total_ticks = 0;
    int valid_runs = 0;
    
    for (int i = 0; i < iterations; i++) {
        timing_result_t result = measure_operation(operation);
        if (result.valid) {
            total_ticks += result.ticks;
            valid_runs++;
        }
    }
    
    if (valid_runs == 0) return 0;
    return total_ticks / valid_runs;
}

// Test setup timing on embedded platform
test_result_t test_setup_timing() {
    qemu_print("=== Setup Timing Test (QEMU/ARM) ===\n\n");
    
    // Initialize hardware timer
    init_hardware_timer();
    
    // Get test start time
    uint32_t test_start_value, test_start_overflows;
    get_timer_snapshot(&test_start_value, &test_start_overflows);
    
    const char* expressions[] = {
        "a*sin(b*3.14159/180) + c*cos(d*3.14159/180) + sqrt(e*e + f*f)",
        "exp(g/10) * log(h+1) + pow(i, 0.5) * j",
        "((a > 5) && (b < 10)) * c + ((d >= e) || (f != g)) * h + min(i, j)",
        "sqrt(pow(a-e, 2) + pow(b-f, 2)) + atan2(c-g, d-h) * (i+j)/2",
        "abs(a-b) * sign(c-d) + max(e, f) * min(g, h) + fmod(i*j, 10)",
        "(a+b+c)/3 * sin((d+e+f)*3.14159/6) + log10(g*h+1) - exp(-i*j/100)",
        "a + b * c - d / (e + 0.001) + pow(f, g) * h - i + j"
    };
    
    const char* param_names[] = {"a", "b", "c", "d", "e", "f", "g", "h", "i", "j"};
    double param_values[] = {1.5, 3.0, 4.5, 6.0, 7.5, 9.0, 10.5, 12.0, 13.5, 15.0};
    
    // Set global pointers for operation functions
    g_expressions = expressions;
    g_param_names = param_names;
    g_param_values = param_values;
    
    // Test 1: Complete setup time
    qemu_print("1. Complete Setup Time\n");
    reset_warning_counts();
    uint32_t avg_setup = measure_average(op_complete_setup, SETUP_ITERATIONS);
    qemu_printf("   Average ticks per complete setup: %u\n", avg_setup);
    uint32_t small_warnings = get_small_elapsed_warning_count();
    uint32_t invalid_warnings = get_invalid_timing_warning_count();
    if (small_warnings > 0 || invalid_warnings > 0) {
        qemu_printf("   [Warnings: %u invalid timing, %u small elapsed]\n", 
                    invalid_warnings, small_warnings);
    }
    
    // Test 2: Component timing
    qemu_print("\n2. Component Timing\n");
    reset_warning_counts();
    
    uint32_t avg_context = measure_average(op_create_context, SETUP_ITERATIONS);
    qemu_printf("   Context creation: %u ticks average\n", avg_context);
    
    uint32_t avg_builder = measure_average(op_create_builder, SETUP_ITERATIONS * 10);
    qemu_printf("   Builder creation: %u ticks average\n", avg_builder);
    
    small_warnings = get_small_elapsed_warning_count();
    invalid_warnings = get_invalid_timing_warning_count();
    if (small_warnings > 0 || invalid_warnings > 0) {
        qemu_printf("   [Warnings: %u invalid timing, %u small elapsed]\n", 
                    invalid_warnings, small_warnings);
    }
    
    // Test 3: Expression parsing
    qemu_print("\n3. Expression Parsing\n");
    reset_warning_counts();
    uint32_t avg_parse = measure_average(op_parse_expressions, SETUP_ITERATIONS);
    qemu_printf("   Parse all 7 expressions: %u ticks average\n", avg_parse);
    qemu_printf("   Per expression: %u ticks\n", avg_parse / 7);
    small_warnings = get_small_elapsed_warning_count();
    invalid_warnings = get_invalid_timing_warning_count();
    if (small_warnings > 0 || invalid_warnings > 0) {
        qemu_printf("   [Warnings: %u invalid timing, %u small elapsed]\n", 
                    invalid_warnings, small_warnings);
    }
    
    // Test 4: Runtime evaluation
    qemu_print("\n4. Runtime Evaluation Performance\n");
    reset_warning_counts();
    
    // Setup a builder for evaluation tests
    g_ctx = create_test_context();
    exp_rs_context_register_native_function(g_ctx, "sign", 1, native_sign);
    
    g_eval_builder = exp_rs_batch_builder_new();
    for (int p = 0; p < 10; p++) {
        exp_rs_batch_builder_add_parameter(g_eval_builder, param_names[p], param_values[p]);
    }
    for (int e = 0; e < 7; e++) {
        exp_rs_batch_builder_add_expression(g_eval_builder, expressions[e]);
    }
    
    // Warm up
    for (int i = 0; i < 100; i++) {
        op_evaluate();
    }
    
    // Measure evaluation time
    benchmark_start();
    for (int i = 0; i < EVAL_ITERATIONS; i++) {
        op_evaluate();
    }
    uint32_t eval_ticks = benchmark_stop();
    
    qemu_printf("   %d evaluations in %u ticks\n", EVAL_ITERATIONS, eval_ticks);
    qemu_printf("   Ticks per evaluation: %u\n", eval_ticks / EVAL_ITERATIONS);
    small_warnings = get_small_elapsed_warning_count();
    invalid_warnings = get_invalid_timing_warning_count();
    if (small_warnings > 0 || invalid_warnings > 0) {
        qemu_printf("   [Warnings: %u invalid timing, %u small elapsed]\n", 
                    invalid_warnings, small_warnings);
    }
    
    // Test 5: Full cycle with parameter updates
    qemu_print("\n5. Full Cycle (params + eval)\n");
    reset_warning_counts();
    
    benchmark_start();
    for (int i = 0; i < EVAL_ITERATIONS; i++) {
        op_full_cycle();
    }
    uint32_t full_ticks = benchmark_stop();
    
    qemu_printf("   %d full cycles in %u ticks\n", EVAL_ITERATIONS, full_ticks);
    qemu_printf("   Ticks per full cycle: %u\n", full_ticks / EVAL_ITERATIONS);
    qemu_printf("   Parameter update overhead: %u ticks\n", 
                (full_ticks - eval_ticks) / EVAL_ITERATIONS);
    small_warnings = get_small_elapsed_warning_count();
    invalid_warnings = get_invalid_timing_warning_count();
    if (small_warnings > 0 || invalid_warnings > 0) {
        qemu_printf("   [Warnings: %u invalid timing, %u small elapsed]\n", 
                    invalid_warnings, small_warnings);
    }
    
    // Summary
    qemu_print("\n6. Summary\n");
    qemu_printf("   Complete setup: %u ticks\n", avg_setup);
    qemu_printf("   - Context creation: %u ticks (%u%%)\n", 
                avg_context, (avg_context * 100) / avg_setup);
    qemu_printf("   - Expression parsing: %u ticks (%u%%)\n", 
                avg_parse, (avg_parse * 100) / avg_setup);
    qemu_printf("   - Evaluation: %u ticks\n", eval_ticks / EVAL_ITERATIONS);
    
    // Cleanup
    exp_rs_batch_builder_free(g_eval_builder);
    exp_rs_context_free(g_ctx);
    
    // Get test end time and calculate total duration
    uint32_t test_end_value, test_end_overflows;
    get_timer_snapshot(&test_end_value, &test_end_overflows);
    
    uint64_t total_test_ticks = calculate_total_ticks(test_start_value, test_end_value,
                                                       test_start_overflows, test_end_overflows);
    
    qemu_print("\n7. Total Test Duration\n");
    // Print the 64-bit value properly
    uint32_t high_part = (uint32_t)(total_test_ticks >> 32);
    uint32_t low_part = (uint32_t)(total_test_ticks & 0xFFFFFFFF);
    if (high_part > 0) {
        qemu_printf("   Total ticks: 0x%08x%08x\n", high_part, low_part);
    } else {
        qemu_printf("   Total ticks: %u\n", low_part);
    }
    qemu_print("   Note: Compare with wall clock time reported by meson/qemu\n");
    qemu_print("   to calculate ticks per second rate\n");
    
    qemu_print("\nTest completed successfully\n");
    return TEST_PASS;
}

// Main entry point
int main(void) {
    test_result_t result = test_setup_timing();
    qemu_exit(result == TEST_PASS ? 0 : 1);
    return 0;
}