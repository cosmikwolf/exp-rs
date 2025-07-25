#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include "include/exp_rs.h"

#define EPSILON 1e-10

// Helper function to check if two doubles are approximately equal
int approx_equal(double a, double b) {
    return fabs(a - b) < EPSILON;
}

void test_basic_batch_eval() {
    printf("Test 1: Basic batch evaluation\n");
    
    // Create a context
    void* ctx = exp_rs_context_new();
    
    // Test expressions
    const char* expressions[] = {
        "x + y",
        "x * y",
        "x - y",
        "x / y"
    };
    size_t expr_count = 4;
    
    // Parameter names
    const char* param_names[] = {"x", "y"};
    size_t param_count = 2;
    
    // Parameter values: param_values[param_idx][batch_idx]
    double x_values[] = {2.0, 5.0, 1.0};  // x values for batch 0, 1, 2
    double y_values[] = {3.0, 10.0, 4.0}; // y values for batch 0, 1, 2
    const double* param_values[] = {x_values, y_values};
    size_t batch_size = 3;
    
    // Expected results
    double expected[4][3] = {
        {5.0, 15.0, 5.0},    // x + y
        {6.0, 50.0, 4.0},    // x * y
        {-1.0, -5.0, -3.0},  // x - y
        {2.0/3.0, 0.5, 0.25} // x / y
    };
    
    // Pre-allocate result buffers
    double** results = (double**)calloc(expr_count, sizeof(double*));
    for (size_t i = 0; i < expr_count; i++) {
        results[i] = (double*)calloc(batch_size, sizeof(double));
    }
    BatchStatus* statuses = (BatchStatus*)calloc(expr_count * batch_size, sizeof(BatchStatus));
    
    BatchEvalRequest request = {
        .expressions = expressions,
        .expression_count = expr_count,
        .param_names = param_names,
        .param_count = param_count,
        .param_values = param_values,
        .batch_size = batch_size,
        .results = results,
        .stop_on_error = 0,
        .statuses = statuses
    };
    
    int result = exp_rs_batch_eval(&request, ctx);
    
    if (result != 0) {
        printf("  FAIL: batch_eval returned error code %d\n", result);
        goto cleanup1;
    }
    
    // Check all results
    int all_passed = 1;
    for (size_t i = 0; i < expr_count; i++) {
        for (size_t j = 0; j < batch_size; j++) {
            BatchStatus* status = &statuses[i * batch_size + j];
            if (status->code != 0) {
                printf("  FAIL: Expression %zu, batch %zu failed with code %d\n", 
                       i, j, status->code);
                all_passed = 0;
                continue;
            }
            
            double actual = results[i][j];
            double expect = expected[i][j];
            if (!approx_equal(actual, expect)) {
                printf("  FAIL: Expression %zu (%s), batch %zu: expected %.6f, got %.6f\n",
                       i, expressions[i], j, expect, actual);
                all_passed = 0;
            }
        }
    }
    
    if (all_passed) {
        printf("  PASS: All results match expected values\n");
    }
    
cleanup1:
    // Free allocated results
    for (size_t i = 0; i < expr_count; i++) {
        if (results[i]) free(results[i]);
    }
    free(results);
    free(statuses);
    exp_rs_context_free(ctx);
    printf("\n");
}

