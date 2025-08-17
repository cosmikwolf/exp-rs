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

// Test single evaluation with detailed tracking
void test_single_evaluation_profile() {
    printf("=== Single Evaluation Allocation Profile ===\n");
    
    init_memory_tracking();
    enable_allocation_tracking();
    reset_memory_stats();
    
    printf("Creating arena and context...\n");
    memory_stats_t start = get_memory_stats();
    
    ExprArena* arena = expr_arena_new(256 * 1024);
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
    
    ExprBatch* builder = expr_batch_new(arena);
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
    
    printf("\n--- FIRST EVALUATION (parsing) ---\n");
    memory_stats_t before_first = get_memory_stats();
    expr_batch_evaluate(builder, ctx);
    memory_stats_t after_first = get_memory_stats();
    
    printf("First eval: +%zu allocs, +%zu bytes\n", 
           after_first.total_allocs - before_first.total_allocs,
           after_first.total_allocated_bytes - before_first.total_allocated_bytes);
    
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
    } else {
        printf("❌ INCONSISTENT: Varying allocation patterns detected\n");
    }
    
    // Cleanup
    expr_batch_free(builder);
    expr_context_free(ctx);
    expr_arena_free(arena);
    
    disable_allocation_tracking();
}

int main() {
    printf("\n==== Single Evaluation Allocation Profiler ====\n\n");
    test_single_evaluation_profile();
    printf("\n==== Analysis Complete ====\n\n");
    return 0;
}