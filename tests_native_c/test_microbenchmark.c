#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <math.h>
#include <stdint.h>
#include "../include/exp_rs.h"

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
    printf("=== C FFI Microbenchmarks ===\n\n");
    
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
    
    // Test 1: Parse single expression timing
    printf("1. Single Expression Parsing\n");
    EvalContextOpaque* ctx = create_test_context();
    
    for (int e = 0; e < 7; e++) {
        uint64_t total_ns = 0;
        int iterations = 1000;
        
        for (int i = 0; i < iterations; i++) {
            BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
            
            // Add parameters
            for (int p = 0; p < 10; p++) {
                exp_rs_batch_builder_add_parameter(builder, param_names[p], param_values[p]);
            }
            
            // Time just the expression parsing
            uint64_t start = get_time_ns();
            exp_rs_batch_builder_add_expression(builder, expressions[e]);
            uint64_t end = get_time_ns();
            
            total_ns += (end - start);
            exp_rs_batch_builder_free(builder);
        }
        
        double avg_us = ns_to_us(total_ns) / iterations;
        printf("   Expression %d: %.3f µs - %.50s...\n", e + 1, avg_us, expressions[e]);
    }
    
    // Test 2: All 7 expressions together
    printf("\n2. All 7 Expressions Parsing\n");
    uint64_t total_all_ns = 0;
    int iterations = 1000;
    
    for (int i = 0; i < iterations; i++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        // Add parameters
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[p], param_values[p]);
        }
        
        // Time all expressions
        uint64_t start = get_time_ns();
        for (int e = 0; e < 7; e++) {
            exp_rs_batch_builder_add_expression(builder, expressions[e]);
        }
        uint64_t end = get_time_ns();
        
        total_all_ns += (end - start);
        exp_rs_batch_builder_free(builder);
    }
    
    double avg_all_us = ns_to_us(total_all_ns) / iterations;
    printf("   Total for 7 expressions: %.3f µs\n", avg_all_us);
    printf("   Average per expression: %.3f µs\n", avg_all_us / 7.0);
    
    // Test 3: Parameter setting timing
    printf("\n3. Parameter Setting\n");
    BatchBuilderOpaque* test_builder = exp_rs_batch_builder_new();
    
    // Add parameters and expressions first
    for (int p = 0; p < 10; p++) {
        exp_rs_batch_builder_add_parameter(test_builder, param_names[p], param_values[p]);
    }
    for (int e = 0; e < 7; e++) {
        exp_rs_batch_builder_add_expression(test_builder, expressions[e]);
    }
    
    // Time parameter updates
    uint64_t param_total_ns = 0;
    iterations = 10000;
    
    for (int i = 0; i < iterations; i++) {
        uint64_t start = get_time_ns();
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_set_param(test_builder, p, param_values[p] + i * 0.001);
        }
        uint64_t end = get_time_ns();
        param_total_ns += (end - start);
    }
    
    double param_us = ns_to_us(param_total_ns) / iterations;
    printf("   Time for 10 param updates: %.3f µs\n", param_us);
    printf("   Per parameter: %.3f µs\n", param_us / 10.0);
    
    // Test 4: Evaluation timing
    printf("\n4. Evaluation Timing\n");
    uint64_t eval_total_ns = 0;
    iterations = 10000;
    
    for (int i = 0; i < iterations; i++) {
        uint64_t start = get_time_ns();
        exp_rs_batch_builder_eval(test_builder, ctx);
        uint64_t end = get_time_ns();
        eval_total_ns += (end - start);
    }
    
    double eval_us = ns_to_us(eval_total_ns) / iterations;
    printf("   Evaluation time (7 expressions): %.3f µs\n", eval_us);
    printf("   Per expression: %.3f µs\n", eval_us / 7.0);
    
    // Test 5: Full cycle timing
    printf("\n5. Full Cycle (params + eval)\n");
    uint64_t full_total_ns = 0;
    iterations = 10000;
    
    for (int i = 0; i < iterations; i++) {
        uint64_t start = get_time_ns();
        
        // Update params
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_set_param(test_builder, p, param_values[p] + i * 0.001);
        }
        
        // Evaluate
        exp_rs_batch_builder_eval(test_builder, ctx);
        
        uint64_t end = get_time_ns();
        full_total_ns += (end - start);
    }
    
    double full_us = ns_to_us(full_total_ns) / iterations;
    printf("   Full cycle time: %.3f µs\n", full_us);
    printf("   Rate: %.0f Hz\n", 1e6 / full_us);
    
    // Summary
    printf("\n6. Summary\n");
    printf("   Expression parsing: %.3f µs total\n", avg_all_us);
    printf("   Parameter updates: %.3f µs\n", param_us);
    printf("   Evaluation: %.3f µs\n", eval_us);
    printf("   Expected full cycle: %.3f µs\n", param_us + eval_us);
    printf("   Actual full cycle: %.3f µs\n", full_us);
    printf("   Overhead: %.3f µs (%.1f%%)\n", 
           full_us - (param_us + eval_us), 
           ((full_us - (param_us + eval_us)) / full_us) * 100.0);
    
    // Cleanup
    exp_rs_batch_builder_free(test_builder);
    exp_rs_context_free(ctx);
    
    return 0;
}