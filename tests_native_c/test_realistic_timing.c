#define NANOTIME_IMPLEMENTATION
#include "nanotime.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include "exp_rs.h"

// Native function implementations
Real native_sin(const Real* args, uintptr_t nargs) { (void)nargs; return sin(args[0]); }
Real native_cos(const Real* args, uintptr_t nargs) { (void)nargs; return cos(args[0]); }
Real native_sqrt(const Real* args, uintptr_t nargs) { (void)nargs; return sqrt(args[0]); }
Real native_exp(const Real* args, uintptr_t nargs) { (void)nargs; return exp(args[0]); }
Real native_log(const Real* args, uintptr_t nargs) { (void)nargs; return log(args[0]); }
Real native_log10(const Real* args, uintptr_t nargs) { (void)nargs; return log10(args[0]); }
Real native_pow(const Real* args, uintptr_t nargs) { (void)nargs; return pow(args[0], args[1]); }
Real native_atan2(const Real* args, uintptr_t nargs) { (void)nargs; return atan2(args[0], args[1]); }
Real native_abs(const Real* args, uintptr_t nargs) { (void)nargs; return fabs(args[0]); }
Real native_sign(const Real* args, uintptr_t nargs) { 
    (void)nargs;
    return (args[0] > 0) ? 1.0 : (args[0] < 0) ? -1.0 : 0.0;
}
Real native_min(const Real* args, uintptr_t nargs) { (void)nargs; return args[0] < args[1] ? args[0] : args[1]; }
Real native_max(const Real* args, uintptr_t nargs) { (void)nargs; return args[0] > args[1] ? args[0] : args[1]; }
Real native_fmod(const Real* args, uintptr_t nargs) { (void)nargs; return fmod(args[0], args[1]); }

// Create and configure a context
EvalContextOpaque* create_test_context() {
    EvalContextOpaque* ctx = exp_rs_context_new();
    
    // Register functions
    exp_rs_context_register_native_function(ctx, "sin", 1, native_sin);
    exp_rs_context_register_native_function(ctx, "cos", 1, native_cos);
    exp_rs_context_register_native_function(ctx, "sqrt", 1, native_sqrt);
    exp_rs_context_register_native_function(ctx, "exp", 1, native_exp);
    exp_rs_context_register_native_function(ctx, "log", 1, native_log);
    exp_rs_context_register_native_function(ctx, "log10", 1, native_log10);
    exp_rs_context_register_native_function(ctx, "pow", 2, native_pow);
    exp_rs_context_register_native_function(ctx, "atan2", 2, native_atan2);
    exp_rs_context_register_native_function(ctx, "abs", 1, native_abs);
    exp_rs_context_register_native_function(ctx, "sign", 1, native_sign);
    exp_rs_context_register_native_function(ctx, "min", 2, native_min);
    exp_rs_context_register_native_function(ctx, "max", 2, native_max);
    exp_rs_context_register_native_function(ctx, "fmod", 2, native_fmod);
    
    return ctx;
}

