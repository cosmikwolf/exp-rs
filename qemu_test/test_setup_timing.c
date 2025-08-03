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
static ArenaOpaque* g_arena = NULL;
static ArenaOpaque* g_eval_arena = NULL;
static BatchBuilderOpaque* g_eval_builder = NULL;
static EvalContextOpaque* g_ctx = NULL;

// Operation wrappers for timing
static void op_complete_setup(void) {
    EvalContextOpaque* ctx = create_test_context();
    ArenaOpaque* arena = exp_rs_arena_new(8192);
    BatchBuilderOpaque* builder = exp_rs_batch_builder_new(arena);
    
    for (int p = 0; p < 10; p++) {
        exp_rs_batch_builder_add_parameter(builder, g_param_names[p], g_param_values[p]);
    }
    
    for (int e = 0; e < 7; e++) {
        exp_rs_batch_builder_add_expression(builder, g_expressions[e]);
    }
    
    exp_rs_batch_builder_eval(builder, ctx);
    
    exp_rs_batch_builder_free(builder);
    exp_rs_arena_free(arena);
    exp_rs_context_free(ctx);
}

static void op_create_context(void) {
    EvalContextOpaque* ctx = create_test_context();
    exp_rs_context_free(ctx);
}

static void op_create_arena(void) {
    ArenaOpaque* arena = exp_rs_arena_new(8192);
    exp_rs_arena_free(arena);
}

static void op_create_builder(void) {
    // Use the global arena for builder creation timing
    if (g_arena) {
        exp_rs_arena_reset(g_arena);
    }
    BatchBuilderOpaque* builder = exp_rs_batch_builder_new(g_arena);
    exp_rs_batch_builder_free(builder);
}

