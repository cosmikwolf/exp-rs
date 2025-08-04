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
ExprContext* create_test_context() {
    ExprContext* ctx = expr_context_new();
    
    // Register functions
    expr_context_add_function(ctx, "sin", 1, native_sin);
    expr_context_add_function(ctx, "cos", 1, native_cos);
    expr_context_add_function(ctx, "sqrt", 1, native_sqrt);
    expr_context_add_function(ctx, "exp", 1, native_exp);
    expr_context_add_function(ctx, "log", 1, native_log);
    expr_context_add_function(ctx, "log10", 1, native_log10);
    expr_context_add_function(ctx, "pow", 2, native_pow);
    expr_context_add_function(ctx, "atan2", 2, native_atan2);
    expr_context_add_function(ctx, "abs", 1, native_abs);
    expr_context_add_function(ctx, "sign", 1, native_sign);
    expr_context_add_function(ctx, "min", 2, native_min);
    expr_context_add_function(ctx, "max", 2, native_max);
    expr_context_add_function(ctx, "fmod", 2, native_fmod);
    
    return ctx;
}

// Prevent optimization by using results
volatile double g_sink = 0.0;

int main() {
    printf("=== Comprehensive Expression Evaluation Timing ===\n");
    
    // Get nanotime info
    uint64_t now_max = nanotime_now_max();
    
    printf("\nNanotime info:\n");
    printf("  Max timestamp: %llu\n", (unsigned long long)now_max);
    
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
    
    // Warm up once
    {
        ExprContext* ctx = create_test_context();
        
        // Create arena and builder
        ExprArena* arena = expr_arena_new(32768);
        ExprBatch* builder = expr_batch_new(arena);
        
        // Add parameters
        for (int p = 0; p < 10; p++) {
            expr_batch_add_variable(builder, param_names[p], param_values[p]);
        }
        
        // Add expressions
        for (int e = 0; e < 7; e++) {
            expr_batch_add_expression(builder, expressions[e]);
        }
        
        // Evaluate
        expr_batch_evaluate(builder, ctx);
        
        // Cleanup
        expr_batch_free(builder);
        expr_arena_free(arena);
        expr_context_free(ctx);
    }
    
    printf("\n1. Context Creation\n");
    
    // Context creation
    const uint64_t ctx_start = nanotime_now();
    for (int i = 0; i < 100; i++) {
        ExprContext* ctx = create_test_context();
        expr_context_free(ctx);
    }
    const uint64_t ctx_end = nanotime_now();
    const double ctx_us = nanotime_interval(ctx_start, ctx_end, now_max) / 1000.0 / 100.0;
    printf("   Context creation + function registration: %.3f µs\n", ctx_us);
    
    // Create context for remaining tests
    ExprContext* ctx = create_test_context();
    
    printf("\n2. Setup Overhead\n");
    
    // Arena creation
    const uint64_t arena_start = nanotime_now();
    for (int i = 0; i < 10000; i++) {
        ExprArena* arena = expr_arena_new(32768);
        expr_arena_free(arena);
    }
    const uint64_t arena_end = nanotime_now();
    const double arena_us = nanotime_interval(arena_start, arena_end, now_max) / 1000.0 / 10000.0;
    printf("   Arena creation: %.3f µs\n", arena_us);
    
    // Builder creation with arena
    ExprArena* test_arena = expr_arena_new(32768);
    const uint64_t builder_start = nanotime_now();
    for (int i = 0; i < 10000; i++) {
        ExprBatch* builder = expr_batch_new(test_arena);
        expr_batch_free(builder);
        expr_arena_reset(test_arena);
    }
    const uint64_t builder_end = nanotime_now();
    expr_arena_free(test_arena);
    const double builder_us = nanotime_interval(builder_start, builder_end, now_max) / 1000.0 / 10000.0;
    printf("   Builder creation (with arena): %.3f µs\n", builder_us);
    
    printf("\n3. Parsing Overhead\n");
    
    // Full parsing with arena reuse
    ExprArena* parse_arena = expr_arena_new(32768);
    const uint64_t parse_start = nanotime_now();
    
    for (int i = 0; i < 1000; i++) {
        if (i > 0) expr_arena_reset(parse_arena);
        ExprBatch* builder = expr_batch_new(parse_arena);
        
        // Add parameters
        for (int p = 0; p < 10; p++) {
            expr_batch_add_variable(builder, param_names[p], param_values[p]);
        }
        
        // Add expressions
        for (int e = 0; e < 7; e++) {
            expr_batch_add_expression(builder, expressions[e]);
        }
        
        expr_batch_free(builder);
    }
    
    const uint64_t parse_end = nanotime_now();
    expr_arena_free(parse_arena);
    const double parse_total_us = nanotime_interval(parse_start, parse_end, now_max) / 1000.0 / 1000.0;
    printf("   Full setup (10 params + 7 expressions): %.3f µs\n", parse_total_us);
    
    // Builder + parameters only (with arena)
    ExprArena* param_arena = expr_arena_new(32768);
    const uint64_t builder_param_start = nanotime_now();
    
    for (int i = 0; i < 1000; i++) {
        if (i > 0) expr_arena_reset(param_arena);
        ExprBatch* builder = expr_batch_new(param_arena);
        
        // Add parameters only
        for (int p = 0; p < 10; p++) {
            expr_batch_add_variable(builder, param_names[p], param_values[p]);
        }
        
        expr_batch_free(builder);
    }
    
    const uint64_t builder_param_end = nanotime_now();
    expr_arena_free(param_arena);
    const double builder_param_us = nanotime_interval(builder_param_start, builder_param_end, now_max) / 1000.0 / 1000.0;
    printf("   Builder + 10 parameters: %.3f µs\n", builder_param_us);
    
    // Derive expression parsing cost
    const double expr_parse_us = (parse_total_us - builder_param_us) / 7.0;
    printf("   Per expression parsing: %.3f µs\n", expr_parse_us);
    
    printf("\n4. Runtime Evaluation Performance\n");
    
    // Setup a builder for evaluation tests
    ExprArena* eval_arena = expr_arena_new(32768);
    ExprBatch* eval_builder = expr_batch_new(eval_arena);
    for (int p = 0; p < 10; p++) {
        expr_batch_add_variable(eval_builder, param_names[p], param_values[p]);
    }
    for (int e = 0; e < 7; e++) {
        expr_batch_add_expression(eval_builder, expressions[e]);
    }
    
    // Single eval timing
    const uint64_t single_start = nanotime_now();
    for (int i = 0; i < 10000; i++) {
        expr_batch_evaluate(eval_builder, ctx);
        
        // Use results to prevent optimization
        for (int e = 0; e < 7; e++) {
            g_sink += expr_batch_get_result(eval_builder, e);
        }
    }
    const uint64_t single_end = nanotime_now();
    const double single_us = nanotime_interval(single_start, single_end, now_max) / 1000.0 / 10000.0;
    printf("   Single batch eval (7 expressions): %.3f µs\n", single_us);
    printf("   Per expression eval: %.3f µs\n", single_us / 7.0);
    
    // Parameter update timing
    const uint64_t param_update_start = nanotime_now();
    for (int i = 0; i < 10000; i++) {
        // Update all parameters
        for (int p = 0; p < 10; p++) {
            expr_batch_set_variable(eval_builder, p, param_values[p] + i * 0.001);
        }
    }
    const uint64_t param_update_end = nanotime_now();
    const double param_update_us = nanotime_interval(param_update_start, param_update_end, now_max) / 1000.0 / 10000.0;
    printf("   Parameter update (10 params): %.3f µs\n", param_update_us);
    printf("   Per parameter update: %.3f µs\n", param_update_us / 10.0);
    
    // Combined update + eval
    const uint64_t combined_start = nanotime_now();
    for (int i = 0; i < 10000; i++) {
        // Update parameters
        for (int p = 0; p < 10; p++) {
            expr_batch_set_variable(eval_builder, p, param_values[p] + i * 0.001);
        }
        
        // Evaluate
        expr_batch_evaluate(eval_builder, ctx);
        
        // Use results
        for (int e = 0; e < 7; e++) {
            g_sink += expr_batch_get_result(eval_builder, e);
        }
    }
    const uint64_t combined_end = nanotime_now();
    const double combined_us = nanotime_interval(combined_start, combined_end, now_max) / 1000.0 / 10000.0;
    printf("   Update + eval cycle: %.3f µs\n", combined_us);
    
    // Cleanup evaluation builder
    expr_batch_free(eval_builder);
    expr_arena_free(eval_arena);
    
    printf("\n5. Individual Expression Timing\n");
    ExprArena* expr_arena = expr_arena_new(32768);
    
    for (int e = 0; e < 7; e++) {
        ExprBatch* expr_builder = expr_batch_new(expr_arena);
        
        // Add parameters
        for (int p = 0; p < 10; p++) {
            expr_batch_add_variable(expr_builder, param_names[p], param_values[p]);
        }
        
        // Add single expression
        expr_batch_add_expression(expr_builder, expressions[e]);
        
        // Time evaluation
        const uint64_t expr_start = nanotime_now();
        for (int i = 0; i < 10000; i++) {
            expr_batch_evaluate(expr_builder, ctx);
            g_sink += expr_batch_get_result(expr_builder, 0);
        }
        const uint64_t expr_end = nanotime_now();
        const double expr_us = nanotime_interval(expr_start, expr_end, now_max) / 1000.0 / 10000.0;
        
        printf("   \"%s\": %.3f µs\n", expressions[e], expr_us);
        
        expr_batch_free(expr_builder);
        expr_arena_reset(expr_arena);
    }
    
    expr_arena_free(expr_arena);
    
    printf("\n6. Summary\n");
    printf("   One-time setup: %.3f µs (context + arena)\n", ctx_us + arena_us);
    printf("   Parse expressions: %.3f µs (%.3f µs per expression)\n", 
           parse_total_us - builder_param_us, expr_parse_us);
    printf("   Runtime evaluation: %.3f µs (%.3f µs per expression)\n", 
           single_us, single_us / 7.0);
    printf("   Typical update cycle: %.3f µs (update params + eval)\n", combined_us);
    
    // Cleanup
    expr_context_free(ctx);
    
    // Print sink to prevent optimization
    printf("\n(Optimization prevention: %f)\n", g_sink);
    
    return 0;
}