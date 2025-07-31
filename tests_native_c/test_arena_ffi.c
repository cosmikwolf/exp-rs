#include <stdio.h>
#include <assert.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>
#include "../target/exp_rs.h"

// Custom malloc to track allocations
static size_t allocation_count = 0;
static size_t total_allocated = 0;

void* custom_malloc(size_t size) {
    allocation_count++;
    total_allocated += size;
    return malloc(size);
}

void test_arena_zero_allocation_eval() {
    printf("Testing arena-based zero-allocation evaluation...\n");
    
    // Configure allocator to track allocations
    exp_rs_set_allocator(custom_malloc, free);
    
    // Create arena with 64KB capacity
    Arena* arena = exp_rs_arena_new(65536);
    assert(arena != NULL);
    
    // Create batch builder with arena
    BatchBuilder* builder = exp_rs_batch_builder_new_with_arena(arena);
    assert(builder != NULL);
    
    // Add expressions (parsed into arena)
    assert(exp_rs_batch_builder_add_expression(builder, "x * sin(y) + z") == 0);
    assert(exp_rs_batch_builder_add_expression(builder, "x^2 + y^2") == 1);
    
    // Add parameters
    assert(exp_rs_batch_builder_add_parameter(builder, "x", 1.0) == 0);
    assert(exp_rs_batch_builder_add_parameter(builder, "y", 2.0) == 1);
    assert(exp_rs_batch_builder_add_parameter(builder, "z", 3.0) == 2);
    
    // Create context with sin function
    EvalContext* ctx = exp_rs_context_new();
    
    // Reset allocation tracking before evaluation
    size_t pre_eval_count = allocation_count;
    size_t pre_eval_total = total_allocated;
    
    // Evaluate 1000 times - should have ZERO allocations
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
    
    // Assert zero allocations during evaluation
    assert(eval_allocations == 0);
    assert(eval_bytes == 0);
    
    // Clean up
    exp_rs_batch_builder_free(builder);
    exp_rs_context_free(ctx);
    exp_rs_arena_free(arena);
    
    printf("✓ Arena-based evaluation achieved zero allocations!\n");
}

void test_arena_reset() {
    printf("Testing arena reset functionality...\n");
    
    Arena* arena = exp_rs_arena_new(4096);
    assert(arena != NULL);
    
    // First batch
    BatchBuilder* builder1 = exp_rs_batch_builder_new_with_arena(arena);
    assert(exp_rs_batch_builder_add_expression(builder1, "a + b + c") == 0);
    exp_rs_batch_builder_free(builder1);
    
    // Reset arena to reclaim memory
    exp_rs_arena_reset(arena);
    
    // Second batch - reuses arena memory
    BatchBuilder* builder2 = exp_rs_batch_builder_new_with_arena(arena);
    assert(exp_rs_batch_builder_add_expression(builder2, "x * y * z") == 0);
    exp_rs_batch_builder_free(builder2);
    
    exp_rs_arena_free(arena);
    
    printf("✓ Arena reset works correctly!\n");
}

void test_arena_size_estimation() {
    printf("Testing arena size estimation...\n");
    
    const char* expressions[] = {
        "sin(x) + cos(y)",
        "a * b + c * d",
        "sqrt(x^2 + y^2)"
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