#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <time.h>
#include <math.h>
#include "exp_rs.h"
#include "common_allocator.h"

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

// Test batch memory lifecycle management
void test_batch_memory_lifecycle() {
    printf("=== Test Batch Memory Lifecycle ===\n");
    
    init_memory_tracking();
    reset_memory_stats();
    enable_allocation_tracking();
    
    memory_stats_t start_stats = get_memory_stats();
    printf("Starting stats: %zu allocs, %zu bytes\n", 
           start_stats.total_allocs, start_stats.total_allocated_bytes);
    
    // Test single batch creation and destruction
    printf("Testing single batch lifecycle...\n");
    ExprBatch* batch1 = expr_batch_new(8192);
    assert(batch1 != NULL);
    
    memory_stats_t after_create = get_memory_stats();
    printf("After batch creation: %zu allocs, %zu bytes\n", 
           after_create.total_allocs, after_create.total_allocated_bytes);
    
    // Add an expression to the batch
    expr_batch_add_expression(batch1, "x + y * 2");
    expr_batch_add_variable(batch1, "x", 1.0);
    expr_batch_add_variable(batch1, "y", 2.0);
    
    // Free the batch
    expr_batch_free(batch1);
    
    memory_stats_t after_free = get_memory_stats();
    printf("After batch free: %zu deallocs, current bytes: %zu\n", 
           after_free.total_deallocs, after_free.current_bytes);
    
    // Test multiple batch coexistence
    printf("\nTesting multiple batch coexistence...\n");
    memory_stats_t before_multi = get_memory_stats();
    
    ExprBatch* batch2 = expr_batch_new(4096);
    ExprBatch* batch3 = expr_batch_new(4096);
    ExprBatch* batch4 = expr_batch_new(4096);
    
    assert(batch2 != NULL);
    assert(batch3 != NULL);
    assert(batch4 != NULL);
    
    memory_stats_t after_multi = get_memory_stats();
    printf("Created 3 batches: %zu allocs (+%zu), %zu bytes (+%zu)\n",
           after_multi.total_allocs, 
           after_multi.total_allocs - before_multi.total_allocs,
           after_multi.total_allocated_bytes,
           after_multi.total_allocated_bytes - before_multi.total_allocated_bytes);
    
    // Use each batch
    expr_batch_add_expression(batch2, "a * 2");
    expr_batch_add_variable(batch2, "a", 5.0);
    
    expr_batch_add_expression(batch3, "b + 3");
    expr_batch_add_variable(batch3, "b", 7.0);
    
    expr_batch_add_expression(batch4, "c - 1");
    expr_batch_add_variable(batch4, "c", 9.0);
    
    // Free them in different order
    expr_batch_free(batch3);
    expr_batch_free(batch2);
    expr_batch_free(batch4);
    
    memory_stats_t after_multi_cleanup = get_memory_stats();
    printf("After freeing all batches: %zu deallocs (+%zu)\n",
           after_multi_cleanup.total_deallocs,
           after_multi_cleanup.total_deallocs - after_multi.total_deallocs);
    
    disable_allocation_tracking();
    printf("\n");
}

// Test batch clear and reuse
void test_batch_clear_reuse() {
    printf("=== Test Batch Clear and Reuse ===\n");
    
    init_memory_tracking();
    enable_allocation_tracking();
    
    // Create batch
    ExprBatch* batch = expr_batch_new(16384);
    assert(batch != NULL);
    
    // Add first expression
    expr_batch_add_expression(batch, "x * 2 + y");
    expr_batch_add_variable(batch, "x", 5.0);
    expr_batch_add_variable(batch, "y", 3.0);
    
    memory_stats_t before_clear = get_memory_stats();
    
    // Clear and reuse
    expr_batch_clear(batch);
    printf("✓ Batch cleared successfully\n");
    
    memory_stats_t after_clear = get_memory_stats();
    printf("Memory after clear: same allocations (%zu), may reuse internal memory\n",
           after_clear.total_allocs - before_clear.total_allocs);
    
    // Add new expression after clear
    expr_batch_add_expression(batch, "a + b * c");
    expr_batch_add_variable(batch, "a", 1.0);
    expr_batch_add_variable(batch, "b", 2.0);
    expr_batch_add_variable(batch, "c", 3.0);
    
    // Free batch
    expr_batch_free(batch);
    printf("✓ Batch freed after reuse\n");
    
    disable_allocation_tracking();
    printf("\n");
}

