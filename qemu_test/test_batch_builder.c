#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#include "qemu_test_harness.h"
#include "../include/exp_rs.h"
#include "register_test_functions.h"

test_result_t test_batch_builder_basic(void) {
    qemu_printf("\n=== Testing BatchBuilder API ===\n");
    
    // Create context and builder
    void* ctx = create_test_context();
    if (!ctx) {
        qemu_printf("FAIL: Failed to create context\n");
        return TEST_FAIL;
    }
    
    // Create arena for zero-allocation expression evaluation
    void* arena = exp_rs_arena_new(8192); // 8KB arena for embedded test
    if (!arena) {
        qemu_printf("FAIL: Failed to create arena\n");
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    void* builder = exp_rs_batch_builder_new(arena);
    if (!builder) {
        qemu_printf("FAIL: Failed to create batch builder\n");
        exp_rs_arena_free(arena);
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Add parameters
    int param_a = exp_rs_batch_builder_add_parameter(builder, "a", 0.0);
    int param_b = exp_rs_batch_builder_add_parameter(builder, "b", 0.0);
    int param_c = exp_rs_batch_builder_add_parameter(builder, "c", 0.0);
    
    if (param_a < 0 || param_b < 0 || param_c < 0) {
        qemu_printf("FAIL: Failed to add parameters\n");
        exp_rs_batch_builder_free(builder);
        exp_rs_arena_free(arena);
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Add expressions
    int expr1 = exp_rs_batch_builder_add_expression(builder, "a + b");
    int expr2 = exp_rs_batch_builder_add_expression(builder, "a * b");
    int expr3 = exp_rs_batch_builder_add_expression(builder, "sqrt(a*a + b*b)");
    int expr4 = exp_rs_batch_builder_add_expression(builder, "a + b + c");
    
    if (expr1 < 0 || expr2 < 0 || expr3 < 0 || expr4 < 0) {
        qemu_printf("FAIL: Failed to add expressions\n");
        exp_rs_batch_builder_free(builder);
        exp_rs_arena_free(arena);
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Test 1: Basic evaluation
    qemu_printf("\nTest 1: Basic evaluation\n");
    exp_rs_batch_builder_set_param(builder, param_a, 3.0);
    exp_rs_batch_builder_set_param(builder, param_b, 4.0);
    exp_rs_batch_builder_set_param(builder, param_c, 5.0);
    
    if (exp_rs_batch_builder_eval(builder, ctx) != 0) {
        qemu_printf("FAIL: Evaluation failed\n");
        exp_rs_batch_builder_free(builder);
        exp_rs_arena_free(arena);
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    double result1 = exp_rs_batch_builder_get_result(builder, expr1);
    double result2 = exp_rs_batch_builder_get_result(builder, expr2);
    double result3 = exp_rs_batch_builder_get_result(builder, expr3);
    double result4 = exp_rs_batch_builder_get_result(builder, expr4);
    
    qemu_printf("  a + b = %.2f (expected 7.0)\n", result1);
    qemu_printf("  a * b = %.2f (expected 12.0)\n", result2);
    qemu_printf("  sqrt(a²+b²) = %.2f (expected 5.0)\n", result3);
    qemu_printf("  a + b + c = %.2f (expected 12.0)\n", result4);
    
    if (fabs(result1 - 7.0) > 0.001 || fabs(result2 - 12.0) > 0.001 ||
        fabs(result3 - 5.0) > 0.001 || fabs(result4 - 12.0) > 0.001) {
        qemu_printf("FAIL: Results don't match expected values\n");
        exp_rs_batch_builder_free(builder);
        exp_rs_arena_free(arena);
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Test 2: Parameter update and re-evaluation
    qemu_printf("\nTest 2: Parameter update and re-evaluation\n");
    exp_rs_batch_builder_set_param(builder, param_a, 5.0);
    exp_rs_batch_builder_set_param(builder, param_b, 12.0);
    exp_rs_batch_builder_set_param(builder, param_c, 0.0);
    
    if (exp_rs_batch_builder_eval(builder, ctx) != 0) {
        qemu_printf("FAIL: Re-evaluation failed\n");
        exp_rs_batch_builder_free(builder);
        exp_rs_arena_free(arena);
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    result1 = exp_rs_batch_builder_get_result(builder, expr1);
    result2 = exp_rs_batch_builder_get_result(builder, expr2);
    result3 = exp_rs_batch_builder_get_result(builder, expr3);
    result4 = exp_rs_batch_builder_get_result(builder, expr4);
    
    qemu_printf("  a + b = %.2f (expected 17.0)\n", result1);
    qemu_printf("  a * b = %.2f (expected 60.0)\n", result2);
    qemu_printf("  sqrt(a²+b²) = %.2f (expected 13.0)\n", result3);
    qemu_printf("  a + b + c = %.2f (expected 17.0)\n", result4);
    
    if (fabs(result1 - 17.0) > 0.001 || fabs(result2 - 60.0) > 0.001 ||
        fabs(result3 - 13.0) > 0.001 || fabs(result4 - 17.0) > 0.001) {
        qemu_printf("FAIL: Updated results don't match expected values\n");
        exp_rs_batch_builder_free(builder);
        exp_rs_arena_free(arena);
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Test 3: Performance test - simulate 1000Hz updates
    qemu_printf("\nTest 3: Performance test (1000 iterations)\n");
    
    init_hardware_timer();
    benchmark_start();
    
    for (int i = 0; i < 1000; i++) {
        // Simulate sensor data updates
        double sensor_a = 1.0 + (i % 10) * 0.1;
        double sensor_b = 2.0 + (i % 5) * 0.2;
        double sensor_c = 3.0 + (i % 7) * 0.3;
        
        exp_rs_batch_builder_set_param(builder, param_a, sensor_a);
        exp_rs_batch_builder_set_param(builder, param_b, sensor_b);
        exp_rs_batch_builder_set_param(builder, param_c, sensor_c);
        
        if (exp_rs_batch_builder_eval(builder, ctx) != 0) {
            qemu_printf("FAIL: Evaluation failed at iteration %d\n", i);
            exp_rs_batch_builder_free(builder);
            exp_rs_context_free(ctx);
            return TEST_FAIL;
        }
        
        // In real use, you would use the results here
        // For test, just verify we can get them
        exp_rs_batch_builder_get_result(builder, expr1);
        exp_rs_batch_builder_get_result(builder, expr2);
        exp_rs_batch_builder_get_result(builder, expr3);
        exp_rs_batch_builder_get_result(builder, expr4);
    }
    
    uint32_t ticks = benchmark_stop();
    qemu_printf("  1000 iterations completed in %u ticks\n", ticks);
    qemu_printf("  Average per iteration: %u ticks\n", ticks / 1000);
    
    // Verify counts
    size_t param_count = exp_rs_batch_builder_param_count(builder);
    size_t expr_count = exp_rs_batch_builder_expression_count(builder);
    
    qemu_printf("\nBuilder stats:\n");
    qemu_printf("  Parameters: %u (expected 3)\n", (unsigned)param_count);
    qemu_printf("  Expressions: %u (expected 4)\n", (unsigned)expr_count);
    
    if (param_count != 3 || expr_count != 4) {
        qemu_printf("FAIL: Incorrect counts\n");
        exp_rs_batch_builder_free(builder);
        exp_rs_arena_free(arena);
        exp_rs_context_free(ctx);
        return TEST_FAIL;
    }
    
    // Clean up
    exp_rs_batch_builder_free(builder);
    exp_rs_arena_free(arena);
    exp_rs_context_free(ctx);
    
    qemu_printf("\nAll tests PASSED!\n");
    return TEST_PASS;
}

int main(void) {
    test_result_t result = test_batch_builder_basic();
    qemu_exit(result == TEST_PASS ? 0 : 1);
    return 0;
}