#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <math.h>
#include <sys/time.h>
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

// High precision timer using gettimeofday
double get_time() {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return tv.tv_sec + tv.tv_usec / 1e6;
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
    printf("=== C FFI Bulk Timing Analysis ===\n");
    printf("Using accumulated time over many iterations for accuracy\n\n");
    
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
    
    // Test 1: Setup operations in bulk
    printf("1. Setup Operations (1000 complete setups)\n");
    double start_time = get_time();
    
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
    
    double setup_time = get_time() - start_time;
    double setup_us = (setup_time * 1e6) / 1000.0;
    printf("   Average complete setup time: %.3f µs\n", setup_us);
    
    // Test 2: Parse expressions separately
    printf("\n2. Expression Parsing Only (10000 iterations)\n");
    EvalContextOpaque* ctx = create_test_context();
    
    start_time = get_time();
    
    for (int i = 0; i < 10000; i++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        // Add parameters first
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[p], param_values[p]);
        }
        
        // Parse all expressions
        for (int e = 0; e < 7; e++) {
            exp_rs_batch_builder_add_expression(builder, expressions[e]);
        }
        
        exp_rs_batch_builder_free(builder);
    }
    
    double parse_time = get_time() - start_time;
    double parse_us = (parse_time * 1e6) / 10000.0;
    printf("   Average time to parse 7 expressions: %.3f µs\n", parse_us);
    printf("   Per expression: %.3f µs\n", parse_us / 7.0);
    
    // Test 3: Runtime evaluation
    printf("\n3. Runtime Evaluation (100000 iterations)\n");
    
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
    start_time = get_time();
    
    for (int i = 0; i < 100000; i++) {
        exp_rs_batch_builder_eval(eval_builder, ctx);
    }
    
    double eval_time = get_time() - start_time;
    double eval_us = (eval_time * 1e6) / 100000.0;
    printf("   Average evaluation time (7 expressions): %.3f µs\n", eval_us);
    printf("   Per expression: %.3f µs\n", eval_us / 7.0);
    printf("   Evaluation rate: %.0f Hz\n", 1e6 / eval_us);
    
    // Test 4: Full cycle with parameter updates
    printf("\n4. Full Cycle (params + eval, 100000 iterations)\n");
    
    start_time = get_time();
    
    for (int i = 0; i < 100000; i++) {
        // Update all parameters
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_set_param(eval_builder, p, param_values[p] + (i % 100) * 0.01);
        }
        
        // Evaluate
        exp_rs_batch_builder_eval(eval_builder, ctx);
    }
    
    double full_time = get_time() - start_time;
    double full_us = (full_time * 1e6) / 100000.0;
    printf("   Average full cycle time: %.3f µs\n", full_us);
    printf("   Parameter update time: %.3f µs\n", full_us - eval_us);
    printf("   Full cycle rate: %.0f Hz\n", 1e6 / full_us);
    
    // Summary
    printf("\n5. Summary\n");
    printf("   Complete setup (context + builder + parse + first eval): %.3f µs\n", setup_us);
    printf("   Expression parsing only: %.3f µs\n", parse_us);
    printf("   Evaluation only: %.3f µs\n", eval_us);
    printf("   Parameter updates: %.3f µs\n", full_us - eval_us);
    printf("   Full cycle (updates + eval): %.3f µs\n", full_us);
    
    printf("\n6. Breakdown of Setup Time\n");
    double context_creation = 3.6;  // From previous measurements
    double builder_creation = 0.05; // Estimate
    double first_eval = eval_us;
    double other = setup_us - context_creation - builder_creation - parse_us - first_eval;
    
    printf("   Context creation: %.3f µs (%.1f%%)\n", context_creation, (context_creation / setup_us) * 100);
    printf("   Builder creation: %.3f µs (%.1f%%)\n", builder_creation, (builder_creation / setup_us) * 100);
    printf("   Expression parsing: %.3f µs (%.1f%%)\n", parse_us, (parse_us / setup_us) * 100);
    printf("   First evaluation: %.3f µs (%.1f%%)\n", first_eval, (first_eval / setup_us) * 100);
    printf("   Other overhead: %.3f µs (%.1f%%)\n", other, (other / setup_us) * 100);
    
    // Cleanup
    exp_rs_batch_builder_free(eval_builder);
    exp_rs_context_free(ctx);
    
    return 0;
}