int main() {
    printf("=== Realistic Usage Timing (Bulk Operations) ===\n");
    
    // Initialize nanotime
    uint64_t now_max = nanotime_now_max();
    
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
    
    // Warm up
    {
        EvalContextOpaque* ctx = create_test_context();
        
        // Create arena and builder
        ArenaOpaque* arena = exp_rs_arena_new(32768);
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new(arena);
        
        // Add parameters
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[p], param_values[p]);
        }
        
        // Add expressions
        for (int e = 0; e < 7; e++) {
            exp_rs_batch_builder_add_expression(builder, expressions[e]);
        }
        
        // Evaluate
        exp_rs_batch_builder_eval(builder, ctx);
        
        // Cleanup
        exp_rs_batch_builder_free(builder);
        exp_rs_arena_free(arena);
        exp_rs_context_free(ctx);
    }
    
    // Create context
    EvalContextOpaque* ctx = create_test_context();
    
    printf("\n1. Context Creation (100 times)\n");
    uint64_t start = nanotime_now();
    
    for (int i = 0; i < 100; i++) {
        EvalContextOpaque* temp_ctx = create_test_context();
        exp_rs_context_free(temp_ctx);
    }
    
    uint64_t elapsed = nanotime_interval(start, nanotime_now(), now_max);
    printf("   Total: %.3f ms\n", elapsed / 1000000.0);
    printf("   Average: %.3f µs per context\n", elapsed / 1000.0 / 100.0);
    
    printf("\n2. Full Setup (Parse + Build) with Arena Reuse (10000 times)\n");
    
    // Create a reusable arena
    ArenaOpaque* setup_arena = exp_rs_arena_new(32768);
    start = nanotime_now();
    
    for (int i = 0; i < 10000; i++) {
        if (i > 0) exp_rs_arena_reset(setup_arena);
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new(setup_arena);
        
        // Add parameters first
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[p], param_values[p]);
        }
        
        // Add expressions
        for (int e = 0; e < 7; e++) {
            exp_rs_batch_builder_add_expression(builder, expressions[e]);
        }
        
        exp_rs_batch_builder_free(builder);
    }
    
    elapsed = nanotime_interval(start, nanotime_now(), now_max);
    exp_rs_arena_free(setup_arena);
    
    printf("   Total: %.3f ms\n", elapsed / 1000000.0);
    printf("   Average: %.3f µs per setup\n", elapsed / 1000.0 / 10000.0);
    
    printf("\n3. Parameter Setup Only with Arena Reuse (10000 times)\n");
    
    ArenaOpaque* param_arena = exp_rs_arena_new(32768);
    start = nanotime_now();
    
    for (int i = 0; i < 10000; i++) {
        if (i > 0) exp_rs_arena_reset(param_arena);
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new(param_arena);
        
        // Add parameters only
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[p], param_values[p]);
        }
        
        exp_rs_batch_builder_free(builder);
    }
    
    elapsed = nanotime_interval(start, nanotime_now(), now_max);
    exp_rs_arena_free(param_arena);
    
    printf("   Total: %.3f ms\n", elapsed / 1000000.0);
    printf("   Average: %.3f µs per param setup\n", elapsed / 1000.0 / 10000.0);
    printf("   Expression parsing overhead: ~%.3f µs per expression\n", 
           ((elapsed / 1000.0 / 10000.0) / 7.0));
    
    printf("\n4. Runtime Performance (100000 iterations)\n");
    
    // Setup once with arena
    ArenaOpaque* eval_arena = exp_rs_arena_new(32768);
    BatchBuilderOpaque* eval_builder = exp_rs_batch_builder_new(eval_arena);
    for (int p = 0; p < 10; p++) {
        exp_rs_batch_builder_add_parameter(eval_builder, param_names[p], param_values[p]);
    }
    for (int e = 0; e < 7; e++) {
        exp_rs_batch_builder_add_expression(eval_builder, expressions[e]);
    }
    
    // Eval only
    start = nanotime_now();
    
    for (int i = 0; i < 100000; i++) {
        exp_rs_batch_builder_eval(eval_builder, ctx);
    }
    
    elapsed = nanotime_interval(start, nanotime_now(), now_max);
    
    printf("   Total: %.3f ms\n", elapsed / 1000000.0);
    printf("   Average: %.3f µs per eval\n", elapsed / 1000.0 / 100000.0);
    printf("   Per expression: %.3f µs\n", elapsed / 1000.0 / 100000.0 / 7.0);
    
    // Parameter update + eval
    printf("\n5. Parameter Update + Eval (100000 iterations)\n");
    start = nanotime_now();
    
    for (int i = 0; i < 100000; i++) {
        // Update parameters
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_set_param(eval_builder, p, param_values[p] + (i % 100) * 0.01);
        }
        
        // Evaluate
        exp_rs_batch_builder_eval(eval_builder, ctx);
    }
    
    elapsed = nanotime_interval(start, nanotime_now(), now_max);
    
    printf("   Total: %.3f ms\n", elapsed / 1000000.0);
    printf("   Average: %.3f µs per cycle\n", elapsed / 1000.0 / 100000.0);
    
    // Cleanup eval builder
    exp_rs_batch_builder_free(eval_builder);
    exp_rs_arena_free(eval_arena);
    
    // Test at different batch sizes
    printf("\n6. Batch Size Scaling\n");
    int batch_sizes[] = {1, 10, 100, 1000};
    
    for (int b = 0; b < 4; b++) {
        int batch_size = batch_sizes[b];
        int iterations = 100000 / batch_size;  // Keep total operations constant
        
        // Create arena and builder for this batch size
        ArenaOpaque* batch_arena = exp_rs_arena_new(32768);
        BatchBuilderOpaque* batch_builder = exp_rs_batch_builder_new(batch_arena);
        
        // Setup
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_add_parameter(batch_builder, param_names[p], param_values[p]);
        }
        for (int e = 0; e < 7; e++) {
            exp_rs_batch_builder_add_expression(batch_builder, expressions[e]);
        }
        
        start = nanotime_now();
        
        for (int i = 0; i < iterations; i++) {
            // Simulate batch processing
            for (int j = 0; j < batch_size; j++) {
                // Update params for each item
                for (int p = 0; p < 10; p++) {
                    exp_rs_batch_builder_set_param(batch_builder, p, 
                        param_values[p] + (i * batch_size + j) * 0.001);
                }
                
                // Evaluate
                exp_rs_batch_builder_eval(batch_builder, ctx);
            }
        }
        
        elapsed = nanotime_interval(start, nanotime_now(), now_max);
        
        printf("   Batch size %4d: %.3f µs per item (%.3f ms total)\n", 
               batch_size,
               elapsed / 1000.0 / (iterations * batch_size),
               elapsed / 1000000.0);
        
        exp_rs_batch_builder_free(batch_builder);
        exp_rs_arena_free(batch_arena);
    }
    
    // Cleanup
    exp_rs_context_free(ctx);
    
    return 0;
}