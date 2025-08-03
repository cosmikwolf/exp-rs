#include <stdio.h>
#include <stdlib.h>
#include "exp_rs.h"

int main() {
    printf("Testing simple evaluation...\n");
    
    // Create a context
    void* ctx = exp_rs_context_new();
    
    // Set parameters
    exp_rs_context_set_parameter(ctx, "x", 5.0);
    exp_rs_context_set_parameter(ctx, "y", 10.0);
    
    // Evaluate expression
    struct EvalResult result = exp_rs_context_eval("x + y", ctx);
    
    if (result.status == 0) {
        printf("Result: %f\n", result.value);
    } else {
        printf("Error: %s\n", result.error);
        exp_rs_free_error(result.error);
    }
    
    // Free context
    exp_rs_context_free(ctx);
    
    return 0;
}