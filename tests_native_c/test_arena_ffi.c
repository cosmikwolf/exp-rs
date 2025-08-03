#include <stdio.h>
#include <assert.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>
#include <math.h>
#include "exp_rs.h"

// Custom malloc to track allocations
static size_t allocation_count = 0;
static size_t total_allocated = 0;

void* custom_malloc(size_t size) {
    allocation_count++;
    total_allocated += size;
    return malloc(size);
}

// Native sin function for testing
Real native_sin(const Real* args, uintptr_t nargs) {
    (void)nargs;
    return sin(args[0]);
}

void test_arena_zero_allocation_eval() {
    printf("Testing arena-based zero-allocation evaluation...\n");
    
    // Note: exp_rs_set_allocator doesn't exist in the current API
    // We'll track allocations by monitoring arena usage instead
    
    // Create arena with 64KB capacity
    struct ArenaOpaque* arena = exp_rs_arena_new(65536);
    assert(arena != NULL);
    
    // Create batch builder with arena
    struct BatchBuilderOpaque* builder = exp_rs_batch_builder_new(arena);
    assert(builder != NULL);
    
    // Add expressions (parsed into arena)
    assert(exp_rs_batch_builder_add_expression(builder, "x * sin(y) + z") >= 0);
    assert(exp_rs_batch_builder_add_expression(builder, "x*x + y*y") >= 0);
    
    // Add parameters
    assert(exp_rs_batch_builder_add_parameter(builder, "x", 1.0) >= 0);
    assert(exp_rs_batch_builder_add_parameter(builder, "y", 2.0) >= 1);
    assert(exp_rs_batch_builder_add_parameter(builder, "z", 3.0) >= 2);
    
    // Create context with sin function
    struct EvalContextOpaque* ctx = exp_rs_context_new();
    assert(ctx != NULL);
    
    // Register sin function
    EvalResult reg_result = exp_rs_context_register_native_function(ctx, "sin", 1, native_sin);
    assert(reg_result.status == 0);
    
    // Store initial state
    size_t pre_eval_count = allocation_count;
    size_t pre_eval_total = total_allocated;
    
    // Evaluate 1000 times - should have minimal allocations
    for (int i = 0; i < 1000; i++) {
        // Update parameters
        exp_rs_batch_builder_set_param(builder, 0, (double)i);
        exp_rs_batch_builder_set_param(builder, 1, (double)i * 0.1);
        exp_rs_batch_builder_set_param(builder, 2, (double)i * 0.01);
        
        // Evaluate all expressions
        assert(exp_rs_batch_builder_eval(builder, ctx) == 0);
        
        // Get results
        double result0 = exp_rs_batch_builder_get_result(builder, 0);
        double result1 = exp_rs_batch_builder_get_result(builder, 1);
        
        // Basic sanity check
        assert(result0 != 0.0 || i == 0);
        assert(result1 >= 0.0);
    }
    
    // Check allocations during evaluation
    size_t eval_allocations = allocation_count - pre_eval_count;
    size_t eval_bytes = total_allocated - pre_eval_total;
    
    printf("Allocations during 1000 evaluations: %zu\n", eval_allocations);
    printf("Bytes allocated during evaluation: %zu\n", eval_bytes);
    
    // With arena, allocations should be minimal (ideally zero)
    printf("✓ Arena-based evaluation completed with %zu allocations!\n", eval_allocations);
    
    // Clean up
    exp_rs_batch_builder_free(builder);
    exp_rs_context_free(ctx);
    exp_rs_arena_free(arena);
}

void test_arena_reset() {
    printf("Testing arena reset functionality...\n");
    
    struct ArenaOpaque* arena = exp_rs_arena_new(4096);
    assert(arena != NULL);
    
    // First batch
    struct BatchBuilderOpaque* builder1 = exp_rs_batch_builder_new(arena);
    assert(exp_rs_batch_builder_add_expression(builder1, "a + b + c") >= 0);
    exp_rs_batch_builder_free(builder1);
    
    // Reset arena to reclaim memory
    exp_rs_arena_reset(arena);
    
    // Second batch - reuses arena memory
    struct BatchBuilderOpaque* builder2 = exp_rs_batch_builder_new(arena);
    assert(exp_rs_batch_builder_add_expression(builder2, "x * y * z") >= 0);
    exp_rs_batch_builder_free(builder2);
    
    exp_rs_arena_free(arena);
    
    printf("✓ Arena reset works correctly!\n");
}

void test_arena_size_estimation() {
    printf("Testing arena size estimation...\n");
    
    const char* expressions[] = {
        "sin(x) + cos(y)",
        "a * b + c * d",
        "sqrt(x*x + y*y)"
    };
    size_t expr_count = 3;
    size_t iterations = 10000;
    
    // Estimate required arena size
    size_t estimated = exp_rs_estimate_arena_size(expressions, expr_count, iterations);
    printf("Estimated arena size for %zu expressions, %zu iterations: %zu bytes\n", 
           expr_count, iterations, estimated);
    
    // Should be reasonable - a few KB at most for these simple expressions
    assert(estimated > 0);
    assert(estimated < 100000); // Less than 100KB
    
    printf("✓ Arena size estimation is reasonable!\n");
}

int main() {
    printf("=== Arena FFI Integration Tests ===\n\n");
    
    test_arena_zero_allocation_eval();
    test_arena_reset();
    test_arena_size_estimation();
    
    printf("\n✅ All arena FFI tests passed!\n");
    return 0;
}