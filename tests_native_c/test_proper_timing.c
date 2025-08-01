#define NANOTIME_IMPLEMENTATION
#include "nanotime.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include "../include/exp_rs.h"

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

// Prevent optimization by using results
volatile double dummy_result = 0.0;

int main() {
    printf("=== C FFI Proper Timing Analysis ===\n\n");
    
    const uint64_t now_max = nanotime_now_max();
    
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
    
    // Test 1: Complete setup time - measure in bulk
    printf("1. Complete Setup Time (100 iterations)\n");
    
    const uint64_t setup_start = nanotime_now();
    
    for (int i = 0; i < 100; i++) {
        // Create context
        EvalContextOpaque* ctx = create_test_context();
        
        // Create builder
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        // Add parameters
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[p], param_values[p]);
        }
        
        // Add expressions
        for (int e = 0; e < 7; e++) {
            exp_rs_batch_builder_add_expression(builder, expressions[e]);
        }
        
        // First evaluation
        exp_rs_batch_builder_eval(builder, ctx);
        
        // Use results to prevent optimization
        for (int r = 0; r < 7; r++) {
            dummy_result += exp_rs_batch_builder_get_result(builder, r);
        }
        
        // Cleanup
        exp_rs_batch_builder_free(builder);
        exp_rs_context_free(ctx);
    }
    
    const uint64_t setup_end = nanotime_now();
    const uint64_t setup_interval = nanotime_interval(setup_start, setup_end, now_max);
    const double setup_us = setup_interval / 1000.0 / 100.0;
    printf("   Average setup time: %.3f µs\n", setup_us);
    
    // Test 2: Measure components separately with larger batches
    printf("\n2. Component Timing (1000 iterations each)\n");
    
    // Context creation
    const uint64_t ctx_start = nanotime_now();
    for (int i = 0; i < 1000; i++) {
        EvalContextOpaque* ctx = create_test_context();
        exp_rs_context_free(ctx);
    }
    const uint64_t ctx_end = nanotime_now();
    const double ctx_us = nanotime_interval(ctx_start, ctx_end, now_max) / 1000.0 / 1000.0;
    printf("   Context creation: %.3f µs\n", ctx_us);
    
    // Create context for remaining tests
    EvalContextOpaque* ctx = create_test_context();
    
    // Builder creation
    const uint64_t builder_start = nanotime_now();
    for (int i = 0; i < 10000; i++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        exp_rs_batch_builder_free(builder);
    }
    const uint64_t builder_end = nanotime_now();
    const double builder_us = nanotime_interval(builder_start, builder_end, now_max) / 1000.0 / 10000.0;
    printf("   Builder creation: %.3f µs\n", builder_us);
    
    // Test 3: Expression parsing time
    printf("\n3. Expression Parsing Time\n");
    
    // Measure the entire builder setup + expression parsing
    const uint64_t parse_start = nanotime_now();
    
    for (int i = 0; i < 1000; i++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        // Add parameters
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[p], param_values[p]);
        }
        
        // Add expressions - this is what we're really timing
        for (int e = 0; e < 7; e++) {
            exp_rs_batch_builder_add_expression(builder, expressions[e]);
        }
        
        exp_rs_batch_builder_free(builder);
    }
    
    const uint64_t parse_end = nanotime_now();
    const double total_parse_us = nanotime_interval(parse_start, parse_end, now_max) / 1000.0 / 1000.0;
    
    // Now measure just builder + params (no expressions)
    const uint64_t builder_param_start = nanotime_now();
    
    for (int i = 0; i < 1000; i++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        // Add parameters only
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[p], param_values[p]);
        }
        
        exp_rs_batch_builder_free(builder);
    }
    
    const uint64_t builder_param_end = nanotime_now();
    const double builder_param_us = nanotime_interval(builder_param_start, builder_param_end, now_max) / 1000.0 / 1000.0;
    
    const double expr_parse_us = total_parse_us - builder_param_us;
    printf("   Builder + params: %.3f µs\n", builder_param_us);
    printf("   Expression parsing (7 expressions): %.3f µs\n", expr_parse_us);
    printf("   Per expression: %.3f µs\n", expr_parse_us / 7.0);
    
    // Test 4: Runtime evaluation
    printf("\n4. Runtime Evaluation Performance\n");
    
    // Setup a builder for evaluation tests
    BatchBuilderOpaque* eval_builder = exp_rs_batch_builder_new();
    for (int p = 0; p < 10; p++) {
        exp_rs_batch_builder_add_parameter(eval_builder, param_names[p], param_values[p]);
    }
    for (int e = 0; e < 7; e++) {
        exp_rs_batch_builder_add_expression(eval_builder, expressions[e]);
    }
    
    // Warm up
    for (int i = 0; i < 1000; i++) {
        exp_rs_batch_builder_eval(eval_builder, ctx);
    }
    
    // Measure evaluation only
    const uint64_t eval_start = nanotime_now();
    
    for (int i = 0; i < 100000; i++) {
        exp_rs_batch_builder_eval(eval_builder, ctx);
        
        // Use results to prevent optimization
        if (i % 1000 == 0) {
            for (int r = 0; r < 7; r++) {
                dummy_result += exp_rs_batch_builder_get_result(eval_builder, r);
            }
        }
    }
    
    const uint64_t eval_end = nanotime_now();
    const double eval_us = nanotime_interval(eval_start, eval_end, now_max) / 1000.0 / 100000.0;
    printf("   Evaluation time (7 expressions): %.3f µs\n", eval_us);
    printf("   Per expression: %.3f µs\n", eval_us / 7.0);
    printf("   Rate: %.0f Hz\n", 1e6 / eval_us);
    
    // Test 5: Full cycle with parameter updates
    printf("\n5. Full Cycle (params + eval)\n");
    
    const uint64_t full_start = nanotime_now();
    
    for (int i = 0; i < 100000; i++) {
        // Update all parameters
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_set_param(eval_builder, p, param_values[p] + (i % 100) * 0.01);
        }
        
        // Evaluate
        exp_rs_batch_builder_eval(eval_builder, ctx);
        
        // Use results periodically
        if (i % 1000 == 0) {
            for (int r = 0; r < 7; r++) {
                dummy_result += exp_rs_batch_builder_get_result(eval_builder, r);
            }
        }
    }
    
    const uint64_t full_end = nanotime_now();
    const double full_us = nanotime_interval(full_start, full_end, now_max) / 1000.0 / 100000.0;
    const double param_us = full_us - eval_us;
    
    printf("   Full cycle time: %.3f µs\n", full_us);
    printf("   Parameter update time: %.3f µs\n", param_us);
    printf("   Rate: %.0f Hz\n", 1e6 / full_us);
    
    // Summary
    printf("\n6. Summary\n");
    printf("   Complete setup: %.3f µs\n", setup_us);
    printf("   ├─ Context creation: %.3f µs (%.1f%%)\n", ctx_us, (ctx_us / setup_us) * 100);
    printf("   ├─ Builder creation: %.3f µs (%.1f%%)\n", builder_us, (builder_us / setup_us) * 100);
    printf("   ├─ Add parameters: %.3f µs (%.1f%%)\n", builder_param_us - builder_us, ((builder_param_us - builder_us) / setup_us) * 100);
    printf("   ├─ Parse expressions: %.3f µs (%.1f%%)\n", expr_parse_us, (expr_parse_us / setup_us) * 100);
    printf("   └─ First evaluation: %.3f µs (%.1f%%)\n", eval_us, (eval_us / setup_us) * 100);
    
    printf("\n7. Comparison with Rust\n");
    printf("   Rust setup: 33.5 µs\n");
    printf("   C FFI setup: %.3f µs\n", setup_us);
    printf("   \n");
    printf("   Rust evaluation: 13.7 µs\n");
    printf("   C FFI evaluation: %.3f µs\n", eval_us);
    
    // Sanity check
    printf("\n8. Sanity Check\n");
    printf("   Dummy result (prevent optimization): %.6f\n", dummy_result);
    
    if (expr_parse_us < 5.0) {
        printf("\n   WARNING: Expression parsing seems too fast (%.3f µs).\n", expr_parse_us);
        printf("   This might indicate deferred parsing or measurement issues.\n");
    }
    
    // Cleanup
    exp_rs_batch_builder_free(eval_builder);
    exp_rs_context_free(ctx);
    
    return 0;
}