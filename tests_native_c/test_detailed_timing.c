#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <time.h>
#include <math.h>
#include "exp_rs.h"

// Helper function to measure time in microseconds
static double get_time_us() {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1e6 + ts.tv_nsec / 1e3;
}

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
    return args[0] > 0.0 ? 1.0 : (args[0] < 0.0 ? -1.0 : 0.0); 
}
Real native_min(const Real* args, uintptr_t nargs) { (void)nargs; return args[0] < args[1] ? args[0] : args[1]; }
Real native_max(const Real* args, uintptr_t nargs) { (void)nargs; return args[0] > args[1] ? args[0] : args[1]; }
Real native_fmod(const Real* args, uintptr_t nargs) { (void)nargs; return fmod(args[0], args[1]); }

int main() {
    printf("=== Detailed Timing Analysis ===\n\n");
    
    // Create arena and context
    ExprArena* arena = expr_arena_new(512 * 1024);
    ExprContext* ctx = expr_context_new();
    
    // Register all functions
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
    
    ExprBatch* builder = expr_batch_new(arena);
    
    // Add the same 7 expressions
    const char* expressions[] = {
        "a*sin(b*3.14159/180) + c*cos(d*3.14159/180) + sqrt(e*e + f*f)",
        "exp(g/10) * log(h+1) + pow(i, 0.5) * j",
        "((a > 5) && (b < 10)) * c + ((d >= e) || (f != g)) * h + min(i, j)",
        "sqrt(pow(a-e, 2) + pow(b-f, 2)) + atan2(c-g, d-h) * (i+j)/2",
        "abs(a-b) * sign(c-d) + max(e, f) * min(g, h) + fmod(i*j, 10)",
        "(a+b+c)/3 * sin((d+e+f)*3.14159/6) + log10(g*h+1) - exp(-i*j/100)",
        "a + b * c - d / (e + 0.001) + pow(f, g) * h - i + j"
    };
    
    for (int i = 0; i < 7; i++) {
        expr_batch_add_expression(builder, expressions[i]);
    }
    
    // Add 10 parameters
    const char* param_names[] = {"a", "b", "c", "d", "e", "f", "g", "h", "i", "j"};
    for (int i = 0; i < 10; i++) {
        expr_batch_add_variable(builder, param_names[i], (i + 1) * 1.5);
    }
    
    // Initial evaluation
    expr_batch_evaluate(builder, ctx);
    
    printf("Warming up...\n");
    // Warm up
    for (int i = 0; i < 1000; i++) {
        for (int p = 0; p < 10; p++) {
            expr_batch_set_variable(builder, p, (p + 1) * 1.5);
        }
        expr_batch_evaluate(builder, ctx);
    }
    
    // Test 1: Time just the eval call
    printf("\nTest 1: Time just eval() call\n");
    const int eval_iterations = 100000;
    double start = get_time_us();
    
    for (int i = 0; i < eval_iterations; i++) {
        expr_batch_evaluate(builder, ctx);
    }
    
    double end = get_time_us();
    double eval_only_us = (end - start) / eval_iterations;
    printf("  Time per eval (7 expressions): %.3f µs\n", eval_only_us);
    printf("  Time per expression: %.3f µs\n", eval_only_us / 7.0);
    printf("  Evaluations per second: %.0f\n", 1e6 / eval_only_us);
    
    // Test 2: Time parameter updates only
    printf("\nTest 2: Time parameter updates only\n");
    start = get_time_us();
    
    for (int i = 0; i < eval_iterations; i++) {
        for (int p = 0; p < 10; p++) {
            expr_batch_set_variable(builder, p, (p + 1) * 1.5 + i * 0.001);
        }
    }
    
    end = get_time_us();
    double param_update_us = (end - start) / eval_iterations;
    printf("  Time for 10 param updates: %.3f µs\n", param_update_us);
    printf("  Time per param update: %.3f µs\n", param_update_us / 10.0);
    
    // Test 3: Time full cycle (params + eval)
    printf("\nTest 3: Time full cycle (params + eval)\n");
    const int full_iterations = 10000;
    start = get_time_us();
    
    for (int i = 0; i < full_iterations; i++) {
        // Update all 10 parameters
        for (int p = 0; p < 10; p++) {
            expr_batch_set_variable(builder, p, (p + 1) * 1.5 + i * 0.001);
        }
        // Evaluate
        expr_batch_evaluate(builder, ctx);
    }
    
    end = get_time_us();
    double full_cycle_us = (end - start) / full_iterations;
    printf("  Time per full cycle: %.3f µs\n", full_cycle_us);
    printf("  Rate: %.0f Hz\n", 1e6 / full_cycle_us);
    printf("  Breakdown:\n");
    printf("    Parameter updates: %.3f µs (%.1f%%)\n", 
           param_update_us, (param_update_us / full_cycle_us) * 100);
    printf("    Evaluation: %.3f µs (%.1f%%)\n", 
           eval_only_us, (eval_only_us / full_cycle_us) * 100);
    
    // Cleanup
    expr_batch_free(builder);
    expr_context_free(ctx);
    expr_arena_free(arena);
    
    return 0;
}