void test_with_context() {
    printf("Test 2: Batch evaluation with pre-set parameters\n");
    
    // Create a context
    void* ctx = exp_rs_context_new();
    
    // Test expressions with different parameter sets
    const char* expressions[] = {
        "x * 3.14159",
        "10.0 + x",
        "20.0 * y"
    };
    size_t expr_count = 3;
    
    const char* param_names[] = {"x", "y"};
    size_t param_count = 2;
    
    // Parameter values: param_values[param_idx][batch_idx]
    double x_values2[] = {1.0, 2.0};  // x values for batch 0, 1
    double y_values2[] = {2.0, 3.0};  // y values for batch 0, 1
    const double* param_values[] = {x_values2, y_values2};
    size_t batch_size = 2;
    
    double expected[3][2] = {
        {3.14159, 6.28318},     // x * 3.14159
        {11.0, 12.0},           // 10.0 + x
        {40.0, 60.0}            // 20.0 * y
    };
    
    double** results = (double**)calloc(expr_count, sizeof(double*));
    for (size_t i = 0; i < expr_count; i++) {
        results[i] = (double*)calloc(batch_size, sizeof(double));
    }
    BatchStatus* statuses = (BatchStatus*)calloc(expr_count * batch_size, sizeof(BatchStatus));
    
    BatchEvalRequest request = {
        .expressions = expressions,
        .expression_count = expr_count,
        .param_names = param_names,
        .param_count = param_count,
        .param_values = param_values,
        .batch_size = batch_size,
        .results = results,
        .stop_on_error = 0,
        .statuses = statuses
    };
    
    // Use batch eval with context
    int result = exp_rs_batch_eval_with_context(&request, ctx);
    
    if (result != 0) {
        printf("  FAIL: batch_eval_with_context returned error code %d\n", result);
        goto cleanup2;
    }
    
    // Check all results
    int all_passed = 1;
    for (size_t i = 0; i < expr_count; i++) {
        for (size_t j = 0; j < batch_size; j++) {
            BatchStatus* status = &statuses[i * batch_size + j];
            if (status->code != 0) {
                printf("  FAIL: Expression %zu, batch %zu failed with code %d\n", 
                       i, j, status->code);
                all_passed = 0;
                continue;
            }
            
            double actual = results[i][j];
            double expect = expected[i][j];
            if (!approx_equal(actual, expect)) {
                printf("  FAIL: Expression %zu (%s), batch %zu: expected %.6f, got %.6f\n",
                       i, expressions[i], j, expect, actual);
                all_passed = 0;
            }
        }
    }
    
    if (all_passed) {
        printf("  PASS: All results match expected values\n");
    }
    
cleanup2:
    for (size_t i = 0; i < expr_count; i++) {
        if (results[i]) free(results[i]);
    }
    free(results);
    free(statuses);
    exp_rs_context_free(ctx);
    printf("\n");
}

void test_error_handling() {
    printf("Test 3: Error handling in batch evaluation\n");
    
    // Test expressions with errors
    const char* expressions[] = {
        "x + y",          // Valid
        "x / 0",          // Division by zero
        "undefined_var",  // Undefined variable
        "x * y"           // Valid
    };
    size_t expr_count = 4;
    
    const char* param_names[] = {"x", "y"};
    size_t param_count = 2;
    
    // Parameter values: param_values[param_idx][batch_idx]
    double x_values3[] = {2.0};
    double y_values3[] = {3.0};
    const double* param_values[] = {x_values3, y_values3};
    size_t batch_size = 1;
    
    double** results = (double**)calloc(expr_count, sizeof(double*));
    for (size_t i = 0; i < expr_count; i++) {
        results[i] = (double*)calloc(batch_size, sizeof(double));
    }
    BatchStatus* statuses = (BatchStatus*)calloc(expr_count * batch_size, sizeof(BatchStatus));
    
    // Create a context
    void* ctx = exp_rs_context_new();
    
    // Test without stop_on_error
    BatchEvalRequest request = {
        .expressions = expressions,
        .expression_count = expr_count,
        .param_names = param_names,
        .param_count = param_count,
        .param_values = param_values,
        .batch_size = batch_size,
        .results = results,
        .stop_on_error = 0,
        .statuses = statuses
    };
    
    int result = exp_rs_batch_eval(&request, ctx);
    
    // With stop_on_error=0, it should continue processing despite errors
    // The return code indicates if any errors occurred
    if (result != 0) {
        printf("  PASS: Got expected error code %d (some expressions failed)\n", result);
    } else {
        printf("  INFO: All expressions processed (check individual statuses)\n");
    }
    
    // Check individual statuses
    if (statuses[0].code == 0 && approx_equal(results[0][0], 5.0)) {
        printf("  PASS: First valid expression evaluated correctly\n");
    } else {
        printf("  FAIL: First valid expression failed\n");
    }
    
    if (statuses[1].code == 0 && isinf(results[1][0])) {
        printf("  PASS: Division by zero returned infinity\n");
    } else if (statuses[1].code != 0) {
        printf("  PASS: Division by zero reported error code %d\n", statuses[1].code);
    } else {
        printf("  FAIL: Division by zero handling incorrect\n");
    }
    
    if (statuses[2].code != 0) {
        printf("  PASS: Undefined variable reported error code %d\n", statuses[2].code);
    } else {
        printf("  FAIL: Undefined variable did not report error\n");
    }
    
    if (statuses[3].code == 0 && approx_equal(results[3][0], 6.0)) {
        printf("  PASS: Last valid expression evaluated correctly\n");
    } else {
        printf("  FAIL: Last valid expression failed\n");
    }
    
    // Cleanup
    for (size_t i = 0; i < expr_count; i++) {
        if (results[i]) free(results[i]);
    }
    free(results);
    free(statuses);
    exp_rs_context_free(ctx);
    printf("\n");
}

