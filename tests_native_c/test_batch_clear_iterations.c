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
Real native_sqrt(const Real* args, uintptr_t nargs) { (void)nargs; return sqrt(args[0]); }
Real native_exp(const Real* args, uintptr_t nargs) { (void)nargs; return exp(args[0]); }
Real native_log(const Real* args, uintptr_t nargs) { (void)nargs; return log(args[0]); }

// Function to run a single iteration of batch operations
void run_batch_iteration(ExprBatch* batch, ExprContext* ctx, int iteration) {
    printf("\n--- Iteration %d ---\n", iteration);
    
    memory_stats_t before_iter = get_memory_stats();
    
    // Add different expressions based on iteration to test variety
    char expr_buffer[256];
    switch (iteration % 5) {
        case 0:
            snprintf(expr_buffer, sizeof(expr_buffer), 
                     "sin(x * %d) + cos(y * %d)", iteration, iteration);
            break;
        case 1:
            snprintf(expr_buffer, sizeof(expr_buffer), 
                     "sqrt(x * x + y * y) * %d", iteration);
            break;
        case 2:
            snprintf(expr_buffer, sizeof(expr_buffer), 
                     "exp(x / %d) * log(y + %d)", iteration + 1, iteration);
            break;
        case 3:
            snprintf(expr_buffer, sizeof(expr_buffer), 
                     "sin(x) * sin(x) + cos(y) * cos(y) + %d", iteration);
            break;
        case 4:
            snprintf(expr_buffer, sizeof(expr_buffer), 
                     "(x + %d) * (y - %d) / sqrt(x * x + y * y + 0.01)", 
                     iteration, iteration);
            break;
    }
    
    // Add expression and variables
    expr_batch_add_expression(batch, expr_buffer);
    expr_batch_add_variable(batch, "x", 1.0 + iteration * 0.1);
    expr_batch_add_variable(batch, "y", 2.0 + iteration * 0.2);
    
    memory_stats_t after_add = get_memory_stats();
    
    // Evaluate the expression multiple times
    const int eval_count = 1000;
    double start_time = get_time_us();
    
    for (int i = 0; i < eval_count; i++) {
        // Update variables
        expr_batch_set_variable(batch, 0, 1.0 + i * 0.001);
        expr_batch_set_variable(batch, 1, 2.0 + i * 0.002);
        
        // Evaluate
        expr_batch_evaluate(batch, ctx);
    }
    
    double end_time = get_time_us();
    memory_stats_t after_eval = get_memory_stats();
    
    // Clear the batch for next iteration
    expr_batch_clear(batch);
    memory_stats_t after_clear = get_memory_stats();
    
    // Report memory changes for this iteration
    printf("Memory changes in iteration %d:\n", iteration);
    printf("  After adding expression:\n");
    printf("    Allocations: +%zu\n", after_add.total_allocs - before_iter.total_allocs);
    printf("    Bytes allocated: +%zu\n", after_add.total_allocated_bytes - before_iter.total_allocated_bytes);
    
    printf("  After %d evaluations:\n", eval_count);
    printf("    Allocations: +%zu\n", after_eval.total_allocs - after_add.total_allocs);
    printf("    Bytes allocated: +%zu\n", after_eval.total_allocated_bytes - after_add.total_allocated_bytes);
    printf("    Bytes deallocated: +%zu\n", after_eval.total_deallocated_bytes - after_add.total_deallocated_bytes);
    printf("    Time: %.2f ms (%.3f µs/eval)\n", 
           (end_time - start_time) / 1000.0, (end_time - start_time) / eval_count);
    
    printf("  After clear:\n");
    printf("    Deallocations: +%zu\n", after_clear.total_deallocs - after_eval.total_deallocs);
    printf("    Bytes deallocated: +%zu\n", after_clear.total_deallocated_bytes - after_eval.total_deallocated_bytes);
    printf("    Current bytes in use: %zu\n", after_clear.current_bytes);
}

