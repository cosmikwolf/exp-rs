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

int main() {
    printf("=== C FFI Realistic Timing Analysis ===\n");
    printf("Measuring bulk operations for accurate timings\n\n");
    
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
    
    // Test 1: Complete setup operations
    printf("1. Complete Setup Time (1000 iterations)\n");
    
    uint64_t start = nanotime_now();
    
    for (int i = 0; i < 1000; i++) {
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
        
        // Cleanup
        exp_rs_batch_builder_free(builder);
        exp_rs_context_free(ctx);
    }
    
    uint64_t end = nanotime_now();
    uint64_t total_ns = nanotime_interval(start, end, now_max);
    double setup_us = total_ns / 1000.0 / 1000.0;
    printf("   Average setup time: %.3f µs\n", setup_us);
    
    // Test 2: Expression parsing in bulk
    printf("\n2. Expression Parsing Only (10000 iterations)\n");
    EvalContextOpaque* ctx = create_test_context();
    
    start = nanotime_now();
    
    for (int i = 0; i < 10000; i++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        // Add parameters first
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[p], param_values[p]);
        }
        
        // Parse all expressions - this is what we're timing
        for (int e = 0; e < 7; e++) {
            exp_rs_batch_builder_add_expression(builder, expressions[e]);
        }
        
        exp_rs_batch_builder_free(builder);
    }
    
    end = nanotime_now();
    total_ns = nanotime_interval(start, end, now_max);
    double parse_total_us = total_ns / 10000.0 / 1000.0;
    printf("   Time to setup builder + parse 7 expressions: %.3f µs\n", parse_total_us);
    
    // Test 3: Just builder setup overhead
    printf("\n3. Builder Setup Overhead (10000 iterations)\n");
    
    start = nanotime_now();
    
    for (int i = 0; i < 10000; i++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        // Add parameters only
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[p], param_values[p]);
        }
        
        exp_rs_batch_builder_free(builder);
    }
    
    end = nanotime_now();
    total_ns = nanotime_interval(start, end, now_max);
    double builder_overhead_us = total_ns / 10000.0 / 1000.0;
    printf("   Builder + parameters time: %.3f µs\n", builder_overhead_us);
    printf("   Expression parsing only: %.3f µs\n", parse_total_us - builder_overhead_us);
    printf("   Per expression: %.3f µs\n", (parse_total_us - builder_overhead_us) / 7.0);
    
    // Test 4: Runtime evaluation performance
    printf("\n4. Runtime Performance (100000 iterations)\n");
    
    // Setup once
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
    
    // Time evaluation only
    start = nanotime_now();
    
    for (int i = 0; i < 100000; i++) {
        exp_rs_batch_builder_eval(eval_builder, ctx);
    }
    
    end = nanotime_now();
    total_ns = nanotime_interval(start, end, now_max);
    double eval_us = total_ns / 100000.0 / 1000.0;
    printf("   Evaluation only: %.3f µs\n", eval_us);
    printf("   Per expression: %.3f µs\n", eval_us / 7.0);
    printf("   Rate: %.0f Hz\n", 1e6 / eval_us);
    
    // Test 5: Full cycle with parameter updates
    printf("\n5. Full Cycle Performance (100000 iterations)\n");
    
    start = nanotime_now();
    
    for (int i = 0; i < 100000; i++) {
        // Update all parameters
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_set_param(eval_builder, p, param_values[p] + (i % 100) * 0.01);
        }
        
        // Evaluate
        exp_rs_batch_builder_eval(eval_builder, ctx);
    }
    
    end = nanotime_now();
    total_ns = nanotime_interval(start, end, now_max);
    double full_us = total_ns / 100000.0 / 1000.0;
    printf("   Full cycle: %.3f µs\n", full_us);
    printf("   Parameter updates: %.3f µs\n", full_us - eval_us);
    printf("   Rate: %.0f Hz\n", 1e6 / full_us);
    
    // Summary and comparison
    printf("\n6. Summary\n");
    printf("   Complete setup: %.3f µs\n", setup_us);
    printf("   Expression parsing: %.3f µs (%.3f µs per expression)\n", 
           parse_total_us - builder_overhead_us, 
           (parse_total_us - builder_overhead_us) / 7.0);
    printf("   Evaluation only: %.3f µs\n", eval_us);
    printf("   Full cycle: %.3f µs\n", full_us);
    
    printf("\n7. Setup Time Breakdown\n");
    double ctx_creation = 3.0;  // Estimate from previous tests
    double builder_creation = builder_overhead_us;
    double expr_parsing = parse_total_us - builder_overhead_us;
    double first_eval = eval_us;
    double other = setup_us - ctx_creation - builder_creation - expr_parsing - first_eval;
    
    printf("   Context creation: %.3f µs (%.1f%%)\n", ctx_creation, (ctx_creation / setup_us) * 100);
    printf("   Builder + params: %.3f µs (%.1f%%)\n", builder_creation, (builder_creation / setup_us) * 100);
    printf("   Expression parsing: %.3f µs (%.1f%%)\n", expr_parsing, (expr_parsing / setup_us) * 100);
    printf("   First evaluation: %.3f µs (%.1f%%)\n", first_eval, (first_eval / setup_us) * 100);
    printf("   Other overhead: %.3f µs (%.1f%%)\n", other, (other / setup_us) * 100);
    
    printf("\n8. Comparison with Rust\n");
    printf("   Rust setup: 33.5 µs\n");
    printf("   C FFI setup: %.3f µs\n", setup_us);
    printf("   Rust evaluation: 13.7 µs\n");
    printf("   C FFI evaluation: %.3f µs\n", eval_us);
    
    // The real story
    if (expr_parsing < 1.0) {
        printf("\n   NOTE: Expression parsing time seems unrealistically low (%.3f µs).\n", expr_parsing);
        printf("   This suggests the FFI may be deferring parsing work until evaluation.\n");
        printf("   The \"setup\" time may not include all initialization work.\n");
    }
    
    // Cleanup
    exp_rs_batch_builder_free(eval_builder);
    exp_rs_context_free(ctx);
    
    return 0;
}