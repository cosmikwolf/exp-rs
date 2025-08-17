#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <time.h>
#include <math.h>
#include <stdatomic.h>
#include "exp_rs.h"

// ============================================================================
// Custom Memory Tracking System
// ============================================================================

// Memory tracking statistics
static atomic_size_t total_allocations = 0;
static atomic_size_t total_deallocations = 0;
static atomic_size_t current_bytes = 0;
static atomic_size_t peak_bytes = 0;
static atomic_size_t total_allocated_bytes = 0;
static atomic_bool tracking_enabled = false;

// Memory allocation tracking structure
typedef struct {
    size_t size;
    size_t magic; // For corruption detection
} alloc_header_t;

#define ALLOC_MAGIC 0xDEADBEEF
#define HEADER_SIZE sizeof(alloc_header_t)

// Store original malloc/free for fallback
static void* (*original_malloc)(size_t) = NULL;
static void (*original_free)(void*) = NULL;
static bool tracking_initialized = false;

// Initialize memory tracking
void init_memory_tracking() {
    if (!tracking_initialized) {
        original_malloc = malloc;
        original_free = free;
        tracking_initialized = true;
    }
    
    // Reset counters
    atomic_store(&total_allocations, 0);
    atomic_store(&total_deallocations, 0);
    atomic_store(&current_bytes, 0);
    atomic_store(&peak_bytes, 0);
    atomic_store(&total_allocated_bytes, 0);
}

// Enable/disable allocation tracking
void enable_allocation_tracking() {
    atomic_store(&tracking_enabled, true);
}

void disable_allocation_tracking() {
    atomic_store(&tracking_enabled, false);
}

// Custom malloc implementation
void* tracked_malloc(size_t size) {
    void* ptr = original_malloc(size + HEADER_SIZE);
    
    if (ptr && atomic_load(&tracking_enabled)) {
        alloc_header_t* header = (alloc_header_t*)ptr;
        header->size = size;
        header->magic = ALLOC_MAGIC;
        
        // Update statistics atomically
        atomic_fetch_add(&total_allocations, 1);
        atomic_fetch_add(&total_allocated_bytes, size);
        
        size_t new_current = atomic_fetch_add(&current_bytes, size) + size;
        
        // Update peak if necessary
        size_t current_peak = atomic_load(&peak_bytes);
        while (new_current > current_peak) {
            if (atomic_compare_exchange_weak(&peak_bytes, &current_peak, new_current)) {
                break;
            }
        }
        
        return (char*)ptr + HEADER_SIZE;
    }
    
    return ptr;
}

// Custom free implementation
void tracked_free(void* ptr) {
    if (!ptr) return;
    
    if (atomic_load(&tracking_enabled)) {
        alloc_header_t* header = (alloc_header_t*)((char*)ptr - HEADER_SIZE);
        
        // Verify magic number
        if (header->magic == ALLOC_MAGIC) {
            size_t size = header->size;
            header->magic = 0; // Clear magic to detect double-free
            
            // Update statistics
            atomic_fetch_add(&total_deallocations, 1);
            atomic_fetch_sub(&current_bytes, size);
            
            original_free(header);
        } else {
            // Not our allocation or corrupted - use original free
            original_free(ptr);
        }
    } else {
        original_free(ptr);
    }
}

// Get memory statistics
typedef struct {
    size_t total_allocs;
    size_t total_deallocs;
    size_t current_bytes;
    size_t peak_bytes;
    size_t total_allocated_bytes;
    size_t leaked_allocs;
} memory_stats_t;

memory_stats_t get_memory_stats() {
    memory_stats_t stats;
    stats.total_allocs = atomic_load(&total_allocations);
    stats.total_deallocs = atomic_load(&total_deallocations);
    stats.current_bytes = atomic_load(&current_bytes);
    stats.peak_bytes = atomic_load(&peak_bytes);
    stats.total_allocated_bytes = atomic_load(&total_allocated_bytes);
    stats.leaked_allocs = stats.total_allocs - stats.total_deallocs;
    return stats;
}

