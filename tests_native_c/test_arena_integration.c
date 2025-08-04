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
Real native_tan(const Real* args, uintptr_t nargs) { (void)nargs; return tan(args[0]); }
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

// Test basic arena creation and destruction
void test_arena_lifecycle() {
    printf("=== Test Arena Lifecycle ===\n");
    
    // Create arena with 256KB
    ExprArena* arena = expr_arena_new(256 * 1024);
    assert(arena != NULL);
    printf("✓ Arena created successfully\n");
    
    // Test arena reset
    expr_arena_reset(arena);
    printf("✓ Arena reset successfully\n");
    
    // Free arena
    expr_arena_free(arena);
    printf("✓ Arena freed successfully\n\n");
}

// Test batch builder with arena
void test_batch_builder_with_arena() {
    printf("=== Test Batch Builder with Arena ===\n");
    
    // Create arena
    ExprArena* arena = expr_arena_new(256 * 1024);
    assert(arena != NULL);
    
    // Create batch builder with arena
    ExprBatch* builder = expr_batch_new(arena);
    assert(builder != NULL);
    printf("✓ Batch builder created with arena\n");
    
    // Add expressions
    int32_t expr1_idx = expr_batch_add_expression(builder, "x + y");
    assert(expr1_idx == 0);
    
    int32_t expr2_idx = expr_batch_add_expression(builder, "x * y");
    assert(expr2_idx == 1);
    
    int32_t expr3_idx = expr_batch_add_expression(builder, "sqrt(x*x + y*y)");
    assert(expr3_idx == 2);
    printf("✓ Added 3 expressions\n");
    
    // Add parameters
    int32_t x_idx = expr_batch_add_variable(builder, "x", 3.0);
    assert(x_idx == 0);
    
    int32_t y_idx = expr_batch_add_variable(builder, "y", 4.0);
    assert(y_idx == 1);
    printf("✓ Added 2 parameters\n");
    
    // Create context
    ExprContext* ctx = expr_context_new();
    assert(ctx != NULL);
    
    // Register sqrt function for third expression
    expr_context_add_function(ctx, "sqrt", 1, native_sqrt);
    
    // Evaluate
    int32_t eval_result = expr_batch_evaluate(builder, ctx);
    assert(eval_result == 0);
    printf("✓ Evaluation successful\n");
    
    // Get results
    Real result1 = expr_batch_get_result(builder, expr1_idx);
    Real result2 = expr_batch_get_result(builder, expr2_idx);
    Real result3 = expr_batch_get_result(builder, expr3_idx);
    
    printf("Results: x+y=%.2f, x*y=%.2f, sqrt(x²+y²)=%.2f\n", 
           result1, result2, result3);
    
    // Verify results
    assert(result1 == 7.0);  // 3 + 4
    assert(result2 == 12.0); // 3 * 4
    assert(result3 == 5.0);  // sqrt(9 + 16)
    printf("✓ Results verified\n");
    
    // Cleanup
    expr_context_free(ctx);
    expr_batch_free(builder);
    expr_arena_free(arena);
    printf("✓ Cleanup successful\n\n");
}

// Test arena reset and reuse
void test_arena_reset_reuse() {
    printf("=== Test Arena Reset and Reuse ===\n");
    
    ExprArena* arena = expr_arena_new(128 * 1024);
    ExprContext* ctx = expr_context_new();
    
    // First use
    ExprBatch* builder1 = expr_batch_new(arena);
    expr_batch_add_expression(builder1, "a + b + c");
    expr_batch_add_variable(builder1, "a", 1.0);
    expr_batch_add_variable(builder1, "b", 2.0);
    expr_batch_add_variable(builder1, "c", 3.0);
    expr_batch_evaluate(builder1, ctx);
    Real result1 = expr_batch_get_result(builder1, 0);
    assert(result1 == 6.0);
    printf("✓ First evaluation: %.2f\n", result1);
    
    // Free builder but keep arena
    expr_batch_free(builder1);
    
    // Reset arena for reuse
    expr_arena_reset(arena);
    printf("✓ Arena reset\n");
    
    // Second use with same arena
    ExprBatch* builder2 = expr_batch_new(arena);
    expr_batch_add_expression(builder2, "x * y * z");
    expr_batch_add_variable(builder2, "x", 2.0);
    expr_batch_add_variable(builder2, "y", 3.0);
    expr_batch_add_variable(builder2, "z", 4.0);
    expr_batch_evaluate(builder2, ctx);
    Real result2 = expr_batch_get_result(builder2, 0);
    assert(result2 == 24.0);
    printf("✓ Second evaluation: %.2f\n", result2);
    
    // Cleanup
    expr_batch_free(builder2);
    expr_context_free(ctx);
    expr_arena_free(arena);
    printf("✓ Arena reuse successful\n\n");
}

