#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#include "qemu_test_harness.h"
#include "exp_rs.h"

test_result_t test_batch_param_order(void) {
    qemu_printf("\n=== Testing Batch Parameter Order ===\n");
    
    // Simple expressions that reveal parameter values
    const char* expressions[] = {
        "a",      // Should return parameter a
        "b",      // Should return parameter b
        "c",      // Should return parameter c
        "a+b+c"   // Should return sum
    };
    
    const char* param_names[] = {"a", "b", "c"};
    
    // Create test data with distinct values
    // Using layout param_values[param_idx][batch_idx]
    double** param_values = (double**)malloc(3 * sizeof(double*));
    for (int p = 0; p < 3; p++) {
        param_values[p] = (double*)malloc(3 * sizeof(double));
    }
    // Set values for each parameter across batches
    for (int b = 0; b < 3; b++) {
        param_values[0][b] = 10.0 + b;  // a: 10, 11, 12
        param_values[1][b] = 20.0 + b;  // b: 20, 21, 22
        param_values[2][b] = 30.0 + b;  // c: 30, 31, 32
    }
    
    // Allocate results
    double** results = (double**)calloc(4, sizeof(double*));
    for (int i = 0; i < 4; i++) {
        results[i] = (double*)calloc(3, sizeof(double));
    }
    
    // Create context
    void* ctx = exp_rs_context_new();
    
    BatchEvalRequest request = {
        .expressions = expressions,
        .expression_count = 4,
        .param_names = param_names,
        .param_count = 3,
        .param_values = (const double**)param_values,
        .batch_size = 3,
        .results = results,
        .stop_on_error = 0,
        .statuses = NULL
    };
    
    int status = exp_rs_batch_eval(&request, ctx);
    
    qemu_printf("\nResults:\n");
    qemu_printf("Batch 0: a=%.0f, b=%.0f, c=%.0f, sum=%.0f (expected: 10, 20, 30, 60)\n",
               results[0][0], results[1][0], results[2][0], results[3][0]);
    qemu_printf("Batch 1: a=%.0f, b=%.0f, c=%.0f, sum=%.0f (expected: 11, 21, 31, 63)\n",
               results[0][1], results[1][1], results[2][1], results[3][1]);
    qemu_printf("Batch 2: a=%.0f, b=%.0f, c=%.0f, sum=%.0f (expected: 12, 22, 32, 66)\n",
               results[0][2], results[1][2], results[2][2], results[3][2]);
    
    int passed = 1;
    if (results[0][0] != 10.0 || results[0][1] != 11.0 || results[0][2] != 12.0) {
        qemu_printf("FAIL: Parameter 'a' values incorrect\n");
        passed = 0;
    }
    if (results[1][0] != 20.0 || results[1][1] != 21.0 || results[1][2] != 22.0) {
        qemu_printf("FAIL: Parameter 'b' values incorrect\n");
        passed = 0;
    }
    if (results[2][0] != 30.0 || results[2][1] != 31.0 || results[2][2] != 32.0) {
        qemu_printf("FAIL: Parameter 'c' values incorrect\n");
        passed = 0;
    }
    
    // Cleanup
    for (int i = 0; i < 4; i++) {
        free(results[i]);
    }
    free(results);
    
    for (int p = 0; p < 3; p++) {
        free(param_values[p]);
    }
    free(param_values);
    
    exp_rs_context_free(ctx);
    
    return passed ? TEST_PASS : TEST_FAIL;
}

int main(void) {
    test_result_t result = test_batch_param_order();
    qemu_exit(result == TEST_PASS ? 0 : 1);
    return 0;
}
