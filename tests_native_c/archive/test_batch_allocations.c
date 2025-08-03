#include <stdio.h>
#include <stdlib.h>
#include <dlfcn.h>
#include <math.h>
#include "exp_rs.h"

// Memory tracking
static size_t g_allocated_before = 0;
static size_t g_allocated_after = 0;
static void* (*real_malloc)(size_t) = NULL;
static void (*real_free)(void*) = NULL;

// Simple allocation tracking
static size_t g_total_allocated = 0;

void* malloc(size_t size) {
    if (!real_malloc) {
        real_malloc = dlsym(RTLD_NEXT, "malloc");
    }
    void* ptr = real_malloc(size);
    if (ptr) g_total_allocated += size;
    return ptr;
}

void free(void* ptr) {
    if (!real_free) {
        real_free = dlsym(RTLD_NEXT, "free");
    }
    real_free(ptr);
}

// Native functions
Real native_sin(const Real* args, uintptr_t n_args) {
    return sin(args[0]);
}

int main() {
    // Initialize dlsym
    void* dummy = malloc(1);
    free(dummy);
    
    printf("=== Batch Allocation Test ===\n\n");
    
    // Create context
    g_allocated_before = g_total_allocated;
    EvalContextOpaque* ctx = exp_rs_context_new();
    printf("Context creation: %zu bytes\n", g_total_allocated - g_allocated_before);
    
    // Register function
    g_allocated_before = g_total_allocated;
    EvalResult reg = exp_rs_context_register_native_function(ctx, "sin", 1, native_sin);
    if (reg.status != 0) {
        printf("Failed to register sin\n");
        return 1;
    }
    printf("Function registration: %zu bytes\n", g_total_allocated - g_allocated_before);
    
    // Create batch builder
    g_allocated_before = g_total_allocated;
    BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
    printf("Batch builder creation: %zu bytes\n", g_total_allocated - g_allocated_before);
    
    // Add expression
    g_allocated_before = g_total_allocated;
    exp_rs_batch_builder_add_expression(builder, "sin(x) + y");
    printf("Add expression: %zu bytes\n", g_total_allocated - g_allocated_before);
    
    // Add parameters
    g_allocated_before = g_total_allocated;
    exp_rs_batch_builder_add_parameter(builder, "x", 0.0);
    exp_rs_batch_builder_add_parameter(builder, "y", 0.0);
    printf("Add parameters: %zu bytes\n", g_total_allocated - g_allocated_before);
    
    printf("\n=== Setup complete, testing runtime ===\n");
    size_t setup_total = g_total_allocated;
    
    // Test 100 evaluations
    for (int i = 0; i < 100; i++) {
        // Update parameters
        g_allocated_before = g_total_allocated;
        exp_rs_batch_builder_set_param(builder, 0, i * 0.1);
        exp_rs_batch_builder_set_param(builder, 1, i * 0.2);
        size_t param_alloc = g_total_allocated - g_allocated_before;
        
        // Evaluate
        g_allocated_before = g_total_allocated;
        int32_t result = exp_rs_batch_builder_eval(builder, ctx);
        size_t eval_alloc = g_total_allocated - g_allocated_before;
        
        // Get result
        g_allocated_before = g_total_allocated;
        Real value = exp_rs_batch_builder_get_result(builder, 0);
        size_t get_alloc = g_total_allocated - g_allocated_before;
        
        if (i < 5 || i == 99) {
            printf("Iteration %d: param_set=%zu, eval=%zu, get=%zu bytes (result=%f)\n", 
                   i, param_alloc, eval_alloc, get_alloc, value);
        }
    }
    
    printf("\nTotal runtime allocations: %zu bytes\n", g_total_allocated - setup_total);
    
    // Cleanup
    exp_rs_batch_builder_free(builder);
    exp_rs_context_free(ctx);
    
    return 0;
}