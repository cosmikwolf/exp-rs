#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <time.h>
#include <math.h>
#include "exp_rs.h"
#include "common_allocator.h"

// Native function implementations
Real native_sin(const Real* args, uintptr_t nargs) { (void)nargs; return sin(args[0]); }
Real native_cos(const Real* args, uintptr_t nargs) { (void)nargs; return cos(args[0]); }
Real native_sqrt(const Real* args, uintptr_t nargs) { (void)nargs; return sqrt(args[0]); }

// Test memory allocation patterns and arena usage
void test_memory_and_arena_analysis() {
    printf("=== Memory and Arena Usage Analysis ===\n");
    
    init_memory_tracking();
    enable_allocation_tracking();
    reset_memory_stats();
    
    printf("Creating arena and context...\n");
    memory_stats_t start = get_memory_stats();
    
    // Try with a minimal arena - 4KB to see how much it actually needs
    // Arena is now managed internally by batch
    memory_stats_t after_arena = get_memory_stats();
    printf("Arena: +%zu allocs, +%zu bytes\n", 
           after_arena.total_allocs - start.total_allocs,
           after_arena.total_allocated_bytes - start.total_allocated_bytes);
    
    ExprContext* ctx = expr_context_new();
    memory_stats_t after_context = get_memory_stats();
    printf("Context: +%zu allocs, +%zu bytes\n", 
           after_context.total_allocs - after_arena.total_allocs,
           after_context.total_allocated_bytes - after_arena.total_allocated_bytes);
    
    expr_context_add_function(ctx, "sin", 1, native_sin);
    expr_context_add_function(ctx, "cos", 1, native_cos);
    expr_context_add_function(ctx, "sqrt", 1, native_sqrt);
    
    ExprBatch* builder = expr_batch_new(4 * 1024);
    memory_stats_t after_batch = get_memory_stats();
    printf("Batch: +%zu allocs, +%zu bytes\n", 
           after_batch.total_allocs - after_context.total_allocs,
           after_batch.total_allocated_bytes - after_context.total_allocated_bytes);
    
    expr_batch_add_expression(builder, "sin(x) * cos(y) + sqrt(x*x + y*y)");
    expr_batch_add_variable(builder, "x", 1.0);
    expr_batch_add_variable(builder, "y", 2.0);
    
    memory_stats_t after_setup = get_memory_stats();
    printf("Expression setup: +%zu allocs, +%zu bytes\n", 
           after_setup.total_allocs - after_batch.total_allocs,
           after_setup.total_allocated_bytes - after_batch.total_allocated_bytes);
    
    // Check arena usage
    size_t arena_bytes_initial = expr_batch_arena_bytes(builder);
    printf("Arena usage after setup: %zu bytes\n", arena_bytes_initial);
    
    // Show memory state before any evaluations
    printf("\n=== MEMORY USAGE BEFORE EVALUATIONS ===\n");
    printf("Total allocations:          %zu\n", after_setup.total_allocs);
    printf("Total deallocations:        %zu\n", after_setup.total_deallocs);
    printf("Total allocated bytes ever: %zu\n", after_setup.total_allocated_bytes);
    printf("Currently in use:           %zu bytes\n", after_setup.current_bytes);
    printf("Peak memory usage:          %zu bytes\n", after_setup.peak_bytes);
    printf("Leaked allocations:         %zu\n", after_setup.leaked_allocs);
    
    printf("\n--- FIRST EVALUATION (parsing) ---\n");
    memory_stats_t before_first = get_memory_stats();
    expr_batch_evaluate(builder, ctx);
    memory_stats_t after_first = get_memory_stats();
    size_t arena_bytes_after_first = expr_batch_arena_bytes(builder);
    
    printf("First eval: +%zu allocs, +%zu bytes\n", 
           after_first.total_allocs - before_first.total_allocs,
           after_first.total_allocated_bytes - before_first.total_allocated_bytes);
    printf("Arena growth: %zu bytes (now %zu total)\n", 
           arena_bytes_after_first - arena_bytes_initial, arena_bytes_after_first);
    
    printf("\n--- SECOND EVALUATION (should be cached) ---\n");
    memory_stats_t before_second = get_memory_stats();
    expr_batch_set_variable(builder, 0, 3.14);
    expr_batch_set_variable(builder, 1, 2.71);
    expr_batch_evaluate(builder, ctx);
    memory_stats_t after_second = get_memory_stats();
    
    printf("Second eval: +%zu allocs, +%zu bytes\n", 
           after_second.total_allocs - before_second.total_allocs,
           after_second.total_allocated_bytes - before_second.total_allocated_bytes);
    
    printf("\n--- THIRD EVALUATION (verify pattern) ---\n");
    memory_stats_t before_third = get_memory_stats();
    expr_batch_set_variable(builder, 0, 1.41);
    expr_batch_set_variable(builder, 1, 1.73);
    expr_batch_evaluate(builder, ctx);
    memory_stats_t after_third = get_memory_stats();
    
    printf("Third eval: +%zu allocs, +%zu bytes\n", 
           after_third.total_allocs - before_third.total_allocs,
           after_third.total_allocated_bytes - before_third.total_allocated_bytes);
    
    // Do many more evaluations to check for memory leak
    printf("\n--- MEMORY LEAK TEST (10000 evaluations) ---\n");
    memory_stats_t before_many = get_memory_stats();
    size_t arena_bytes_before_many = expr_batch_arena_bytes(builder);
    size_t allocs_per_eval = 0;
    size_t bytes_per_eval = 0;
    
    for (int i = 0; i < 10000; i++) {
        expr_batch_set_variable(builder, 0, i * 0.1);
        expr_batch_set_variable(builder, 1, i * 0.2);
        expr_batch_evaluate(builder, ctx);
    }
    
    memory_stats_t after_many = get_memory_stats();
    size_t arena_bytes_after_many = expr_batch_arena_bytes(builder);
    size_t total_allocs_10000 = after_many.total_allocs - before_many.total_allocs;
    size_t total_bytes_10000 = after_many.total_allocated_bytes - before_many.total_allocated_bytes;
    
    printf("10000 evaluations: +%zu allocs, +%zu bytes\n", total_allocs_10000, total_bytes_10000);
    printf("Average per eval: %.2f allocs, %.2f bytes\n", 
           total_allocs_10000 / 10000.0, total_bytes_10000 / 10000.0);
    printf("Arena growth during 10000 evals: %zu bytes\n", 
           arena_bytes_after_many - arena_bytes_before_many);
    
    if (arena_bytes_after_many == arena_bytes_before_many) {
        printf("✅ Arena: Zero growth during evaluations!\n");
    } else {
        printf("⚠️  Arena: Grew by %zu bytes\n", arena_bytes_after_many - arena_bytes_before_many);
    }
    
    // Check current memory usage vs deallocations
    printf("\n=== MEMORY USAGE AFTER ALL EVALUATIONS ===\n");
    printf("Total allocations:          %zu\n", after_many.total_allocs);
    printf("Total deallocations:        %zu\n", after_many.total_deallocs);
    printf("Total allocated bytes ever: %zu\n", after_many.total_allocated_bytes);
    printf("Currently in use:           %zu bytes\n", after_many.current_bytes);
    printf("Peak memory usage:          %zu bytes\n", after_many.peak_bytes);
    printf("Leaked allocations:         %zu\n", after_many.leaked_allocs);
    
    // Analyze the pattern
    size_t first_eval_allocs = after_first.total_allocs - before_first.total_allocs;
    size_t second_eval_allocs = after_second.total_allocs - before_second.total_allocs;
    size_t third_eval_allocs = after_third.total_allocs - before_third.total_allocs;
    
    printf("\n=== ALLOCATION PATTERN ANALYSIS ===\n");
    printf("First evaluation (parsing): %zu allocations\n", first_eval_allocs);
    printf("Second evaluation (cached): %zu allocations\n", second_eval_allocs);
    printf("Third evaluation (cached):  %zu allocations\n", third_eval_allocs);
    
    if (second_eval_allocs == 0 && third_eval_allocs == 0) {
        printf("✅ SUCCESS: Zero allocations after initial parsing!\n");
    } else if (second_eval_allocs == third_eval_allocs && second_eval_allocs > 0) {
        printf("⚠️  PATTERN: Consistent %zu allocations per evaluation\n", second_eval_allocs);
        printf("   This suggests the allocations are NOT for parsing/caching\n");
        printf("   They're likely for evaluation temporaries or intermediate results\n");
        
        // Check if memory is being freed properly
        size_t net_allocs = after_many.total_allocs - after_many.total_deallocs;
        size_t initial_net_allocs = after_setup.total_allocs - after_setup.total_deallocs;
        size_t net_bytes_growth = after_many.current_bytes - before_many.current_bytes;
        
        if (net_bytes_growth == 0) {
            printf("   ✅ GOOD: Memory is being properly freed (no leak)\n");
        } else if (net_bytes_growth < 1000) {
            printf("   ⚠️  MINOR: Small amount of memory retained: %zu bytes per 10000 evals\n", net_bytes_growth);
        } else {
            printf("   ❌ LEAK: Memory is leaking at %zu bytes per 10000 evaluations\n", net_bytes_growth);
            printf("        That's %.4f bytes per evaluation\n", net_bytes_growth / 10000.0);
        }
    } else {
        printf("❌ INCONSISTENT: Varying allocation patterns detected\n");
    }
    
    // Show arena statistics
    printf("\n=== ARENA USAGE SUMMARY ===\n");
    size_t final_arena_bytes = expr_batch_arena_bytes(builder);
    size_t arena_initial_size = 4 * 1024; // 4 KB
    printf("Arena initial capacity: %zu bytes (4 KB)\n", arena_initial_size);
    printf("Arena bytes used:       %zu bytes (%.2f%% of initial)\n", 
           final_arena_bytes, (final_arena_bytes * 100.0) / arena_initial_size);
    if (final_arena_bytes > arena_initial_size) {
        printf("Arena has grown beyond initial capacity by %zu bytes\n", 
               final_arena_bytes - arena_initial_size);
    } else {
        printf("Arena bytes free:       %zu bytes\n", arena_initial_size - final_arena_bytes);
    }
    
    // Cleanup and verify memory
    printf("\n=== CLEANUP AND MEMORY VERIFICATION ===\n");
    
    // Free batch first
    expr_batch_free(builder);
    memory_stats_t after_batch_free = get_memory_stats();
    printf("After freeing batch: %zu bytes in use\n", after_batch_free.current_bytes);
    
    // Free arena
    memory_stats_t after_arena_free = get_memory_stats();
    printf("After freeing arena: %zu bytes in use\n", after_arena_free.current_bytes);
    
    // Free context
    expr_context_free(ctx);
    memory_stats_t after_context_free = get_memory_stats();
    printf("After freeing context: %zu bytes in use\n", after_context_free.current_bytes);
    
    // Verify all memory is freed
    if (after_context_free.current_bytes == 0) {
        printf("✅ SUCCESS: All memory properly freed!\n");
    } else {
        printf("❌ ERROR: %zu bytes still in use after cleanup!\n", after_context_free.current_bytes);
        printf("   This indicates a memory leak in the library.\n");
    }
    
    // Check that allocations match deallocations
    if (after_context_free.total_allocs == after_context_free.total_deallocs) {
        printf("✅ Allocations balanced: %zu allocs == %zu deallocs\n", 
               after_context_free.total_allocs, after_context_free.total_deallocs);
    } else {
        printf("⚠️  Unbalanced: %zu allocs vs %zu deallocs (diff: %zd)\n",
               after_context_free.total_allocs, after_context_free.total_deallocs,
               (ssize_t)(after_context_free.total_allocs - after_context_free.total_deallocs));
    }
    
    disable_allocation_tracking();
}

int main() {
    printf("\n==== Expression Evaluation Memory Analysis ====\n\n");
    test_memory_and_arena_analysis();
    printf("\n==== Analysis Complete ====\n\n");
    return 0;
}