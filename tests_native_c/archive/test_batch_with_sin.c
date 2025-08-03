#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#include "exp_rs.h"

// Native sin function wrapper
Real native_sin(const Real* args, uintptr_t n_args) {
    return sin(args[0]);
}

int main() {
    // Create context
    EvalContextOpaque* ctx = exp_rs_context_new();
    if (!ctx) {
        printf("Failed to create context\n");
        return 1;
    }
    
    // Register sin function
    EvalResult reg_result = exp_rs_context_register_native_function(ctx, "sin", 1, native_sin);
    if (reg_result.status != 0) {
        printf("Failed to register sin function: %s\n", reg_result.error);
        exp_rs_free_error((char*)reg_result.error);
        exp_rs_context_free(ctx);
        return 1;
    }
    
    // Create batch builder
    BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
    if (!builder) {
        printf("Failed to create batch builder\n");
        exp_rs_context_free(ctx);
        return 1;
    }
    
    // Add expression using sin
    int32_t expr_idx = exp_rs_batch_builder_add_expression(builder, "sin(x)");
    if (expr_idx < 0) {
        printf("Failed to add expression: %d\n", expr_idx);
        exp_rs_batch_builder_free(builder);
        exp_rs_context_free(ctx);
        return 1;
    }
    
    // Add parameter
    int32_t x_idx = exp_rs_batch_builder_add_parameter(builder, "x", 1.5708); // PI/2
    if (x_idx < 0) {
        printf("Failed to add parameter: %d\n", x_idx);
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
    printf("sin(PI/2) = %f (expected ~1.0)\n", value);
    
    // Cleanup
    exp_rs_batch_builder_free(builder);
    exp_rs_context_free(ctx);
    
    printf("Test passed!\n");
    return 0;
}