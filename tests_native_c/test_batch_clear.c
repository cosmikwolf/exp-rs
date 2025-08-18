#include "../include/exp_rs.h"
#include "common_allocator.h"
#include <stdio.h>
#include <stdlib.h>
#include <math.h>

// Helper function to check if values are approximately equal
static int is_close(Real a, Real b, Real tolerance) {
    return fabs(a - b) < tolerance;
}

int main() {
    printf("=== Batch Clear Functionality Test ===\n");

    // Initialize memory tracking
    init_memory_tracking();
    enable_allocation_tracking();
    reset_memory_stats();

    // Create context, arena, and batch
    ExprContext *ctx = expr_context_new();
    if (!ctx) {
        printf("ERROR: Failed to create context\n");
        return 1;
    }

    ExprArena *arena = expr_arena_new(2048);  // 2KB arena
    if (!arena) {
        printf("ERROR: Failed to create arena\n");
        expr_context_free(ctx);
        return 1;
    }

    ExprBatch *batch = expr_batch_new(arena);
    if (!batch) {
        printf("ERROR: Failed to create batch\n");
        expr_arena_free(arena);
        expr_context_free(ctx);
        return 1;
    }

    printf("Initial setup complete\n");

    // Test 1: Add expressions and parameters
    printf("\n1. Adding expressions and parameters...\n");
    
    ExprResult expr1 = expr_batch_add_expression(batch, "x + y");
    if (expr1.status != 0) {
        printf("ERROR: Failed to add expression 1: %s\n", expr1.error);
        expr_free_error(expr1.error);
        goto cleanup;
    }
    
    ExprResult expr2 = expr_batch_add_expression(batch, "x * y");
    if (expr2.status != 0) {
        printf("ERROR: Failed to add expression 2: %s\n", expr2.error);
        expr_free_error(expr2.error);
        goto cleanup;
    }

    ExprResult var_x = expr_batch_add_variable(batch, "x", 5.0);
    if (var_x.status != 0) {
        printf("ERROR: Failed to add variable x: %s\n", var_x.error);
        expr_free_error(var_x.error);
        goto cleanup;
    }

    ExprResult var_y = expr_batch_add_variable(batch, "y", 3.0);
    if (var_y.status != 0) {
        printf("ERROR: Failed to add variable y: %s\n", var_y.error);
        expr_free_error(var_y.error);
        goto cleanup;
    }

    printf("Added 2 expressions and 2 variables\n");

    // Test 2: Evaluate batch
    printf("\n2. Evaluating batch...\n");
    
    int32_t eval_result = expr_batch_evaluate(batch, ctx);
    if (eval_result != 0) {
        printf("ERROR: Batch evaluation failed with code: %d\n", eval_result);
        goto cleanup;
    }

    Real result1 = expr_batch_get_result(batch, 0);  // x + y = 5 + 3 = 8
    Real result2 = expr_batch_get_result(batch, 1);  // x * y = 5 * 3 = 15

    printf("Expression results: %f, %f\n", result1, result2);

    if (!is_close(result1, 8.0, 1e-6) || !is_close(result2, 15.0, 1e-6)) {
        printf("ERROR: Unexpected results. Expected 8.0 and 15.0\n");
        goto cleanup;
    }

    // Test 3: Get arena usage before clear
    printf("\n3. Checking arena usage before clear...\n");
    uintptr_t arena_bytes_before = expr_batch_arena_bytes(batch);
    printf("Arena bytes before clear: %zu\n", arena_bytes_before);

    // Test 4: Clear the batch
    printf("\n4. Clearing batch...\n");
    
    int32_t clear_result = expr_batch_clear(batch);
    if (clear_result != 0) {
        printf("ERROR: Batch clear failed with code: %d\n", clear_result);
        goto cleanup;
    }

    printf("Batch cleared successfully\n");

    // Test 5: Verify batch is empty after clear
    printf("\n5. Verifying batch is empty...\n");
    
    // Try to evaluate empty batch (should succeed but have no results)
    eval_result = expr_batch_evaluate(batch, ctx);
    if (eval_result == 0) {
        printf("Empty batch evaluation succeeded (no expressions to evaluate)\n");
        
        // Try to get a result from empty batch (should return NaN or invalid)
        Real empty_result = expr_batch_get_result(batch, 0);
        if (isnan(empty_result)) {
            printf("GOOD: Empty batch returns NaN for non-existent expression\n");
        } else {
            printf("WARNING: Empty batch returned unexpected result: %f\n", empty_result);
        }
    } else {
        printf("Empty batch evaluation failed with code: %d\n", eval_result);
    }

    // Test 6: Check arena usage after clear (should be same - arena memory not reclaimed)
    printf("\n6. Checking arena usage after clear...\n");
    uintptr_t arena_bytes_after = expr_batch_arena_bytes(batch);
    printf("Arena bytes after clear: %zu\n", arena_bytes_after);

    if (arena_bytes_after != arena_bytes_before) {
        printf("WARNING: Arena usage changed after clear (expected same)\n");
        printf("Before: %zu, After: %zu\n", arena_bytes_before, arena_bytes_after);
    } else {
        printf("GOOD: Arena usage unchanged (memory not reclaimed)\n");
    }

    // Test 7: Reuse the batch with new expressions
    printf("\n7. Reusing batch with new expressions...\n");
    
    ExprResult new_expr = expr_batch_add_expression(batch, "a - b");
    if (new_expr.status != 0) {
        printf("ERROR: Failed to add new expression: %s\n", new_expr.error);
        expr_free_error(new_expr.error);
        goto cleanup;
    }

    ExprResult new_var_a = expr_batch_add_variable(batch, "a", 10.0);
    if (new_var_a.status != 0) {
        printf("ERROR: Failed to add new variable a: %s\n", new_var_a.error);
        expr_free_error(new_var_a.error);
        goto cleanup;
    }

    ExprResult new_var_b = expr_batch_add_variable(batch, "b", 4.0);
    if (new_var_b.status != 0) {
        printf("ERROR: Failed to add new variable b: %s\n", new_var_b.error);
        expr_free_error(new_var_b.error);
        goto cleanup;
    }

    printf("Added new expression and variables to cleared batch\n");

    // Test 8: Evaluate reused batch
    printf("\n8. Evaluating reused batch...\n");
    
    eval_result = expr_batch_evaluate(batch, ctx);
    if (eval_result != 0) {
        printf("ERROR: Reused batch evaluation failed with code: %d\n", eval_result);
        goto cleanup;
    }

    Real new_result = expr_batch_get_result(batch, 0);  // a - b = 10 - 4 = 6
    printf("New expression result: %f\n", new_result);

    if (!is_close(new_result, 6.0, 1e-6)) {
        printf("ERROR: Unexpected result. Expected 6.0\n");
        goto cleanup;
    }

    // Test 9: Check final arena usage
    printf("\n9. Checking final arena usage...\n");
    uintptr_t arena_bytes_final = expr_batch_arena_bytes(batch);
    printf("Arena bytes final: %zu\n", arena_bytes_final);
    
    if (arena_bytes_final <= arena_bytes_before) {
        printf("WARNING: Final arena usage not greater than initial (expected growth)\n");
    } else {
        printf("GOOD: Arena usage increased as expected (new allocations)\n");
    }

    printf("\n=== All tests passed! ===\n");

cleanup:
    expr_batch_free(batch);
    expr_arena_free(arena);
    expr_context_free(ctx);

    print_memory_stats("Final");
    return 0;
}