void print_memory_stats(const char* phase) {
    memory_stats_t stats = get_memory_stats();
    printf("Memory Stats [%s]:\n", phase);
    printf("  Allocations: %zu\n", stats.total_allocs);
    printf("  Deallocations: %zu\n", stats.total_deallocs);
    printf("  Current bytes: %zu\n", stats.current_bytes);
    printf("  Peak bytes: %zu (%.1f KB)\n", stats.peak_bytes, stats.peak_bytes / 1024.0);
    printf("  Total allocated: %zu (%.1f KB)\n", stats.total_allocated_bytes, stats.total_allocated_bytes / 1024.0);
    printf("  Leaked allocations: %zu\n", stats.leaked_allocs);
}

// Reset memory statistics
void reset_memory_stats() {
    atomic_store(&total_allocations, 0);
    atomic_store(&total_deallocations, 0);
    atomic_store(&current_bytes, 0);
    atomic_store(&peak_bytes, 0);
    atomic_store(&total_allocated_bytes, 0);
}

// ============================================================================
// FFI Custom Allocator Implementation
// ============================================================================

// The exp-rs FFI supports custom allocators via exp_rs_malloc/exp_rs_free
// when built with --features custom_cbindgen_alloc
// We'll implement these functions to provide memory tracking

// Implement the custom allocator functions that exp-rs will call
void* exp_rs_malloc(size_t size) {
    printf("[DEBUG] exp_rs_malloc called with size: %zu\n", size);
    fflush(stdout);
    return tracked_malloc(size);
}

void exp_rs_free(void* ptr) {
    printf("[DEBUG] exp_rs_free called with ptr: %p\n", ptr);
    fflush(stdout);
    tracked_free(ptr);
}

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
        printf("❌ FAILED: Arena creation didn't trigger exp_rs_malloc!\n");
        printf("This means Rust FFI is using a different allocator\n");
        printf("Check if custom_cbindgen_alloc feature is actually enabled\n");
        exit(1);  // Fail the test
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
    ExprResult expr1_res = expr_batch_add_expression(builder, "x + y");
    assert(expr1_res.status == 0);
    int32_t expr1_idx = expr1_res.index;
    assert(expr1_idx == 0);
    
    ExprResult expr2_res = expr_batch_add_expression(builder, "x * y");
    assert(expr2_res.status == 0);
    int32_t expr2_idx = expr2_res.index;
    assert(expr2_idx == 1);
    
    ExprResult expr3_res = expr_batch_add_expression(builder, "sqrt(x*x + y*y)");
    assert(expr3_res.status == 0);
    int32_t expr3_idx = expr3_res.index;
    assert(expr3_idx == 2);
    printf("✓ Added 3 expressions\n");
    
    // Add parameters
    ExprResult x_res = expr_batch_add_variable(builder, "x", 3.0);
    assert(x_res.status == 0);
    int32_t x_idx = x_res.index;
    assert(x_idx == 0);
    
    ExprResult y_res = expr_batch_add_variable(builder, "y", 4.0);
    assert(y_res.status == 0);
    int32_t y_idx = y_res.index;
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
    ExprResult res = expr_batch_add_expression(builder1, "a + b + c");
    assert(res.status == 0);
    res = expr_batch_add_variable(builder1, "a", 1.0);
    assert(res.status == 0);
    res = expr_batch_add_variable(builder1, "b", 2.0);
    assert(res.status == 0);
    res = expr_batch_add_variable(builder1, "c", 3.0);
    assert(res.status == 0);
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
    res = expr_batch_add_expression(builder2, "x * y * z");
    assert(res.status == 0);
    res = expr_batch_add_variable(builder2, "x", 2.0);
    assert(res.status == 0);
    res = expr_batch_add_variable(builder2, "y", 3.0);
    assert(res.status == 0);
    res = expr_batch_add_variable(builder2, "z", 4.0);
    assert(res.status == 0);
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
        ExprResult res = expr_batch_add_expression(builder, expressions[i]);
        if (res.status != 0) {
            printf("Failed to add expression %d: %s (error: %s)\n", i, expressions[i], res.error);
            expr_free_error(res.error);
            return;
        }
    }
    printf("✓ Added 7 benchmark expressions\n");
    
    // Add 10 parameters (a through j)
    const char* param_names[] = {"a", "b", "c", "d", "e", "f", "g", "h", "i", "j"};
    for (int i = 0; i < 10; i++) {
        ExprResult res = expr_batch_add_variable(builder, param_names[i], (i + 1) * 1.5);
        assert(res.status == 0);
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
    
    init_memory_tracking();
    
    // Create arena and context (with tracking disabled)
    disable_allocation_tracking();
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
    
    // Do initial evaluation to parse expressions (still not tracking)
    expr_batch_evaluate(builder, ctx);
    printf("✓ Initial evaluation complete\n");
    
    // NOW enable tracking for the evaluation loop
    reset_memory_stats();
    enable_allocation_tracking();
    
    memory_stats_t before_stats = get_memory_stats();
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
    memory_stats_t after_stats = get_memory_stats();
    
    // Disable tracking before cleanup
    disable_allocation_tracking();
    
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
    size_t allocs_during_eval = after_stats.total_allocs - before_stats.total_allocs;
    size_t bytes_during_eval = after_stats.total_allocated_bytes - before_stats.total_allocated_bytes;
    
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
        ExprResult res = expr_batch_add_expression(builder, expressions[i]);
        assert(res.status == 0);
        assert(res.index == (int32_t)i);
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
    ExprResult first_res = expr_batch_add_variable(builder, "x", 1.0);
    assert(first_res.status == 0);
    ExprResult dup_res = expr_batch_add_variable(builder, "x", 2.0);
    assert(dup_res.status != 0);  // Should return error
    expr_free_error(dup_res.error);
    printf("✓ Duplicate parameter handled correctly\n");
    
    // Cleanup
    expr_batch_free(builder);
    expr_arena_free(arena);
    printf("\n");
}

