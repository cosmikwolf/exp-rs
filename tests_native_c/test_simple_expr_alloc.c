#include <stdio.h>
#include <stdlib.h>
#include <dlfcn.h>
#include "../include/exp_rs.h"

// Memory tracking
static void* (*real_malloc)(size_t) = NULL;
static size_t g_total_allocated = 0;

void* malloc(size_t size) {
    if (!real_malloc) real_malloc = dlsym(RTLD_NEXT, "malloc");
    void* ptr = real_malloc(size);
    if (ptr) g_total_allocated += size;
    return ptr;
}

int main() {
    void* dummy = malloc(1);
    free(dummy);
    
    printf("=== AST Complexity vs Allocation Test ===\n\n");
    
    EvalContextOpaque* ctx = exp_rs_context_new();
    
    // Test different expression complexities
    const char* expressions[] = {
        "x",                    // Simple variable
        "x + y",                // Binary op
        "x + y + z",            // Two binary ops
        "x + y * z - w",        // Multiple ops
    };
    
    for (int i = 0; i < 4; i++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        exp_rs_batch_builder_add_expression(builder, expressions[i]);
        exp_rs_batch_builder_add_parameter(builder, "x", 1.0);
        exp_rs_batch_builder_add_parameter(builder, "y", 2.0);
        exp_rs_batch_builder_add_parameter(builder, "z", 3.0);
        exp_rs_batch_builder_add_parameter(builder, "w", 4.0);
        
        size_t before = g_total_allocated;
        
        // Evaluate 10 times
        for (int j = 0; j < 10; j++) {
            exp_rs_batch_builder_eval(builder, ctx);
        }
        
        size_t allocated = g_total_allocated - before;
        printf("Expression '%s': %zu bytes per eval\n", 
               expressions[i], allocated / 10);
        
        exp_rs_batch_builder_free(builder);
    }
    
    exp_rs_context_free(ctx);
    return 0;
}