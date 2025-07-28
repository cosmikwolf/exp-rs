#include <stdio.h>
#include <stdlib.h>
#include "../include/exp_rs.h"

int main() {
    // Create context
    EvalContextOpaque* ctx = exp_rs_context_new();
    if (!ctx) {
        printf("Failed to create context\n");
        return 1;
    }
    
    // Create batch builder
    BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
    if (!builder) {
        printf("Failed to create batch builder\n");
        exp_rs_context_free(ctx);
        return 1;
    }
    
    // Add simple expression
    int32_t expr_idx = exp_rs_batch_builder_add_expression(builder, "a + b");
    if (expr_idx < 0) {
        printf("Failed to add expression: %d\n", expr_idx);
        exp_rs_batch_builder_free(builder);
        exp_rs_context_free(ctx);
        return 1;
    }
    
    // Add parameters
    int32_t a_idx = exp_rs_batch_builder_add_parameter(builder, "a", 2.0);
    int32_t b_idx = exp_rs_batch_builder_add_parameter(builder, "b", 3.0);
    if (a_idx < 0 || b_idx < 0) {
        printf("Failed to add parameters: a=%d, b=%d\n", a_idx, b_idx);
        exp_rs_batch_builder_free(builder);
        exp_rs_context_free(ctx);
        return 1;
    }
    
    // Evaluate
    int32_t result = exp_rs_batch_builder_eval(builder, ctx);
    if (result != 0) {
        printf("Evaluation failed with code %d\n", result);
        exp_rs_batch_builder_free(builder);
        exp_rs_context_free(ctx);
        return 1;
    }
    
    // Get result
    Real value = exp_rs_batch_builder_get_result(builder, 0);
    printf("Result: %f (expected 5.0)\n", value);
    
    // Update parameters and re-evaluate
    exp_rs_batch_builder_set_param(builder, a_idx, 10.0);
    exp_rs_batch_builder_set_param(builder, b_idx, 20.0);
    
    result = exp_rs_batch_builder_eval(builder, ctx);
    if (result != 0) {
        printf("Second evaluation failed with code %d\n", result);
        exp_rs_batch_builder_free(builder);
        exp_rs_context_free(ctx);
        return 1;
    }
    
    value = exp_rs_batch_builder_get_result(builder, 0);
    printf("Result: %f (expected 30.0)\n", value);
    
    // Cleanup
    exp_rs_batch_builder_free(builder);
    exp_rs_context_free(ctx);
    
    printf("Test passed!\n");
    return 0;
}