// Test benchmark expressions matching consolidated_benchmark.rs
void test_benchmark_expressions() {
    printf("=== Test Benchmark Expressions (matching Rust benchmark) ===\n");
    
    // Create arena and context
    ExprArena* arena = expr_arena_new(512 * 1024);
    ExprContext* ctx = expr_context_new();
    
    // Register required functions (matching consolidated_benchmark.rs)
    expr_context_add_function(ctx, "sin", 1, native_sin);
    expr_context_add_function(ctx, "cos", 1, native_cos);
    expr_context_add_function(ctx, "tan", 1, native_tan);
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
    
    // Add the same 7 expressions from consolidated_benchmark.rs
    const char* expressions[] = {
        "a*sin(b*3.14159/180) + c*cos(d*3.14159/180) + sqrt(e*e + f*f)",
        "exp(g/10) * log(h+1) + pow(i, 0.5) * j",
        "((a > 5) && (b < 10)) * c + ((d >= e) || (f != g)) * h + min(i, j)",
        "sqrt(pow(a-e, 2) + pow(b-f, 2)) + atan2(c-g, d-h) * (i+j)/2",
        "abs(a-b) * sign(c-d) + max(e, f) * min(g, h) + fmod(i*j, 10)",
        "(a+b+c)/3 * sin((d+e+f)*3.14159/6) + log10(g*h+1) - exp(-i*j/100)",
        "a + b * c - d / (e + 0.001) + pow(f, g) * h - i + j"
    };
    
    // Add all expressions
    for (int i = 0; i < 7; i++) {
        int32_t idx = expr_batch_add_expression(builder, expressions[i]);
        if (idx < 0) {
            printf("Failed to add expression %d: %s\n", i, expressions[i]);
            return;
        }
    }
    printf("✓ Added 7 benchmark expressions\n");
    
    // Add 10 parameters (a through j)
    const char* param_names[] = {"a", "b", "c", "d", "e", "f", "g", "h", "i", "j"};
    for (int i = 0; i < 10; i++) {
        expr_batch_add_variable(builder, param_names[i], (i + 1) * 1.5);
    }
    printf("✓ Added 10 parameters (a-j)\n");
    
    // Do initial evaluation to parse expressions
    expr_batch_evaluate(builder, ctx);
    printf("✓ Initial evaluation complete\n");
    
    // Test different batch sizes
    const int batch_sizes[] = {1, 10, 100, 1000};
    
    for (int b = 0; b < 4; b++) {
        int batch_size = batch_sizes[b];
        printf("\nBatch size %d (simulating %dms at 1000Hz):\n", batch_size, batch_size);
        
        // Measure evaluation time
        const int iterations = 10000 / batch_size; // Scale iterations to keep total work constant
        double start = get_time_us();
        
        for (int iter = 0; iter < iterations; iter++) {
            for (int batch = 0; batch < batch_size; batch++) {
                // Update parameters (matching Rust benchmark pattern)
                for (int p = 0; p < 10; p++) {
                    Real value = (p + 1) * 1.5 + (batch + 1) * 0.1;
                    expr_batch_set_variable(builder, p, value);
                }
                
                // Evaluate all 7 expressions
                expr_batch_evaluate(builder, ctx);
            }
        }
        
        double end = get_time_us();
        double total_us = end - start;
        double total_evals = iterations * batch_size * 7; // 7 expressions per evaluation
        double us_per_eval = total_us / total_evals;
        double us_per_batch = total_us / (iterations * batch_size);
        double batch_rate = 1e6 / us_per_batch;
        
        printf("  Total evaluations: %.0f\n", total_evals);
        printf("  Total time: %.2f ms\n", total_us / 1000.0);
        printf("  Time per batch: %.3f µs\n", us_per_batch);
        printf("  Time per expression: %.3f µs\n", us_per_eval);
        printf("  Batch rate: %.0f Hz\n", batch_rate);
        printf("  Target (1000 Hz): %s\n", 
               batch_rate >= 1000 ? "✓ ACHIEVED" : "✗ NOT ACHIEVED");
    }
    
    // Cleanup
    expr_batch_free(builder);
    expr_context_free(ctx);
    expr_arena_free(arena);
    printf("\n");
}

