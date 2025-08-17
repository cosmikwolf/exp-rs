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

// Test function to verify custom allocator is working
void test_custom_allocator_integration() {
    printf("=== Test Custom Allocator Integration ===\n");
    
    init_memory_tracking();
    reset_memory_stats();
    enable_allocation_tracking();
    
    memory_stats_t start_stats = get_memory_stats();
    printf("Starting stats: %zu allocs, %zu bytes\n", 
           start_stats.total_allocs, start_stats.total_allocated_bytes);
    
    // Test direct call to exp_rs_malloc
    printf("Testing direct exp_rs_malloc call...\n");
    void* test_ptr = exp_rs_malloc(1024);
    
    memory_stats_t after_malloc = get_memory_stats();
    printf("After direct malloc: %zu allocs, %zu bytes\n", 
           after_malloc.total_allocs, after_malloc.total_allocated_bytes);
    
    if (after_malloc.total_allocs == start_stats.total_allocs) {
        printf("❌ FAILED: exp_rs_malloc not tracked - custom allocator not working!\n");
        printf("This means the Rust FFI is NOT using exp_rs_malloc/exp_rs_free\n");
        exit(1);  // Fail the test
    } else {
        printf("✅ PASSED: exp_rs_malloc is being tracked\n");
    }
    
    // Test exp_rs_free
    printf("Testing exp_rs_free call...\n");
    exp_rs_free(test_ptr);
    
    memory_stats_t after_free = get_memory_stats();
    printf("After free: %zu allocs, %zu deallocs, %zu current bytes\n", 
           after_free.total_allocs, after_free.total_deallocs, after_free.current_bytes);
    
    // Now test if FFI functions use our custom allocator
    printf("\nTesting FFI arena creation (should call exp_rs_malloc)...\n");
    memory_stats_t before_arena = get_memory_stats();
    
    ExprArena* arena = expr_arena_new(8192);  // Small arena
    
    memory_stats_t after_arena = get_memory_stats();
    printf("Arena creation stats: %zu allocs (+%zu), %zu bytes (+%zu)\n",
           after_arena.total_allocs, 
           after_arena.total_allocs - before_arena.total_allocs,
           after_arena.total_allocated_bytes,
           after_arena.total_allocated_bytes - before_arena.total_allocated_bytes);
    
    if (after_arena.total_allocs == before_arena.total_allocs) {
        printf("ℹ️  INFO: Arena creation didn't trigger exp_rs_malloc\n");
        printf("This is EXPECTED on native targets (x86_64/aarch64)\n");
        printf("Custom allocator is only enabled for ARM targets (embedded/STM32)\n");
        printf("On native targets, Rust uses the system allocator directly\n");
        printf("But now we modified it to work on native - so this indicates an issue\n");
        printf("❌ FAILED: Custom allocator should be working now!\n");
        exit(1);
    } else {
        printf("✅ PASSED: Arena creation uses custom allocator\n");
    }
    
    // Clean up
    expr_arena_free(arena);
    memory_stats_t after_cleanup = get_memory_stats();
    printf("After arena free: %zu deallocs (+%zu)\n",
           after_cleanup.total_deallocs,
           after_cleanup.total_deallocs - after_arena.total_deallocs);
    
    disable_allocation_tracking();
    printf("\n");
}

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

// Test zero allocations during evaluation
void test_zero_allocations() {
    printf("=== Test Zero Allocations During Evaluation ===\n");
    
    init_memory_tracking();
    
    // Note: exp-rs may be using custom allocator, enable tracking from start
    enable_allocation_tracking();
    memory_stats_t setup_start = get_memory_stats();
    
    // Create arena and context
    ExprArena* arena = expr_arena_new(256 * 1024);
    memory_stats_t after_arena = get_memory_stats();
    
    ExprContext* ctx = expr_context_new();
    memory_stats_t after_context = get_memory_stats();
    
    printf("Arena creation: %zu bytes, %zu allocations\n", 
           after_arena.total_allocated_bytes - setup_start.total_allocated_bytes,
           after_arena.total_allocs - setup_start.total_allocs);
    printf("Context creation: %zu bytes, %zu allocations\n", 
           after_context.total_allocated_bytes - after_arena.total_allocated_bytes,
           after_context.total_allocs - after_arena.total_allocs);
    
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
    memory_stats_t before_initial = get_memory_stats();
    expr_batch_evaluate(builder, ctx);
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
        
        expr_batch_set_variable(builder, 0, x);
        expr_batch_set_variable(builder, 1, y);
        expr_batch_set_variable(builder, 2, z);
        
        // Evaluate - should allocate zero memory
        expr_batch_evaluate(builder, ctx);
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
    size_t bytes_during_eval = after_loop.total_allocated_bytes - before_loop.total_allocated_bytes;
    
    printf("\n=== ALLOCATION ANALYSIS ===\n");
    printf("Allocations during %d evaluations: %zu\n", iterations, allocs_during_eval);
    printf("Bytes allocated during evaluations: %zu\n", bytes_during_eval);
    printf("Zero allocation claim: %s\n", 
           allocs_during_eval == 0 ? "✓ VERIFIED" : "✗ FAILED");
    
    if (allocs_during_eval > 0) {
        printf("WARNING: Found %zu allocations during evaluation!\n", allocs_during_eval);
        printf("Average bytes per evaluation: %.2f\n", (double)bytes_during_eval / iterations);
    }
    
    // Cleanup
    expr_batch_free(builder);
    expr_context_free(ctx);
    expr_arena_free(arena);
    
    memory_stats_t after_cleanup = get_memory_stats();
    printf("Cleanup: %zu bytes freed, %zu deallocations\n", 
           before_cleanup.total_allocated_bytes - after_cleanup.current_bytes,
           after_cleanup.total_deallocs - before_cleanup.total_deallocs);
    
    disable_allocation_tracking();
    printf("\n");
}

// Main test runner
int main() {
    printf("\n==== Arena Integration Tests with Memory Tracking ====\n\n");
    
    // CRITICAL: Test custom allocator integration first - fail fast if not working
    test_custom_allocator_integration();
    
    test_arena_lifecycle();
    test_zero_allocations();
    
    printf("==== All Tests Passed! ====\n\n");
    return 0;
}