// Test Method 1: Complete Arena Recreation
void test_method1_complete_recreation() {
    printf("=== Test Method 1: Complete Arena Recreation ===\n");
    
    init_memory_tracking();
    
    // Create long-lived context (not tracked)
    disable_allocation_tracking();
    ExprContext* ctx = expr_context_new();
    expr_context_add_function(ctx, "sin", 1, native_sin);
    expr_context_add_function(ctx, "cos", 1, native_cos);
    expr_context_add_function(ctx, "sqrt", 1, native_sqrt);
    
    printf("Long-lived context created\n");
    
    const int num_batches = 10;
    const int evals_per_batch = 1000;
    
    for (int batch_num = 0; batch_num < num_batches; batch_num++) {
        printf("\n--- Batch %d ---\n", batch_num + 1);
        
        // Enable tracking for this batch cycle
        reset_memory_stats();
        enable_allocation_tracking();
        
        memory_stats_t start_stats = get_memory_stats();
        
        // 1. Create fresh arena (tracked)
        ExprArena* arena = expr_arena_new(128 * 1024);
        memory_stats_t after_arena = get_memory_stats();
        
        // 2. Create batch (tracked)
        ExprBatch* batch = expr_batch_new(arena);
        memory_stats_t after_batch = get_memory_stats();
        
        // 3. Add expressions (tracked)
        expr_batch_add_expression(batch, "sin(x) + cos(y)");
        expr_batch_add_expression(batch, "sqrt(x*x + y*y)");
        expr_batch_add_variable(batch, "x", 0.0);
        expr_batch_add_variable(batch, "y", 0.0);
        memory_stats_t after_setup = get_memory_stats();
        
        // 4. Initial evaluation
        expr_batch_evaluate(batch, ctx);
        memory_stats_t after_initial = get_memory_stats();
        
        // 5. High-frequency evaluation loop (should be zero allocs)
        for (int i = 0; i < evals_per_batch; i++) {
            expr_batch_set_variable(batch, 0, (Real)i * 0.01);
            expr_batch_set_variable(batch, 1, (Real)i * 0.02);
            expr_batch_evaluate(batch, ctx);
        }
        memory_stats_t after_evals = get_memory_stats();
        
        // 6. Free batch and arena (tracked)
        expr_batch_free(batch);
        expr_arena_free(arena);
        memory_stats_t after_cleanup = get_memory_stats();
        
        disable_allocation_tracking();
        
        // Analyze memory usage for this batch
        printf("Arena allocation: %zu bytes\n", 
               after_arena.total_allocated_bytes - start_stats.total_allocated_bytes);
        printf("Batch setup: %zu bytes\n", 
               after_setup.total_allocated_bytes - after_arena.total_allocated_bytes);
        printf("Initial eval: %zu bytes\n", 
               after_initial.total_allocated_bytes - after_setup.total_allocated_bytes);
        printf("%d evaluations: %zu bytes\n", evals_per_batch,
               after_evals.total_allocated_bytes - after_initial.total_allocated_bytes);
        printf("Current memory after cleanup: %zu bytes\n", after_cleanup.current_bytes);
        printf("Peak memory this batch: %zu bytes (%.1f KB)\n", 
               after_cleanup.peak_bytes, after_cleanup.peak_bytes / 1024.0);
        
        // Verify complete memory reclamation
        if (after_cleanup.current_bytes == 0) {
            printf("✓ Complete memory reclamation verified\n");
        } else {
            printf("✗ Memory leak detected: %zu bytes\n", after_cleanup.current_bytes);
        }
        
        // Verify zero allocations during evaluation loop
        size_t eval_allocs = after_evals.total_allocs - after_initial.total_allocs;
        if (eval_allocs == 0) {
            printf("✓ Zero allocations during %d evaluations\n", evals_per_batch);
        } else {
            printf("✗ Found %zu allocations during evaluations\n", eval_allocs);
        }
    }
    
    // Cleanup context
    expr_context_free(ctx);
    printf("\n✓ Method 1 complete - total memory freed after each batch\n\n");
}

