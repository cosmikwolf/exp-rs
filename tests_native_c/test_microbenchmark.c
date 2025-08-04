#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <math.h>
#include <stdint.h>
#include "exp_rs.h"

#ifdef __APPLE__
#include <mach/mach_time.h>
#endif

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

// High-resolution timer functions
uint64_t get_time_ns() {
#ifdef __APPLE__
    return mach_absolute_time();
#else
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000000000ULL + ts.tv_nsec;
#endif
}

double ns_to_us(uint64_t ns) {
#ifdef __APPLE__
    mach_timebase_info_data_t timebase;
    mach_timebase_info(&timebase);
    return (double)ns * timebase.numer / timebase.denom / 1000.0;
#else
    return (double)ns / 1000.0;
#endif
}

int main() {
    printf("=== Expression Evaluation Microbenchmark ===\n\n");
    
    // Create context with native functions
    ExprContext* ctx = expr_context_new();
    
    // Register native functions
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
    
    // Test expressions
    const char* expressions[] = {
        "p0 + p1",
        "p0 * p1 + p2",
        "sqrt(p0*p0 + p1*p1)",
        "sin(p0) * cos(p1)",
        "log10(abs(p5) + 1) * p6",
        "pow(p7, 2) + pow(p8, 2) + pow(p9, 2)",
        "(exp(p0) - exp(-p0)) / 2"  // sinh manually
    };
    int num_expressions = sizeof(expressions) / sizeof(expressions[0]);
    
    const char* param_names[] = {"p0", "p1", "p2", "p3", "p4", "p5", "p6", "p7", "p8", "p9"};
    double param_values[] = {1.5, 2.3, 3.7, 0.5, 1.2, -0.8, 2.1, 0.9, 1.4, 0.7};
    
    // Test 1: Expression parsing overhead with arena reuse
    printf("1. Expression Parsing (with Arena Reuse)\n");
    {
        int iterations = 1000;
        
        // Create arena once for all iterations
        ExprArena* arena = expr_arena_new(32768); // 32KB arena
        
        uint64_t start = get_time_ns();
        
        for (int i = 0; i < iterations; i++) {
            // Reset arena for reuse
            if (i > 0) {
                expr_arena_reset(arena);
            }
            
            ExprBatch* builder = expr_batch_new(arena);
            
            // Add parameters
            for (int p = 0; p < 10; p++) {
                expr_batch_add_variable(builder, param_names[p], param_values[p]);
            }
            
            // Add and parse expressions
            for (int e = 0; e < num_expressions; e++) {
                expr_batch_add_expression(builder, expressions[e]);
            }
            
            expr_batch_free(builder);
        }
        
        uint64_t elapsed = get_time_ns() - start;
        
        expr_arena_free(arena);
        
        printf("  Total: %.2f us for %d iterations\n", ns_to_us(elapsed), iterations);
        printf("  Per iteration: %.2f us\n", ns_to_us(elapsed) / iterations);
        printf("  Per expression: %.2f us\n\n", ns_to_us(elapsed) / (iterations * num_expressions));
    }
    
    // Test 2: Full cycle (parse + eval) with arena reuse
    printf("2. Full Cycle (Parse + Evaluate) with Arena Reuse\n");
    {
        int iterations = 1000;
        
        // Create arena once for all iterations
        ExprArena* arena = expr_arena_new(32768); // 32KB arena
        
        uint64_t start = get_time_ns();
        
        for (int i = 0; i < iterations; i++) {
            // Reset arena for reuse
            if (i > 0) {
                expr_arena_reset(arena);
            }
            
            ExprBatch* builder = expr_batch_new(arena);
            
            // Add parameters
            for (int p = 0; p < 10; p++) {
                expr_batch_add_variable(builder, param_names[p], param_values[p]);
            }
            
            // Add expressions
            for (int e = 0; e < num_expressions; e++) {
                expr_batch_add_expression(builder, expressions[e]);
            }
            
            // Evaluate
            expr_batch_evaluate(builder, ctx);
            
            expr_batch_free(builder);
        }
        
        uint64_t elapsed = get_time_ns() - start;
        
        expr_arena_free(arena);
        
        printf("  Total: %.2f us for %d iterations\n", ns_to_us(elapsed), iterations);
        printf("  Per iteration: %.2f us\n", ns_to_us(elapsed) / iterations);
        printf("  Per expression: %.2f us\n\n", ns_to_us(elapsed) / (iterations * num_expressions));
    }
    
    // Test 3: Parameter setting timing
    printf("3. Parameter Setting\n");
    {
        ExprArena* arena = expr_arena_new(32768);
        ExprBatch* test_builder = expr_batch_new(arena);
        
        // Add parameters and expressions first
        for (int p = 0; p < 10; p++) {
            expr_batch_add_variable(test_builder, param_names[p], param_values[p]);
        }
        for (int e = 0; e < num_expressions; e++) {
            expr_batch_add_expression(test_builder, expressions[e]);
        }
        
        int iterations = 10000;
        uint64_t start = get_time_ns();
        
        for (int i = 0; i < iterations; i++) {
            // Set parameters by index (fastest method)
            for (int p = 0; p < 10; p++) {
                expr_batch_set_variable(test_builder, p, param_values[p] + i * 0.001);
            }
        }
        
        uint64_t elapsed = get_time_ns() - start;
        
        printf("  Total: %.2f us for %d iterations\n", ns_to_us(elapsed), iterations);
        printf("  Per iteration: %.2f us\n", ns_to_us(elapsed) / iterations);
        printf("  Per parameter: %.2f us\n\n", ns_to_us(elapsed) / (iterations * 10));
        
        expr_batch_free(test_builder);
        expr_arena_free(arena);
    }
    
    // Test 4: Pure evaluation timing (no parsing)
    printf("4. Pure Evaluation (Pre-parsed)\n");
    {
        ExprArena* arena = expr_arena_new(32768);
        ExprBatch* builder = expr_batch_new(arena);
        
        // Setup
        for (int p = 0; p < 10; p++) {
            expr_batch_add_variable(builder, param_names[p], param_values[p]);
        }
        for (int e = 0; e < num_expressions; e++) {
            expr_batch_add_expression(builder, expressions[e]);
        }
        
        int iterations = 10000;
        uint64_t start = get_time_ns();
        
        for (int i = 0; i < iterations; i++) {
            expr_batch_evaluate(builder, ctx);
        }
        
        uint64_t elapsed = get_time_ns() - start;
        
        printf("  Total: %.2f us for %d iterations\n", ns_to_us(elapsed), iterations);
        printf("  Per iteration: %.2f us\n", ns_to_us(elapsed) / iterations);
        printf("  Per expression: %.2f us\n\n", ns_to_us(elapsed) / (iterations * num_expressions));
        
        expr_batch_free(builder);
        expr_arena_free(arena);
    }
    
    // Test 5: Individual expression timing
    printf("5. Individual Expression Performance\n");
    {
        ExprArena* arena = expr_arena_new(32768);
        
        for (int e = 0; e < num_expressions; e++) {
            ExprBatch* builder = expr_batch_new(arena);
            
            // Setup
            for (int p = 0; p < 10; p++) {
                expr_batch_add_variable(builder, param_names[p], param_values[p]);
            }
            expr_batch_add_expression(builder, expressions[e]);
            
            int iterations = 10000;
            uint64_t start = get_time_ns();
            
            for (int i = 0; i < iterations; i++) {
                expr_batch_evaluate(builder, ctx);
            }
            
            uint64_t elapsed = get_time_ns() - start;
            
            printf("  \"%s\": %.2f us/eval\n", 
                   expressions[e], 
                   ns_to_us(elapsed) / iterations);
            
            expr_batch_free(builder);
            expr_arena_reset(arena);
        }
        
        expr_arena_free(arena);
    }
    
    // Cleanup
    expr_context_free(ctx);
    
    return 0;
}