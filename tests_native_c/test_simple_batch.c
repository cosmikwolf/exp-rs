#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#include "exp_rs.h"

int main() {
    // Create context
    struct EvalContextOpaque* ctx = exp_rs_context_new();
    if (!ctx) {
        printf("Failed to create context\n");
        return 1;
    }
    
    // Create arena for zero-allocation expression evaluation
    struct ArenaOpaque* arena = exp_rs_arena_new(8192); // 8KB arena
    if (!arena) {
        printf("Failed to create arena\n");
        exp_rs_context_free(ctx);
        return 1;
    }
    printf("Arena created successfully: %p\n", arena);
    
    // Create batch builder with arena
    struct BatchBuilderOpaque* builder = exp_rs_batch_builder_new(arena);
    if (!builder) {
        printf("Failed to create batch builder with arena: %p\n", arena);
        exp_rs_arena_free(arena);
        exp_rs_context_free(ctx);
        return 1;
    }
    
    // Add simple expression
    int32_t expr_idx = exp_rs_batch_builder_add_expression(builder, "a + b");
    if (expr_idx < 0) {
        printf("Failed to add expression: %d\n", expr_idx);
        exp_rs_batch_builder_free(builder);
        exp_rs_arena_free(arena);
        exp_rs_context_free(ctx);
        return 1;
    }
    
    // Add parameters
    int32_t a_idx = exp_rs_batch_builder_add_parameter(builder, "a", 2.0);
    int32_t b_idx = exp_rs_batch_builder_add_parameter(builder, "b", 3.0);
    if (a_idx < 0 || b_idx < 0) {
        printf("Failed to add parameters: a=%d, b=%d\n", a_idx, b_idx);
        exp_rs_batch_builder_free(builder);
        exp_rs_arena_free(arena);
        exp_rs_context_free(ctx);
        return 1;
    }
    
    // Evaluate
    int32_t result = exp_rs_batch_builder_eval(builder, ctx);
    if (result != 0) {
        printf("Evaluation failed with code %d\n", result);
        exp_rs_batch_builder_free(builder);
        exp_rs_arena_free(arena);
        exp_rs_context_free(ctx);
        return 1;
    }
    
    // Get result
    double value = exp_rs_batch_builder_get_result(builder, 0);
    printf("Result: %f (expected 5.0)\n", value);
    if (fabs(value - 5.0) > 0.0001) {
        printf("ERROR: Expected 5.0 but got %f\n", value);
        exp_rs_batch_builder_free(builder);
        exp_rs_arena_free(arena);
        exp_rs_context_free(ctx);
        return 1;
    }
    
    // Update parameters and re-evaluate
    exp_rs_batch_builder_set_param(builder, a_idx, 10.0);
    exp_rs_batch_builder_set_param(builder, b_idx, 20.0);
    
    result = exp_rs_batch_builder_eval(builder, ctx);
    if (result != 0) {
        printf("Second evaluation failed with code %d\n", result);
        exp_rs_batch_builder_free(builder);
        exp_rs_arena_free(arena);
        exp_rs_context_free(ctx);
        return 1;
    }
    
    value = exp_rs_batch_builder_get_result(builder, 0);
    printf("Result: %f (expected 30.0)\n", value);
    if (fabs(value - 30.0) > 0.0001) {
        printf("ERROR: Expected 30.0 but got %f\n", value);
        exp_rs_batch_builder_free(builder);
        exp_rs_arena_free(arena);
        exp_rs_context_free(ctx);
        return 1;
    }
    
    // Cleanup
    exp_rs_batch_builder_free(builder);
    exp_rs_arena_free(arena);
    exp_rs_context_free(ctx);
    
    printf("Test passed!\n");
    return 0;
}