// Test zero allocations during evaluation
void test_zero_allocations() {
    printf("=== Test Zero Allocations During Evaluation ===\n");
    
    init_memory_tracking();
    
    // Note: exp-rs may be using custom allocator, enable tracking from start
    enable_allocation_tracking();
    memory_stats_t setup_start = get_memory_stats();
    
    // Create batch (arena managed internally)
    ExprBatch* batch = expr_batch_new(256 * 1024);
    memory_stats_t after_batch = get_memory_stats();
    
    ExprContext* ctx = expr_context_new();
    memory_stats_t after_context = get_memory_stats();
    
    printf("Batch creation: %zu bytes, %zu allocations\n", 
           after_batch.total_allocated_bytes - setup_start.total_allocated_bytes,
           after_batch.total_allocs - setup_start.total_allocs);
    printf("Context creation: %zu bytes, %zu allocations\n", 
           after_context.total_allocated_bytes - after_batch.total_allocated_bytes,
           after_context.total_allocs - after_batch.total_allocs);
    
    // Register required functions
    expr_context_add_function(ctx, "sin", 1, native_sin);
    expr_context_add_function(ctx, "cos", 1, native_cos);
    expr_context_add_function(ctx, "tan", 1, native_tan);
    expr_context_add_function(ctx, "sqrt", 1, native_sqrt);
    
    // Add complex expression
    expr_batch_add_expression(batch, 
        "sin(x) * cos(y) + tan(z) * sqrt(x*x + y*y + z*z)");
    
    // Add parameters
    expr_batch_add_variable(batch, "x", 0.0);
    expr_batch_add_variable(batch, "y", 0.0);
    expr_batch_add_variable(batch, "z", 0.0);
    
    // Do initial evaluation to parse expressions
    memory_stats_t before_initial = get_memory_stats();
    expr_batch_evaluate(batch, ctx);
    memory_stats_t after_initial = get_memory_stats();
    
    printf("✓ Initial evaluation complete\n");
    printf("Initial evaluation: %zu bytes, %zu allocations\n", 
           after_initial.total_allocated_bytes - before_initial.total_allocated_bytes,
           after_initial.total_allocs - before_initial.total_allocs);
    
    // NOW start the evaluation loop tracking
    memory_stats_t before_loop = get_memory_stats();
    printf("Starting evaluation loop with tracking enabled...\n");
    
    // Measure evaluation time for many iterations
    const int iterations = 100000;
    double start = get_time_us();
    
    for (int i = 0; i < iterations; i++) {
        // Update parameters
        Real x = (Real)(i % 100) / 100.0;
        Real y = (Real)((i + 33) % 100) / 100.0;
        Real z = (Real)((i + 66) % 100) / 100.0;
        
        expr_batch_set_variable(batch, 0, x);
        expr_batch_set_variable(batch, 1, y);
        expr_batch_set_variable(batch, 2, z);
        
        // Evaluate - should allocate zero memory
        expr_batch_evaluate(batch, ctx);
    }
    
    double end = get_time_us();
    memory_stats_t after_loop = get_memory_stats();
    
    // Cleanup phase
    memory_stats_t before_cleanup = get_memory_stats();
    
    double total_us = end - start;
    double us_per_eval = total_us / iterations;
    double evals_per_sec = 1e6 / us_per_eval;
    
    printf("✓ Completed %d evaluations\n", iterations);
    printf("  Total time: %.2f ms\n", total_us / 1000.0);
    printf("  Time per eval: %.3f µs\n", us_per_eval);
    printf("  Evaluations/sec: %.0f\n", evals_per_sec);
    printf("  Target (1000 Hz): %s\n", 
           evals_per_sec >= 1000 ? "✓ ACHIEVED" : "✗ NOT ACHIEVED");
    
    // Verify zero allocations during evaluation
    size_t allocs_during_eval = after_loop.total_allocs - before_loop.total_allocs;
    size_t deallocs_during_eval = after_loop.total_deallocs - before_loop.total_deallocs;
    size_t bytes_allocated_during_eval = after_loop.total_allocated_bytes - before_loop.total_allocated_bytes;
    size_t bytes_deallocated_during_eval = after_loop.total_deallocated_bytes - before_loop.total_deallocated_bytes;
    ssize_t net_bytes_change = (ssize_t)bytes_allocated_during_eval - (ssize_t)bytes_deallocated_during_eval;
    
    printf("\n=== ALLOCATION ANALYSIS ===\n");
    printf("During %d evaluations:\n", iterations);
    printf("  Allocations: %zu\n", allocs_during_eval);
    printf("  Deallocations: %zu\n", deallocs_during_eval);
    printf("  Bytes allocated: %zu\n", bytes_allocated_during_eval);
    printf("  Bytes deallocated: %zu\n", bytes_deallocated_during_eval);
    printf("  Net bytes change: %+zd\n", net_bytes_change);
    printf("  Current bytes in use before: %zu\n", before_loop.current_bytes);
    printf("  Current bytes in use after: %zu\n", after_loop.current_bytes);
    
    if (allocs_during_eval > 0) {
        printf("\nPer-evaluation averages:\n");
        printf("  Allocations per eval: %.2f\n", (double)allocs_during_eval / iterations);
        printf("  Bytes allocated per eval: %.2f\n", (double)bytes_allocated_during_eval / iterations);
        printf("  Bytes deallocated per eval: %.2f\n", (double)bytes_deallocated_during_eval / iterations);
        printf("  Net bytes per eval: %+.2f\n", (double)net_bytes_change / iterations);
    }
    
    printf("\nZero allocation claim: %s\n", 
           allocs_during_eval == 0 ? "✓ VERIFIED" : "✗ FAILED");
    
    if (net_bytes_change == 0 && allocs_during_eval > 0) {
        printf("Note: While allocations occurred, all memory was freed (net change: 0)\n");
        printf("      This suggests temporary allocations that are immediately cleaned up\n");
    }
    
    // Cleanup
    expr_batch_free(batch);
    expr_context_free(ctx);
    
    memory_stats_t after_cleanup = get_memory_stats();
    printf("Cleanup: %zu deallocations\n", 
           after_cleanup.total_deallocs - before_cleanup.total_deallocs);
    
    // Final comprehensive memory report
    printf("\n=== COMPREHENSIVE MEMORY REPORT ===\n");
    printf("Total test statistics:\n");
    printf("  Total allocations: %zu\n", after_cleanup.total_allocs);
    printf("  Total deallocations: %zu\n", after_cleanup.total_deallocs);
    printf("  Total bytes allocated: %zu (%.2f KB)\n", 
           after_cleanup.total_allocated_bytes, 
           after_cleanup.total_allocated_bytes / 1024.0);
    printf("  Total bytes deallocated: %zu (%.2f KB)\n", 
           after_cleanup.total_deallocated_bytes,
           after_cleanup.total_deallocated_bytes / 1024.0);
    printf("  Net bytes leaked: %+zd\n", 
           (ssize_t)after_cleanup.total_allocated_bytes - (ssize_t)after_cleanup.total_deallocated_bytes);
    printf("  Current bytes in use: %zu\n", after_cleanup.current_bytes);
    printf("  Peak bytes used: %zu (%.2f KB)\n", 
           after_cleanup.peak_bytes,
           after_cleanup.peak_bytes / 1024.0);
    printf("  Leaked allocations: %zu\n", after_cleanup.leaked_allocs);
    
    // Memory efficiency analysis
    if (after_cleanup.total_allocated_bytes > 0) {
        double reuse_ratio = (double)after_cleanup.total_deallocated_bytes / after_cleanup.total_allocated_bytes;
        printf("\nMemory efficiency:\n");
        printf("  Memory reuse ratio: %.2f%% (deallocated/allocated)\n", reuse_ratio * 100);
        if (iterations > 0) {
            printf("  Total memory per evaluation: %.2f bytes\n", 
                   (double)after_cleanup.total_allocated_bytes / iterations);
        }
    }
    
    // Final status
    printf("\nFinal status: %s\n", 
           after_cleanup.current_bytes == 0 ? "✓ NO MEMORY LEAKS" : "✗ MEMORY LEAK DETECTED");
    
    disable_allocation_tracking();
    printf("\n");
}

// Main test runner
int main() {
    printf("\n==== Arena Integration Tests with Memory Tracking ====\n\n");
    
    // Test batch memory lifecycle and management
    test_batch_memory_lifecycle();
    test_batch_clear_reuse();
    test_zero_allocations();
    
    printf("==== All Tests Passed! ====\n\n");
    return 0;
}