static void op_parse_expressions(void) {
    // Use the global arena for parse timing
    if (g_arena) {
        exp_rs_arena_reset(g_arena);
    }
    BatchBuilderOpaque* builder = exp_rs_batch_builder_new(g_arena);
    
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

static void op_param_update(void) {
    for (int p = 0; p < 10; p++) {
        exp_rs_batch_builder_set_param(g_eval_builder, p, g_param_values[p] + 0.1);
    }
}

test_result_t test_setup_timing(void) {
    qemu_printf("=== Setup Timing Test ===\n");
    
    // Test data
    const char* expressions[] = {
        "p0 + p1",
        "p0 * p1 + p2",
        "sqrt(p0*p0 + p1*p1)",
        "p3 * sin(p4)",
        "p5 + p6 - p7",
        "p8 * p8 * p9",
        "(p0 + p1 + p2) / 3.0"
    };
    
    const char* param_names[] = {"p0", "p1", "p2", "p3", "p4", "p5", "p6", "p7", "p8", "p9"};
    double param_values[] = {1.5, 2.3, 3.7, 0.5, 1.2, -0.8, 2.1, 0.9, 1.4, 0.7};
    
    // Set global pointers
    g_expressions = expressions;
    g_param_names = param_names;
    g_param_values = param_values;
    
    // Initialize hardware timer
    init_hardware_timer();
    reset_invalid_timing_warning();
    
    // Warm up
    qemu_printf("\nWarming up...\n");
    for (int i = 0; i < 10; i++) {
        op_complete_setup();
    }
    
    qemu_printf("\n1. Setup Operations (%d iterations each)\n", SETUP_ITERATIONS);
    
    // Create global arena for builder/parse timing
    g_arena = exp_rs_arena_new(8192);
    if (!g_arena) {
        qemu_printf("FAIL: Failed to create arena\n");
        return TEST_FAIL;
    }
    
    // Context creation
    uint32_t total = 0;
    int valid_count = 0;
    qemu_printf("   Context creation: ");
    for (int i = 0; i < SETUP_ITERATIONS; i++) {
        timing_result_t result = measure_operation(op_create_context);
        if (result.valid) {
            total += result.ticks;
            valid_count++;
        }
    }
    if (valid_count > 0) {
        qemu_printf("%u ticks avg\n", total / valid_count);
    } else {
        qemu_printf("TIMING ERROR\n");
    }
    
    // Arena creation
    total = 0;
    valid_count = 0;
    qemu_printf("   Arena creation: ");
    for (int i = 0; i < SETUP_ITERATIONS; i++) {
        timing_result_t result = measure_operation(op_create_arena);
        if (result.valid) {
            total += result.ticks;
            valid_count++;
        }
    }
    if (valid_count > 0) {
        qemu_printf("%u ticks avg\n", total / valid_count);
    } else {
        qemu_printf("TIMING ERROR\n");
    }
    
    // Builder creation (with arena)
    total = 0;
    valid_count = 0;
    qemu_printf("   Builder creation: ");
    for (int i = 0; i < SETUP_ITERATIONS; i++) {
        timing_result_t result = measure_operation(op_create_builder);
        if (result.valid) {
            total += result.ticks;
            valid_count++;
        }
    }
    if (valid_count > 0) {
        qemu_printf("%u ticks avg\n", total / valid_count);
    } else {
        qemu_printf("TIMING ERROR\n");
    }
    
    // Expression parsing
    total = 0;
    valid_count = 0;
    qemu_printf("   Expression parsing: ");
    for (int i = 0; i < SETUP_ITERATIONS; i++) {
        timing_result_t result = measure_operation(op_parse_expressions);
        if (result.valid) {
            total += result.ticks;
            valid_count++;
        }
    }
    if (valid_count > 0) {
        qemu_printf("%u ticks avg\n", total / valid_count);
    } else {
        qemu_printf("TIMING ERROR\n");
    }
    
    // Clean up global arena
    exp_rs_arena_free(g_arena);
    g_arena = NULL;
    
    // Setup for evaluation tests
    qemu_printf("\n2. Runtime Operations\n");
    qemu_printf("   Setting up for evaluation tests...\n");
    
    g_ctx = create_test_context();
    exp_rs_context_register_native_function(g_ctx, "sign", 1, native_sign);
    
    g_eval_arena = exp_rs_arena_new(8192);
    if (!g_eval_arena) {
        qemu_printf("FAIL: Failed to create eval arena\n");
        exp_rs_context_free(g_ctx);
        return TEST_FAIL;
    }
    
    g_eval_builder = exp_rs_batch_builder_new(g_eval_arena);
    for (int p = 0; p < 10; p++) {
        exp_rs_batch_builder_add_parameter(g_eval_builder, param_names[p], param_values[p]);
    }
    for (int e = 0; e < 7; e++) {
        exp_rs_batch_builder_add_expression(g_eval_builder, expressions[e]);
    }
    
    // Evaluation timing
    total = 0;
    valid_count = 0;
    qemu_printf("   Evaluation (%d iterations): ", EVAL_ITERATIONS);
    
    benchmark_start();
    for (int i = 0; i < EVAL_ITERATIONS; i++) {
        exp_rs_batch_builder_eval(g_eval_builder, g_ctx);
    }
    uint32_t eval_ticks = benchmark_stop();
    
    if (eval_ticks < 0xF0000000) {
        qemu_printf("%u total ticks, ", eval_ticks);
        qemu_printf("%u ticks/eval\n", eval_ticks / EVAL_ITERATIONS);
    } else {
        qemu_printf("TIMING ERROR\n");
        increment_invalid_timing_warning();
    }
    
    // Parameter update timing
    qemu_printf("   Parameter update (%d iterations): ", EVAL_ITERATIONS);
    
    benchmark_start();
    for (int i = 0; i < EVAL_ITERATIONS; i++) {
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_set_param(g_eval_builder, p, param_values[p] + (i & 0xFF) * 0.001);
        }
    }
    uint32_t param_ticks = benchmark_stop();
    
    if (param_ticks < 0xF0000000) {
        qemu_printf("%u total ticks, ", param_ticks);
        qemu_printf("%u ticks/update\n", param_ticks / EVAL_ITERATIONS);
    } else {
        qemu_printf("TIMING ERROR\n");
        increment_invalid_timing_warning();
    }
    
    // Combined update + eval
    qemu_printf("   Update + eval (%d iterations): ", EVAL_ITERATIONS);
    
    benchmark_start();
    for (int i = 0; i < EVAL_ITERATIONS; i++) {
        // Update parameters
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_set_param(g_eval_builder, p, param_values[p] + (i & 0xFF) * 0.001);
        }
        // Evaluate
        exp_rs_batch_builder_eval(g_eval_builder, g_ctx);
    }
    uint32_t combined_ticks = benchmark_stop();
    
    if (combined_ticks < 0xF0000000) {
        qemu_printf("%u total ticks, ", combined_ticks);
        qemu_printf("%u ticks/cycle\n", combined_ticks / EVAL_ITERATIONS);
    } else {
        qemu_printf("TIMING ERROR\n");
        increment_invalid_timing_warning();
    }
    
    // Cleanup
    exp_rs_batch_builder_free(g_eval_builder);
    exp_rs_arena_free(g_eval_arena);
    exp_rs_context_free(g_ctx);
    
    // Report warnings
    int warnings = get_invalid_timing_warnings();
    if (warnings > 0) {
        qemu_printf("\nWARNING: %d invalid timing measurements detected\n", warnings);
    }
    
    qemu_printf("\nTest completed successfully\n");
    return TEST_PASS;
}

int main(void) {
    test_result_t result = test_setup_timing();
    qemu_exit(result == TEST_PASS ? 0 : 1);
    return 0;
}