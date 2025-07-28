#include <stdio.h>
#include <stdlib.h>
#include <dlfcn.h>
#include <math.h>
#include "../include/exp_rs.h"

// Memory tracking
static void* (*real_malloc)(size_t) = NULL;
static size_t g_total_allocated = 0;

void* malloc(size_t size) {
    if (!real_malloc) {
        real_malloc = dlsym(RTLD_NEXT, "malloc");
    }
    void* ptr = real_malloc(size);
    if (ptr) g_total_allocated += size;
    return ptr;
}

// Native function
Real native_sin(const Real* args, uintptr_t n_args) {
    return sin(args[0]);
}

int main() {
    // Initialize
    void* dummy = malloc(1);
    free(dummy);
    
    printf("=== Direct Batch FFI Allocation Test ===\n\n");
    
    // Create context
    EvalContextOpaque* ctx = exp_rs_context_new();
    EvalResult reg = exp_rs_context_register_native_function(ctx, "sin", 1, native_sin);
    if (reg.status != 0) return 1;
    
    // Setup batch request
    const char* expressions[] = {"sin(x) + y"};
    const char* param_names[] = {"x", "y"};
    Real param_values[2][100]; // 2 params, 100 iterations
    Real results[1][100];      // 1 expression, 100 results
    
    // Fill parameter values
    for (int i = 0; i < 100; i++) {
        param_values[0][i] = i * 0.1;  // x
        param_values[1][i] = i * 0.2;  // y
    }
    
    Real* param_ptrs[] = {param_values[0], param_values[1]};
    Real* result_ptrs[] = {results[0]};
    
    BatchEvalRequest request = {
        .expressions = expressions,
        .expression_count = 1,
        .param_names = param_names,
        .param_count = 2,
        .param_values = (const Real* const*)param_ptrs,
        .batch_size = 100,
        .results = result_ptrs,
        .stop_on_error = false,
        .statuses = NULL
    };
    
    printf("Setup complete\n");
    size_t before = g_total_allocated;
    
    // Evaluate batch
    int32_t result = exp_rs_batch_eval_with_context(&request, ctx);
    
    size_t allocated = g_total_allocated - before;
    printf("\nBatch evaluation of 100 items: %zu bytes (%zu per item)\n", 
           allocated, allocated / 100);
    
    if (result == 0) {
        printf("First result: %f\n", results[0][0]);
        printf("Last result: %f\n", results[0][99]);
    } else {
        printf("Evaluation failed: %d\n", result);
    }
    
    exp_rs_context_free(ctx);
    return 0;
}