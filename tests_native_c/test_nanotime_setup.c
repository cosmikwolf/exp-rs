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

// Convert nanoseconds to microseconds
double ns_to_us(uint64_t ns) {
    return ns / 1000.0;
}

int main() {
    printf("=== C FFI Setup Time Analysis (using nanotime) ===\n\n");
    
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
    
    // 1. Context Creation Time
    printf("1. Context Creation Time (1000 iterations)\n");
    uint64_t total_ns = 0;
    
    for (int i = 0; i < 1000; i++) {
        uint64_t start = nanotime_now();
        EvalContextOpaque* ctx = create_test_context();
        uint64_t end = nanotime_now();
        total_ns += nanotime_interval(start, end, now_max);
        exp_rs_context_free(ctx);
    }
    
    double ctx_us = ns_to_us(total_ns) / 1000.0;
    printf("   Average time: %.3f µs\n", ctx_us);
    
    // Create one context for subsequent tests
    EvalContextOpaque* ctx = create_test_context();
    
    // 2. BatchBuilder Creation Time
    printf("\n2. BatchBuilder Creation Time (10000 iterations)\n");
    total_ns = 0;
    
    for (int i = 0; i < 10000; i++) {
        uint64_t start = nanotime_now();
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        uint64_t end = nanotime_now();
        total_ns += nanotime_interval(start, end, now_max);
        exp_rs_batch_builder_free(builder);
    }
    
    double builder_us = ns_to_us(total_ns) / 10000.0;
    printf("   Average time: %.3f µs\n", builder_us);
    
    // 3. Adding Parameters
    printf("\n3. Adding 10 Parameters (1000 iterations)\n");
    total_ns = 0;
    
    for (int iter = 0; iter < 1000; iter++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        uint64_t start = nanotime_now();
        for (int i = 0; i < 10; i++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[i], param_values[i]);
        }
        uint64_t end = nanotime_now();
        total_ns += nanotime_interval(start, end, now_max);
        
        exp_rs_batch_builder_free(builder);
    }
    
    double param_us = ns_to_us(total_ns) / 1000.0;
    printf("   Average time: %.3f µs\n", param_us);
    printf("   Per parameter: %.3f µs\n", param_us / 10.0);
    
    // 4. Parsing and Adding Expressions
    printf("\n4. Parsing and Adding 7 Expressions (1000 iterations)\n");
    total_ns = 0;
    
    for (int iter = 0; iter < 1000; iter++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        // Add parameters first
        for (int i = 0; i < 10; i++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[i], param_values[i]);
        }
        
        uint64_t start = nanotime_now();
        for (int i = 0; i < 7; i++) {
            exp_rs_batch_builder_add_expression(builder, expressions[i]);
        }
        uint64_t end = nanotime_now();
        total_ns += nanotime_interval(start, end, now_max);
        
        exp_rs_batch_builder_free(builder);
    }
    
    double expr_us = ns_to_us(total_ns) / 1000.0;
    printf("   Average time: %.3f µs\n", expr_us);
    printf("   Per expression: %.3f µs\n", expr_us / 7.0);
    
    // 5. Individual expression parsing
    printf("\n5. Individual Expression Parsing Times\n");
    for (int e = 0; e < 7; e++) {
        total_ns = 0;
        
        for (int iter = 0; iter < 1000; iter++) {
            BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
            
            // Add parameters
            for (int i = 0; i < 10; i++) {
                exp_rs_batch_builder_add_parameter(builder, param_names[i], param_values[i]);
            }
            
            uint64_t start = nanotime_now();
            exp_rs_batch_builder_add_expression(builder, expressions[e]);
            uint64_t end = nanotime_now();
            total_ns += nanotime_interval(start, end, now_max);
            
            exp_rs_batch_builder_free(builder);
        }
        
        double us = ns_to_us(total_ns) / 1000.0;
        printf("   Expression %d: %.3f µs - %.40s...\n", e + 1, us, expressions[e]);
    }
    
    // 6. First Evaluation Time
    printf("\n6. First Evaluation Time (1000 iterations)\n");
    total_ns = 0;
    
    for (int iter = 0; iter < 1000; iter++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        // Setup
        for (int i = 0; i < 10; i++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[i], param_values[i]);
        }
        for (int i = 0; i < 7; i++) {
            exp_rs_batch_builder_add_expression(builder, expressions[i]);
        }
        
        // Measure first eval
        uint64_t start = nanotime_now();
        exp_rs_batch_builder_eval(builder, ctx);
        uint64_t end = nanotime_now();
        total_ns += nanotime_interval(start, end, now_max);
        
        exp_rs_batch_builder_free(builder);
    }
    
    double first_eval_us = ns_to_us(total_ns) / 1000.0;
    printf("   Average time: %.3f µs\n", first_eval_us);
    
    // 7. Complete Setup Time
    printf("\n7. Complete Setup Time (100 iterations)\n");
    printf("   (Context + Builder + 10 params + 7 expressions + first eval)\n");
    total_ns = 0;
    
    for (int iter = 0; iter < 100; iter++) {
        uint64_t start = nanotime_now();
        
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
        
        uint64_t end = nanotime_now();
        total_ns += nanotime_interval(start, end, now_max);
        
        exp_rs_batch_builder_free(builder);
        exp_rs_context_free(test_ctx);
    }
    
    double total_us = ns_to_us(total_ns) / 100.0;
    printf("   Average time: %.3f µs\n", total_us);
    
    // 8. Subsequent Evaluation Time
    printf("\n8. Subsequent Evaluation Time (100000 iterations)\n");
    BatchBuilderOpaque* test_builder = exp_rs_batch_builder_new();
    
    // Setup
    for (int i = 0; i < 10; i++) {
        exp_rs_batch_builder_add_parameter(test_builder, param_names[i], param_values[i]);
    }
    for (int i = 0; i < 7; i++) {
        exp_rs_batch_builder_add_expression(test_builder, expressions[i]);
    }
    
    // Warm up
    for (int i = 0; i < 1000; i++) {
        exp_rs_batch_builder_eval(test_builder, ctx);
    }
    
    // Measure subsequent evaluations
    total_ns = 0;
    for (int i = 0; i < 100000; i++) {
        // Update params
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_set_param(test_builder, p, param_values[p] + i * 0.001);
        }
        
        uint64_t start = nanotime_now();
        exp_rs_batch_builder_eval(test_builder, ctx);
        uint64_t end = nanotime_now();
        total_ns += nanotime_interval(start, end, now_max);
    }
    
    double eval_us = ns_to_us(total_ns) / 100000.0;
    printf("   Average time: %.3f µs\n", eval_us);
    printf("   Rate: %.0f Hz\n", 1e6 / eval_us);
    
    exp_rs_batch_builder_free(test_builder);
    
    // 9. Breakdown
    printf("\n9. Setup Time Breakdown\n");
    printf("   Context creation:    %7.3f µs (%5.1f%%)\n", ctx_us, (ctx_us / total_us) * 100.0);
    printf("   Builder creation:    %7.3f µs (%5.1f%%)\n", builder_us, (builder_us / total_us) * 100.0);
    printf("   Add parameters:      %7.3f µs (%5.1f%%)\n", param_us, (param_us / total_us) * 100.0);
    printf("   Parse expressions:   %7.3f µs (%5.1f%%)\n", expr_us, (expr_us / total_us) * 100.0);
    printf("   First evaluation:    %7.3f µs (%5.1f%%)\n", first_eval_us, (first_eval_us / total_us) * 100.0);
    double overhead = total_us - (ctx_us + builder_us + param_us + expr_us + first_eval_us);
    printf("   Other overhead:      %7.3f µs (%5.1f%%)\n", overhead, (overhead / total_us) * 100.0);
    printf("   ──────────────────────────────────────────\n");
    printf("   Total:               %7.3f µs\n", total_us);
    
    // 10. Comparison with Rust
    printf("\n10. Comparison with Rust Implementation\n");
    printf("   Rust total setup: 33.5 µs\n");
    printf("   C FFI total setup: %.3f µs\n", total_us);
    printf("   C FFI is %.1fx %s than Rust\n", 
           total_us > 33.5 ? total_us / 33.5 : 33.5 / total_us,
           total_us > 33.5 ? "slower" : "faster");
    
    // Cleanup
    exp_rs_context_free(ctx);
    
    return 0;
}