// Main test function
void test_batch_clear_with_iterations() {
    printf("=== Test Batch Clear Across Multiple Iterations ===\n");
    
    init_memory_tracking();
    reset_memory_stats();
    enable_allocation_tracking();
    
    memory_stats_t initial_stats = get_memory_stats();
    printf("Initial state: %zu bytes in use\n", initial_stats.current_bytes);
    
    // Create batch and context once
    ExprBatch* batch = expr_batch_new(64 * 1024);  // 64KB arena
    assert(batch != NULL);
    
    ExprContext* ctx = expr_context_new();
    assert(ctx != NULL);
    
    // Register functions
    expr_context_add_function(ctx, "sin", 1, native_sin);
    expr_context_add_function(ctx, "cos", 1, native_cos);
    expr_context_add_function(ctx, "sqrt", 1, native_sqrt);
    expr_context_add_function(ctx, "exp", 1, native_exp);
    expr_context_add_function(ctx, "log", 1, native_log);
    
    memory_stats_t after_setup = get_memory_stats();
    printf("After setup: %zu bytes allocated, %zu bytes in use\n", 
           after_setup.total_allocated_bytes, after_setup.current_bytes);
    
    // Track memory stats across iterations
    size_t peak_memory_per_iter[15];
    size_t bytes_allocated_per_iter[15];
    size_t bytes_deallocated_per_iter[15];
    
    // Run multiple iterations with clear in between
    const int num_iterations = 15;
    for (int i = 0; i < num_iterations; i++) {
        memory_stats_t before = get_memory_stats();
        run_batch_iteration(batch, ctx, i + 1);
        memory_stats_t after = get_memory_stats();
        
        peak_memory_per_iter[i] = after.peak_bytes;
        bytes_allocated_per_iter[i] = after.total_allocated_bytes - before.total_allocated_bytes;
        bytes_deallocated_per_iter[i] = after.total_deallocated_bytes - before.total_deallocated_bytes;
    }
    
    // Analyze memory patterns across iterations
    printf("\n=== MEMORY PATTERN ANALYSIS ===\n");
    printf("Memory usage per iteration:\n");
    for (int i = 0; i < num_iterations; i++) {
        printf("  Iteration %2d: allocated=%6zu, deallocated=%6zu, net=%+6zd\n",
               i + 1, 
               bytes_allocated_per_iter[i],
               bytes_deallocated_per_iter[i],
               (ssize_t)bytes_allocated_per_iter[i] - (ssize_t)bytes_deallocated_per_iter[i]);
    }
    
    // Check for memory growth
    int memory_stable = 1;
    for (int i = 1; i < num_iterations; i++) {
        ssize_t net_change = (ssize_t)bytes_allocated_per_iter[i] - (ssize_t)bytes_deallocated_per_iter[i];
        if (net_change > 100) {  // Allow small variations
            memory_stable = 0;
            printf("\nWARNING: Memory growth detected in iteration %d: %+zd bytes\n", i + 1, net_change);
        }
    }
    
    if (memory_stable) {
        printf("\n✓ Memory usage is stable across iterations (batch clear is working correctly)\n");
    } else {
        printf("\n✗ Memory usage is growing (potential issue with batch clear)\n");
    }
    
    // Final memory state before cleanup
    memory_stats_t before_cleanup = get_memory_stats();
    printf("\n=== BEFORE CLEANUP ===\n");
    printf("Total allocations: %zu\n", before_cleanup.total_allocs);
    printf("Total deallocations: %zu\n", before_cleanup.total_deallocs);
    printf("Total bytes allocated: %zu (%.2f KB)\n", 
           before_cleanup.total_allocated_bytes, before_cleanup.total_allocated_bytes / 1024.0);
    printf("Total bytes deallocated: %zu (%.2f KB)\n", 
           before_cleanup.total_deallocated_bytes, before_cleanup.total_deallocated_bytes / 1024.0);
    printf("Current bytes in use: %zu\n", before_cleanup.current_bytes);
    printf("Peak bytes: %zu (%.2f KB)\n", before_cleanup.peak_bytes, before_cleanup.peak_bytes / 1024.0);
    
    // Cleanup
    expr_batch_free(batch);
    expr_context_free(ctx);
    
    memory_stats_t final_stats = get_memory_stats();
    printf("\n=== AFTER CLEANUP ===\n");
    printf("Final bytes in use: %zu\n", final_stats.current_bytes);
    printf("Final leaked allocations: %zu\n", final_stats.leaked_allocs);
    printf("Status: %s\n", 
           final_stats.current_bytes == 0 ? "✓ NO MEMORY LEAKS" : "✗ MEMORY LEAK DETECTED");
    
    disable_allocation_tracking();
}

int main() {
    printf("\n==== Batch Clear Iteration Test ====\n\n");
    
    test_batch_clear_with_iterations();
    
    printf("\n==== Test Complete ====\n\n");
    return 0;
}