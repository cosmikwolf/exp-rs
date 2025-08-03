#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
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

// Helper function to get time in microseconds
double get_time_us() {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1e6 + ts.tv_nsec / 1e3;
}

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
    printf("=== C FFI Setup Time Analysis ===\n\n");
    
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
    
    // 1. Context Creation Time
    printf("1. Context Creation Time\n");
    int iterations = 1000;
    double start = get_time_us();
    
    for (int i = 0; i < iterations; i++) {
        EvalContextOpaque* ctx = create_test_context();
        exp_rs_context_free(ctx);
    }
    
    double ctx_duration = get_time_us() - start;
    double ctx_us = ctx_duration / iterations;
    printf("   Average time: %.1f µs\n", ctx_us);
    
    // Create one context for subsequent tests
    EvalContextOpaque* ctx = create_test_context();
    
    // 2. BatchBuilder Creation Time
    printf("\n2. BatchBuilder Creation Time\n");
    start = get_time_us();
    
    for (int i = 0; i < iterations; i++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        exp_rs_batch_builder_free(builder);
    }
    
    double builder_duration = get_time_us() - start;
    double builder_us = builder_duration / iterations;
    printf("   Average time: %.1f µs\n", builder_us);
    
    // 3. Adding Parameters
    printf("\n3. Adding 10 Parameters\n");
    double total_param_time = 0.0;
    
    for (int iter = 0; iter < 100; iter++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        start = get_time_us();
        for (int i = 0; i < 10; i++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[i], param_values[i]);
        }
        double duration = get_time_us() - start;
        total_param_time += duration;
        
        exp_rs_batch_builder_free(builder);
    }
    
    double param_us = total_param_time / 100.0;
    printf("   Average time: %.1f µs\n", param_us);
    printf("   Per parameter: %.1f µs\n", param_us / 10.0);
    
    // 4. Parsing and Adding Expressions
    printf("\n4. Parsing and Adding 7 Expressions\n");
    double total_expr_time = 0.0;
    
    for (int iter = 0; iter < 100; iter++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        // Add parameters first
        for (int i = 0; i < 10; i++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[i], param_values[i]);
        }
        
        start = get_time_us();
        for (int i = 0; i < 7; i++) {
            exp_rs_batch_builder_add_expression(builder, expressions[i]);
        }
        double duration = get_time_us() - start;
        total_expr_time += duration;
        
        exp_rs_batch_builder_free(builder);
    }
    
    double expr_us = total_expr_time / 100.0;
    printf("   Average time: %.1f µs\n", expr_us);
    printf("   Per expression: %.1f µs\n", expr_us / 7.0);
    
    // 5. Complete Setup Time
    printf("\n5. Complete Setup Time\n");
    printf("   (Context + Builder + 10 params + 7 expressions + first eval)\n");
    
    double total_setup_time = 0.0;
    
    for (int iter = 0; iter < 100; iter++) {
        start = get_time_us();
        
        // Complete setup
        EvalContextOpaque* test_ctx = create_test_context();
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        // Add parameters
        for (int i = 0; i < 10; i++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[i], param_values[i]);
        }
        
        // Add expressions
        for (int i = 0; i < 7; i++) {
            exp_rs_batch_builder_add_expression(builder, expressions[i]);
        }
        
        // First evaluation
        exp_rs_batch_builder_eval(builder, test_ctx);
        
        double duration = get_time_us() - start;
        total_setup_time += duration;
        
        exp_rs_batch_builder_free(builder);
        exp_rs_context_free(test_ctx);
    }
    
    double total_us = total_setup_time / 100.0;
    printf("   Average time: %.1f µs\n", total_us);
    
    // 6. Breakdown
    printf("\n6. Setup Time Breakdown\n");
    printf("   Context creation:    %6.1f µs (%4.1f%%)\n", ctx_us, (ctx_us / total_us) * 100.0);
    printf("   Builder creation:    %6.1f µs (%4.1f%%)\n", builder_us, (builder_us / total_us) * 100.0);
    printf("   Add parameters:      %6.1f µs (%4.1f%%)\n", param_us, (param_us / total_us) * 100.0);
    printf("   Parse expressions:   %6.1f µs (%4.1f%%)\n", expr_us, (expr_us / total_us) * 100.0);
    double first_eval_us = total_us - (ctx_us + builder_us + param_us + expr_us);
    printf("   First evaluation:    %6.1f µs (%4.1f%%)\n", first_eval_us, (first_eval_us / total_us) * 100.0);
    printf("   ─────────────────────────────────────────\n");
    printf("   Total:               %6.1f µs\n", total_us);
    
    // 7. Amortization Analysis
    printf("\n7. Amortization Analysis\n");
    printf("   Setup cost: %.1f µs\n", total_us);
    printf("   Per-evaluation cost: ~16.0 µs (from previous tests)\n");
    printf("\n");
    int breakeven = (int)(total_us / 16.0) + 1;
    printf("   Break-even point: %d evaluations\n", breakeven);
    printf("   At 1000 Hz for 1 second:\n");
    printf("     Setup overhead: %.2f%%\n", (total_us / 1e6) * 100.0);
    printf("     Amortized setup cost: %.3f µs per evaluation\n", total_us / 1000.0);
    
    // 8. Memory Usage
    printf("\n8. Memory Usage\n");
    printf("   Context size: ~%zu bytes\n", sizeof(void*) * 50); // Estimate
    printf("   BatchBuilder overhead: ~%zu bytes\n", sizeof(void*) * 20); // Estimate
    printf("   Per expression: ~%zu bytes\n", sizeof(void*) * 10); // Estimate
    printf("   Per parameter: ~%zu bytes\n", sizeof(double) + sizeof(void*)); // Estimate
    
    // Cleanup
    exp_rs_context_free(ctx);
    
    return 0;
}