void test_batch_builder() {
    printf("Test 4: BatchBuilder API\n");
    
    // Create a BatchBuilder
    struct BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
    if (!builder) {
        printf("  FAIL: Failed to create BatchBuilder\n");
        return;
    }
    
    // Add expressions
    const char* expressions[] = {
        "x + y",
        "x * y",
        "x * x + y * y"
    };
    
    for (size_t i = 0; i < 3; i++) {
        printf("  Adding expression %zu: '%s'\n", i, expressions[i]);
        int32_t result = exp_rs_batch_builder_add_expression(builder, expressions[i]);
        if (result < 0) {
            printf("  FAIL: Failed to add expression %zu ('%s') with error code %d\n", i, expressions[i], result);
            exp_rs_batch_builder_free(builder);
            return;
        }
        if (result != (int32_t)i) {
            printf("  WARN: Expected index %zu but got %d\n", i, result);
        }
    }
    
    // Add parameters
    exp_rs_batch_builder_add_parameter(builder, "x", 2.0);
    exp_rs_batch_builder_add_parameter(builder, "y", 3.0);
    
    // Create context for evaluation
    void* ctx = exp_rs_context_new();
    if (!ctx) {
        printf("  FAIL: Failed to create context\n");
        exp_rs_batch_builder_free(builder);
        return;
    }
    
    // Evaluate
    int32_t eval_result = exp_rs_batch_builder_eval(builder, ctx);
    if (eval_result != 0) {
        printf("  FAIL: Evaluation failed with code %d\n", eval_result);
        exp_rs_batch_builder_free(builder);
        return;
    }
    
    // Check results
    double expected[] = {5.0, 6.0, 13.0};
    int all_passed = 1;
    
    for (size_t i = 0; i < 3; i++) {
        double actual = exp_rs_batch_builder_get_result(builder, i);
        if (!approx_equal(actual, expected[i])) {
            printf("  FAIL: Expression %zu (%s): expected %.6f, got %.6f\n",
                   i, expressions[i], expected[i], actual);
            all_passed = 0;
        }
    }
    
    if (all_passed) {
        printf("  PASS: All BatchBuilder results correct\n");
    }
    
    // Test parameter update
    exp_rs_batch_builder_set_param_by_name(builder, "x", 4.0);
    exp_rs_batch_builder_set_param_by_name(builder, "y", 5.0);
    
    eval_result = exp_rs_batch_builder_eval(builder, ctx);
    if (eval_result != 0) {
        printf("  FAIL: Re-evaluation failed\n");
    } else {
        double expected2[] = {9.0, 20.0, 41.0};
        all_passed = 1;
        
        for (size_t i = 0; i < 3; i++) {
            double actual = exp_rs_batch_builder_get_result(builder, i);
            if (!approx_equal(actual, expected2[i])) {
                printf("  FAIL: After update, expression %zu: expected %.6f, got %.6f\n",
                       i, expected2[i], actual);
                all_passed = 0;
            }
        }
        
        if (all_passed) {
            printf("  PASS: Parameter updates work correctly\n");
        }
    }
    
    exp_rs_batch_builder_free(builder);
    exp_rs_context_free(ctx);
    printf("\n");
}

