#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#include "exp_rs.h"
#include "common_allocator.h"

int main() {
    init_memory_tracking();
    // Create context
    struct ExprContext* ctx = expr_context_new();
    if (!ctx) {
        printf("Failed to create context\n");
        return 1;
    }
    
    // Create batch with integrated arena (8KB)
    struct ExprBatch* builder = expr_batch_new(8192);
    if (!builder) {
        printf("Failed to create batch\n");
        expr_context_free(ctx);
        return 1;
    }
    printf("Batch created successfully: %p\n", builder);
    
    // Add simple expression
    ExprResult expr_res = expr_batch_add_expression(builder, "a + b");
    if (expr_res.status != 0) {
        printf("Failed to add expression: %s\n", expr_res.error);
        expr_batch_free(builder);
        expr_context_free(ctx);
        return 1;
    }
    int32_t expr_idx = expr_res.index;
    
    // Add parameters
    ExprResult a_res = expr_batch_add_variable(builder, "a", 2.0);
    ExprResult b_res = expr_batch_add_variable(builder, "b", 3.0);
    if (a_res.status != 0 || b_res.status != 0) {
        printf("Failed to add parameters: a=%s, b=%s\n", 
               a_res.status != 0 ? a_res.error : "OK",
               b_res.status != 0 ? b_res.error : "OK");
        expr_batch_free(builder);
        expr_context_free(ctx);
        return 1;
    }
    int32_t a_idx = a_res.index;
    int32_t b_idx = b_res.index;
    
    // Evaluate
    int32_t result = expr_batch_evaluate(builder, ctx);
    if (result != 0) {
        printf("Evaluation failed with code %d\n", result);
        expr_batch_free(builder);
        expr_context_free(ctx);
        return 1;
    }
    
    // Get result
    double value = expr_batch_get_result(builder, 0);
    printf("Result: %f (expected 5.0)\n", value);
    if (fabs(value - 5.0) > 0.0001) {
        printf("ERROR: Expected 5.0 but got %f\n", value);
        expr_batch_free(builder);
        expr_context_free(ctx);
        return 1;
    }
    
    // Update parameters and re-evaluate
    expr_batch_set_variable(builder, a_idx, 10.0);
    expr_batch_set_variable(builder, b_idx, 20.0);
    
    result = expr_batch_evaluate(builder, ctx);
    if (result != 0) {
        printf("Second evaluation failed with code %d\n", result);
        expr_batch_free(builder);
        expr_context_free(ctx);
        return 1;
    }
    
    value = expr_batch_get_result(builder, 0);
    printf("Result: %f (expected 30.0)\n", value);
    if (fabs(value - 30.0) > 0.0001) {
        printf("ERROR: Expected 30.0 but got %f\n", value);
        expr_batch_free(builder);
        expr_context_free(ctx);
        return 1;
    }
    
    // Cleanup - batch_free now also frees the internal arena
    expr_batch_free(builder);
    expr_context_free(ctx);
    
    printf("Test passed!\n");
    return 0;
}