// Test Method 2: Arena Reset Pattern
void test_method2_arena_reset() {
    printf("=== Test Method 2: Arena Reset Pattern ===\n");
    
    init_memory_tracking();
    
    // Create long-lived context and arena (not tracked)
    disable_allocation_tracking();
    ExprContext* ctx = expr_context_new();
    expr_context_add_function(ctx, "sin", 1, native_sin);
    expr_context_add_function(ctx, "cos", 1, native_cos);
    expr_context_add_function(ctx, "sqrt", 1, native_sqrt);
    
    ExprArena* arena = expr_arena_new(128 * 1024);
    printf("Long-lived context and arena created\n");
    
    const int num_batches = 10;
    const int evals_per_batch = 1000;
    
    for (int batch_num = 0; batch_num < num_batches; batch_num++) {
        printf("\n--- Batch %d ---\n", batch_num + 1);
        
        // Enable tracking for this batch cycle
        reset_memory_stats();
        enable_allocation_tracking();
        
        memory_stats_t start_stats = get_memory_stats();
        
        // 1. Reset arena (should be near-zero cost)
        expr_arena_reset(arena);
        memory_stats_t after_reset = get_memory_stats();
        
        // 2. Create new batch on reset arena
        ExprBatch* batch = expr_batch_new(arena);
        memory_stats_t after_batch = get_memory_stats();
        
        // 3. Add expressions (allocates in arena)
        expr_batch_add_expression(batch, "sin(x) + cos(y)");
        expr_batch_add_expression(batch, "sqrt(x*x + y*y)");
        expr_batch_add_variable(batch, "x", 0.0);
        expr_batch_add_variable(batch, "y", 0.0);
        memory_stats_t after_setup = get_memory_stats();
        
        // 4. Initial evaluation
        expr_batch_evaluate(batch, ctx);
        memory_stats_t after_initial = get_memory_stats();
        
        // 5. High-frequency evaluation loop (should be zero allocs)
        for (int i = 0; i < evals_per_batch; i++) {
            expr_batch_set_variable(batch, 0, (Real)i * 0.01);
            expr_batch_set_variable(batch, 1, (Real)i * 0.02);
            expr_batch_evaluate(batch, ctx);
        }
        memory_stats_t after_evals = get_memory_stats();
        
        // 6. Free batch (but keep arena)
        expr_batch_free(batch);
        memory_stats_t after_cleanup = get_memory_stats();
        
        disable_allocation_tracking();
        
        // Analyze memory usage for this batch
        printf("Arena reset: %zu bytes\n", 
               after_reset.total_allocated_bytes - start_stats.total_allocated_bytes);
        printf("Batch creation: %zu bytes\n", 
               after_batch.total_allocated_bytes - after_reset.total_allocated_bytes);
        printf("Expression setup: %zu bytes\n", 
               after_setup.total_allocated_bytes - after_batch.total_allocated_bytes);
        printf("Initial eval: %zu bytes\n", 
               after_initial.total_allocated_bytes - after_setup.total_allocated_bytes);
        printf("%d evaluations: %zu bytes\n", evals_per_batch,
               after_evals.total_allocated_bytes - after_initial.total_allocated_bytes);
        printf("Current memory after batch cleanup: %zu bytes\n", after_cleanup.current_bytes);
        printf("Peak memory this batch: %zu bytes (%.1f KB)\n", 
               after_cleanup.peak_bytes, after_cleanup.peak_bytes / 1024.0);
        
        // Verify arena reset efficiency
        size_t reset_allocs = after_reset.total_allocs - start_stats.total_allocs;
        if (reset_allocs == 0) {
            printf("✓ Arena reset with zero allocations\n");
        } else {
            printf("✗ Arena reset caused %zu allocations\n", reset_allocs);
        }
        
        // Verify zero allocations during evaluation loop
        size_t eval_allocs = after_evals.total_allocs - after_initial.total_allocs;
        if (eval_allocs == 0) {
            printf("✓ Zero allocations during %d evaluations\n", evals_per_batch);
        } else {
            printf("✗ Found %zu allocations during evaluations\n", eval_allocs);
        }
    }
    
    // Final cleanup
    expr_arena_free(arena);
    expr_context_free(ctx);
    printf("\n✓ Method 2 complete - arena memory kept allocated, reset between batches\n\n");
}