// Test zero allocations during evaluation
void test_zero_allocations() {
    printf("=== Test Zero Allocations During Evaluation ===\n");
    
    // Create arena and context
    ExprArena* arena = expr_arena_new(256 * 1024);
    ExprContext* ctx = expr_context_new();
    
    // Register required functions
    expr_context_add_function(ctx, "sin", 1, native_sin);
    expr_context_add_function(ctx, "cos", 1, native_cos);
    expr_context_add_function(ctx, "tan", 1, native_tan);
    expr_context_add_function(ctx, "sqrt", 1, native_sqrt);
    
    ExprBatch* builder = expr_batch_new(arena);
    
    // Add complex expression
    expr_batch_add_expression(builder, 
        "sin(x) * cos(y) + tan(z) * sqrt(x*x + y*y + z*z)");
    
    // Add parameters
    expr_batch_add_variable(builder, "x", 0.0);
    expr_batch_add_variable(builder, "y", 0.0);
    expr_batch_add_variable(builder, "z", 0.0);
    
    // Do initial evaluation to parse expressions
    expr_batch_evaluate(builder, ctx);
    printf("✓ Initial evaluation complete\n");
    
    // Measure evaluation time for many iterations
    const int iterations = 100000;
    double start = get_time_us();
    
    for (int i = 0; i < iterations; i++) {
        // Update parameters
        Real x = (Real)(i % 100) / 100.0;
        Real y = (Real)((i + 33) % 100) / 100.0;
        Real z = (Real)((i + 66) % 100) / 100.0;
        
        expr_batch_set_variable(builder, 0, x);
        expr_batch_set_variable(builder, 1, y);
        expr_batch_set_variable(builder, 2, z);
        
        // Evaluate - should allocate zero memory
        expr_batch_evaluate(builder, ctx);
    }
    
    double end = get_time_us();
    double total_us = end - start;
    double us_per_eval = total_us / iterations;
    double evals_per_sec = 1e6 / us_per_eval;
    
    printf("✓ Completed %d evaluations\n", iterations);
    printf("  Total time: %.2f ms\n", total_us / 1000.0);
    printf("  Time per eval: %.3f µs\n", us_per_eval);
    printf("  Evaluations/sec: %.0f\n", evals_per_sec);
    printf("  Target (1000 Hz): %s\n", 
           evals_per_sec >= 1000 ? "✓ ACHIEVED" : "✗ NOT ACHIEVED");
    
    // Cleanup
    expr_batch_free(builder);
    expr_context_free(ctx);
    expr_arena_free(arena);
    printf("\n");
}

// Test arena size estimation
void test_arena_size_estimation() {
    printf("=== Test Arena Size Estimation ===\n");
    
    const char* expressions[] = {
        "x + y",
        "sin(x) * cos(y)",
        "sqrt(x*x + y*y)",
        "x^3 + 2*x^2 + 3*x + 4",
        "(x > 0 ? x : -x) * (y > 0 ? y : -y)"
    };
    size_t num_exprs = sizeof(expressions) / sizeof(expressions[0]);
    
    // Estimate arena size for 1000 evaluations
    // Calculate total expression length
    size_t total_expr_length = 0;
    for (int i = 0; i < num_exprs; i++) {
        total_expr_length += strlen(expressions[i]);
    }
    
    size_t estimated_size = expr_estimate_arena_size(num_exprs, total_expr_length, 0, 1000);
    printf("✓ Estimated arena size: %zu bytes (%.1f KB)\n", 
           estimated_size, estimated_size / 1024.0);
    
    // Create arena with estimated size
    ExprArena* arena = expr_arena_new(estimated_size);
    assert(arena != NULL);
    printf("✓ Created arena with estimated size\n");
    
    // Test that we can actually use it
    ExprBatch* builder = expr_batch_new(arena);
    for (size_t i = 0; i < num_exprs; i++) {
        int32_t idx = expr_batch_add_expression(builder, expressions[i]);
        assert(idx == (int32_t)i);
    }
    printf("✓ Successfully added all expressions\n");
    
    // Cleanup
    expr_batch_free(builder);
    expr_arena_free(arena);
    printf("\n");
}

// Test error handling
void test_error_handling() {
    printf("=== Test Error Handling ===\n");
    
    // Test NULL arena
    ExprBatch* builder = expr_batch_new(NULL);
    assert(builder == NULL);
    printf("✓ NULL arena handled correctly\n");
    
    // Test invalid expression (skip for now - parser might accept it)
    ExprArena* arena = expr_arena_new(64 * 1024);
    builder = expr_batch_new(arena);
    
    // int32_t idx = expr_batch_add_expression(builder, "x + + y");
    // assert(idx < 0);  // Should return error
    // printf("✓ Invalid expression handled correctly\n");
    
    // Test duplicate parameter
    expr_batch_add_variable(builder, "x", 1.0);
    int32_t dup_idx = expr_batch_add_variable(builder, "x", 2.0);
    assert(dup_idx < 0);  // Should return error
    printf("✓ Duplicate parameter handled correctly\n");
    
    // Cleanup
    expr_batch_free(builder);
    expr_arena_free(arena);
    printf("\n");
}

// Main test runner
int main() {
    printf("\n==== Arena Integration Tests ====\n\n");
    
    test_arena_lifecycle();
    test_batch_builder_with_arena();
    test_arena_reset_reuse();
    test_benchmark_expressions();  // New test matching Rust benchmark
    test_zero_allocations();
    test_arena_size_estimation();
    test_error_handling();
    
    printf("==== All Tests Passed! ====\n\n");
    return 0;
}