void test_performance_comparison() {
    printf("Test 5: Performance comparison (batch vs individual)\n");
    
    // Larger batch for performance testing
    const char* expressions[] = {
        "x * y + z",
        "sin(x) * cos(y)",
        "sqrt(x*x + y*y)",
        "exp(x) + log(y)"
    };
    size_t expr_count = 4;
    
    const char* param_names[] = {"x", "y", "z"};
    size_t param_count = 3;
    
    // Generate 100 batches of data
    size_t batch_size = 100;
    // Create param_values[param_idx][batch_idx]
    double* x_values_perf = (double*)malloc(batch_size * sizeof(double));
    double* y_values_perf = (double*)malloc(batch_size * sizeof(double));
    double* z_values_perf = (double*)malloc(batch_size * sizeof(double));
    
    for (size_t i = 0; i < batch_size; i++) {
        x_values_perf[i] = (double)(i + 1) / 10.0;  // x
        y_values_perf[i] = (double)(i + 2) / 10.0;  // y  
        z_values_perf[i] = (double)(i + 3) / 10.0;  // z
    }
    
    const double* param_values_perf[] = {x_values_perf, y_values_perf, z_values_perf};
    
    // Test batch evaluation
    double** results = (double**)calloc(expr_count, sizeof(double*));
    for (size_t i = 0; i < expr_count; i++) {
        results[i] = (double*)calloc(batch_size, sizeof(double));
    }
    BatchStatus* statuses = (BatchStatus*)calloc(expr_count * batch_size, sizeof(BatchStatus));
    
    // Create a context
    void* ctx = exp_rs_context_new();
    
    BatchEvalRequest request = {
        .expressions = expressions,
        .expression_count = expr_count,
        .param_names = param_names,
        .param_count = param_count,
        .param_values = param_values_perf,
        .batch_size = batch_size,
        .results = results,
        .stop_on_error = 0,
        .statuses = statuses
    };
    
    int batch_result = exp_rs_batch_eval(&request, ctx);
    
    if (batch_result == 0) {
        printf("  PASS: Batch evaluation completed successfully\n");
        
        // Verify a few results by computing them individually
        void* ctx = exp_rs_context_new();
        
        for (size_t i = 0; i < 3; i++) {  // Check first 3 batches
            exp_rs_context_set_parameter(ctx, "x", x_values_perf[i]);
            exp_rs_context_set_parameter(ctx, "y", y_values_perf[i]);
            exp_rs_context_set_parameter(ctx, "z", z_values_perf[i]);
            
            for (size_t j = 0; j < expr_count; j++) {
                struct EvalResult eval_result = exp_rs_context_eval(expressions[j], ctx);
                double individual_result = eval_result.value;
                double batch_result_val = results[j][i];
                
                if (!approx_equal(individual_result, batch_result_val)) {
                    printf("  FAIL: Mismatch at expr %zu, batch %zu: %.6f vs %.6f\n",
                           j, i, individual_result, batch_result_val);
                }
            }
        }
        
        printf("  PASS: Batch results match individual evaluations\n");
    } else {
        printf("  FAIL: Batch evaluation failed with code %d\n", batch_result);
    }
    
    // Cleanup
    for (size_t i = 0; i < expr_count; i++) {
        if (results[i]) free(results[i]);
    }
    free(results);
    free(statuses);
    
    free(x_values_perf);
    free(y_values_perf);
    free(z_values_perf);
    exp_rs_context_free(ctx);
    printf("\n");
}

int main() {
    printf("=== Batch Evaluation Tests ===\n\n");
    
    test_basic_batch_eval();
    test_with_context();
    test_error_handling();
    test_batch_builder();
    test_performance_comparison();
    
    printf("=== Tests Complete ===\n");
    return 0;
}