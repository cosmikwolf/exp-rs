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
    printf("=== C FFI Detailed Timing Analysis ===\n\n");
    
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
    
    // Create context once
    EvalContextOpaque* ctx = create_test_context();
    
    // Test 1: Measure individual expression parsing more carefully
    printf("1. Individual Expression Parsing (100000 iterations each)\n");
    
    for (int e = 0; e < 7; e++) {
        uint64_t total_time = 0;
        
        // Do multiple runs to get stable measurement
        for (int run = 0; run < 100000; run++) {
            BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
            
            // Add parameters
            for (int p = 0; p < 10; p++) {
                exp_rs_batch_builder_add_parameter(builder, param_names[p], param_values[p]);
            }
            
            // Measure just this one expression
            uint64_t start = nanotime_now();
            int result = exp_rs_batch_builder_add_expression(builder, expressions[e]);
            uint64_t end = nanotime_now();
            
            if (result >= 0) {
                total_time += nanotime_interval(start, end, now_max);
            }
            
            exp_rs_batch_builder_free(builder);
        }
        
        double avg_ns = total_time / 100000.0;
        double avg_us = avg_ns / 1000.0;
        printf("   Expression %d: %.6f µs (%.3f ns)\n", e + 1, avg_us, avg_ns);
    }
    
    // Test 2: Try to isolate parsing time by comparing simple vs complex expressions
    printf("\n2. Simple vs Complex Expression Parsing\n");
    
    const char* simple_exprs[] = {
        "a",
        "a + b",
        "a + b + c"
    };
    
    const char* complex_exprs[] = {
        "sin(a)",
        "sin(a) + cos(b)",
        "a*sin(b*3.14159/180) + c*cos(d*3.14159/180) + sqrt(e*e + f*f)"
    };
    
    for (int i = 0; i < 3; i++) {
        uint64_t simple_time = 0;
        uint64_t complex_time = 0;
        
        // Measure simple expression
        for (int run = 0; run < 100000; run++) {
            BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
            for (int p = 0; p < 10; p++) {
                exp_rs_batch_builder_add_parameter(builder, param_names[p], param_values[p]);
            }
            
            uint64_t start = nanotime_now();
            exp_rs_batch_builder_add_expression(builder, simple_exprs[i]);
            uint64_t end = nanotime_now();
            simple_time += nanotime_interval(start, end, now_max);
            
            exp_rs_batch_builder_free(builder);
        }
        
        // Measure complex expression
        for (int run = 0; run < 100000; run++) {
            BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
            for (int p = 0; p < 10; p++) {
                exp_rs_batch_builder_add_parameter(builder, param_names[p], param_values[p]);
            }
            
            uint64_t start = nanotime_now();
            exp_rs_batch_builder_add_expression(builder, complex_exprs[i]);
            uint64_t end = nanotime_now();
            complex_time += nanotime_interval(start, end, now_max);
            
            exp_rs_batch_builder_free(builder);
        }
        
        double simple_us = simple_time / 1000.0 / 100000.0;
        double complex_us = complex_time / 1000.0 / 100000.0;
        
        printf("   Level %d:\n", i + 1);
        printf("     Simple: %.3f µs - %s\n", simple_us, simple_exprs[i]);
        printf("     Complex: %.3f µs - %.40s...\n", complex_us, complex_exprs[i]);
        printf("     Difference: %.3f µs\n", complex_us - simple_us);
    }
    
    // Test 3: Measure in larger batches to accumulate measurable time
    printf("\n3. Batch Measurement (parse 7 expressions 100000 times)\n");
    
    uint64_t batch_start = nanotime_now();
    
    for (int i = 0; i < 100000; i++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        // Add parameters
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[p], param_values[p]);
        }
        
        // Add all 7 expressions
        for (int e = 0; e < 7; e++) {
            exp_rs_batch_builder_add_expression(builder, expressions[e]);
        }
        
        exp_rs_batch_builder_free(builder);
    }
    
    uint64_t batch_end = nanotime_now();
    uint64_t batch_total = nanotime_interval(batch_start, batch_end, now_max);
    double batch_ms = batch_total / 1000000.0;
    double per_iteration_us = batch_total / 1000.0 / 100000.0;
    double per_expression_us = per_iteration_us / 7.0;
    
    printf("   Total time: %.3f ms\n", batch_ms);
    printf("   Per iteration (7 expressions): %.3f µs\n", per_iteration_us);
    printf("   Per expression: %.3f µs\n", per_expression_us);
    
    // Cleanup
    exp_rs_context_free(ctx);
    
    printf("\n4. Analysis\n");
    if (per_expression_us < 1.0) {
        printf("   Expression parsing appears to be very fast (< 1 µs per expression).\n");
        printf("   This could indicate:\n");
        printf("   - Highly optimized parsing in Rust\n");
        printf("   - Simple expression structure allowing fast parsing\n");
        printf("   - Possible caching or optimization in the FFI layer\n");
    } else {
        printf("   Expression parsing takes %.3f µs per expression.\n", per_expression_us);
        printf("   This is reasonable for parsing mathematical expressions.\n");
    }
    
    return 0;
}