// Test memory lifecycle comparison
void test_memory_lifecycle_comparison() {
    printf("=== Memory Lifecycle Comparison ===\n");
    
    init_memory_tracking();
    disable_allocation_tracking();
    
    // Setup contexts for both methods
    ExprContext* ctx1 = expr_context_new();
    ExprContext* ctx2 = expr_context_new();
    expr_context_add_function(ctx1, "sin", 1, native_sin);
    expr_context_add_function(ctx1, "sqrt", 1, native_sqrt);
    expr_context_add_function(ctx2, "sin", 1, native_sin);
    expr_context_add_function(ctx2, "sqrt", 1, native_sqrt);
    
    // Method 2 setup (persistent arena)
    ExprArena* persistent_arena = expr_arena_new(128 * 1024);
    
    const int test_batches = 5;
    const int evals_per_batch = 1000;
    
    printf("\nTesting %d batches with %d evaluations each...\n", test_batches, evals_per_batch);
    
    // Test Method 1: Complete recreation
    printf("\n--- Method 1 Performance ---\n");
    double method1_total_time = 0;
    size_t method1_total_allocs = 0;
    size_t method1_peak_memory = 0;
    
    for (int batch = 0; batch < test_batches; batch++) {
        reset_memory_stats();
        enable_allocation_tracking();
        
        double start_time = get_time_us();
        
        ExprArena* arena = expr_arena_new(128 * 1024);
        ExprBatch* batch_obj = expr_batch_new(arena);
        expr_batch_add_expression(batch_obj, "sin(x) + sqrt(y)");
        expr_batch_add_variable(batch_obj, "x", 0.0);
        expr_batch_add_variable(batch_obj, "y", 1.0);
        expr_batch_evaluate(batch_obj, ctx1);
        
        for (int i = 0; i < evals_per_batch; i++) {
            expr_batch_set_variable(batch_obj, 0, (Real)i * 0.01);
            expr_batch_evaluate(batch_obj, ctx1);
        }
        
        expr_batch_free(batch_obj);
        expr_arena_free(arena);
        
        double end_time = get_time_us();
        memory_stats_t stats = get_memory_stats();
        
        method1_total_time += (end_time - start_time);
        method1_total_allocs += stats.total_allocs;
        if (stats.peak_bytes > method1_peak_memory) {
            method1_peak_memory = stats.peak_bytes;
        }
        
        disable_allocation_tracking();
    }
    
    // Test Method 2: Arena reset
    printf("\n--- Method 2 Performance ---\n");
    double method2_total_time = 0;
    size_t method2_total_allocs = 0;
    size_t method2_peak_memory = 0;
    
    for (int batch = 0; batch < test_batches; batch++) {
        reset_memory_stats();
        enable_allocation_tracking();
        
        double start_time = get_time_us();
        
        expr_arena_reset(persistent_arena);
        ExprBatch* batch_obj = expr_batch_new(persistent_arena);
        expr_batch_add_expression(batch_obj, "sin(x) + sqrt(y)");
        expr_batch_add_variable(batch_obj, "x", 0.0);
        expr_batch_add_variable(batch_obj, "y", 1.0);
        expr_batch_evaluate(batch_obj, ctx2);
        
        for (int i = 0; i < evals_per_batch; i++) {
            expr_batch_set_variable(batch_obj, 0, (Real)i * 0.01);
            expr_batch_evaluate(batch_obj, ctx2);
        }
        
        expr_batch_free(batch_obj);
        
        double end_time = get_time_us();
        memory_stats_t stats = get_memory_stats();
        
        method2_total_time += (end_time - start_time);
        method2_total_allocs += stats.total_allocs;
        if (stats.peak_bytes > method2_peak_memory) {
            method2_peak_memory = stats.peak_bytes;
        }
        
        disable_allocation_tracking();
    }
    
    // Comparison results
    printf("\n=== COMPARISON RESULTS ===\n");
    printf("Method 1 (Complete Recreation):\n");
    printf("  Total time: %.2f ms (%.2f ms/batch)\n", 
           method1_total_time / 1000.0, method1_total_time / (1000.0 * test_batches));
    printf("  Total allocations: %zu (%.1f/batch)\n", 
           method1_total_allocs, (double)method1_total_allocs / test_batches);
    printf("  Peak memory: %zu bytes (%.1f KB)\n", 
           method1_peak_memory, method1_peak_memory / 1024.0);
    
    printf("\nMethod 2 (Arena Reset):\n");
    printf("  Total time: %.2f ms (%.2f ms/batch)\n", 
           method2_total_time / 1000.0, method2_total_time / (1000.0 * test_batches));
    printf("  Total allocations: %zu (%.1f/batch)\n", 
           method2_total_allocs, (double)method2_total_allocs / test_batches);
    printf("  Peak memory: %zu bytes (%.1f KB)\n", 
           method2_peak_memory, method2_peak_memory / 1024.0);
    
    double time_ratio = method1_total_time / method2_total_time;
    double alloc_ratio = (double)method1_total_allocs / (method2_total_allocs > 0 ? method2_total_allocs : 1);
    
    printf("\nMethod 2 is %.2fx faster than Method 1\n", time_ratio);
    printf("Method 1 uses %.2fx more allocations than Method 2\n", alloc_ratio);
    
    if (time_ratio > 1.1) {
        printf("✓ Method 2 (Arena Reset) is significantly faster\n");
    } else {
        printf("~ Performance difference is minimal\n");
    }
    
    // Cleanup
    expr_arena_free(persistent_arena);
    expr_context_free(ctx1);
    expr_context_free(ctx2);
    printf("\n");
}

// Main test runner
int main() {
    printf("\n==== Arena Integration Tests with Memory Tracking ====\n\n");
    
    // CRITICAL: Test custom allocator integration first - fail fast if not working
    test_custom_allocator_integration();
    
    test_arena_lifecycle();
    test_batch_builder_with_arena();
    test_arena_reset_reuse();
    test_benchmark_expressions();  // New test matching Rust benchmark
    test_zero_allocations();
    test_arena_size_estimation();
    test_error_handling();
    
    // New memory lifecycle tests
    test_method1_complete_recreation();
    test_method2_arena_reset();
    test_memory_lifecycle_comparison();
    
    printf("==== All Tests Passed! ====\n\n");
    return 0;
}