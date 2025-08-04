#include <stdio.h>
#include <math.h>
#include "../include/exp_rs.h"

int main() {
    printf("=== Expression Function Test ===\n\n");
    
    // Create context
    ExprContext* ctx = expr_context_new();
    if (!ctx) {
        printf("Failed to create context\n");
        return 1;
    }
    
    // Create arena and batch for testing
    ExprArena* arena = expr_arena_new(8192);
    ExprBatch* batch = expr_batch_new(arena);
    
    // Test 1: Add expression functions
    printf("1. Adding expression functions:\n");
    
    int result = expr_context_add_expression_function(ctx, "distance", "x1,y1,x2,y2", 
                                                      "sqrt((x2-x1)^2 + (y2-y1)^2)");
    printf("   - Added 'distance' function: %s\n", result == 0 ? "success" : "failed");
    
    result = expr_context_add_expression_function(ctx, "avg", "a,b", "(a+b)/2");
    printf("   - Added 'avg' function: %s\n", result == 0 ? "success" : "failed");
    
    result = expr_context_add_expression_function(ctx, "square", "x", "x*x");
    printf("   - Added 'square' function: %s\n", result == 0 ? "success" : "failed");
    
    // Test 2: Use expression functions
    printf("\n2. Using expression functions:\n");
    
    // Test distance function
    expr_batch_add_expression(batch, "distance(0, 0, 3, 4)");
    expr_batch_evaluate(batch, ctx);
    Real dist = expr_batch_get_result(batch, 0);
    printf("   - distance(0, 0, 3, 4) = %.1f (expected 5.0)\n", dist);
    
    // Test avg function
    expr_batch_add_expression(batch, "avg(10, 20)");
    expr_batch_evaluate(batch, ctx);
    Real average = expr_batch_get_result(batch, 1);
    printf("   - avg(10, 20) = %.1f (expected 15.0)\n", average);
    
    // Test square function
    expr_batch_add_expression(batch, "square(7)");
    expr_batch_evaluate(batch, ctx);
    Real sq = expr_batch_get_result(batch, 2);
    printf("   - square(7) = %.1f (expected 49.0)\n", sq);
    
    // Test 3: Expression functions can call each other
    printf("\n3. Nested expression functions:\n");
    
    result = expr_context_add_expression_function(ctx, "dist_squared", "x1,y1,x2,y2", 
                                                  "square(distance(x1,y1,x2,y2))");
    printf("   - Added 'dist_squared' function: %s\n", result == 0 ? "success" : "failed");
    
    expr_batch_add_expression(batch, "dist_squared(0, 0, 3, 4)");
    expr_batch_evaluate(batch, ctx);
    Real dist_sq = expr_batch_get_result(batch, 3);
    printf("   - dist_squared(0, 0, 3, 4) = %.1f (expected 25.0)\n", dist_sq);
    
    // Test 4: Remove expression functions
    printf("\n4. Removing expression functions:\n");
    
    result = expr_context_remove_expression_function(ctx, "avg");
    printf("   - Removed 'avg' function: %s (result=%d)\n", 
           result == 1 ? "found and removed" : "not found", result);
    
    result = expr_context_remove_expression_function(ctx, "avg");
    printf("   - Try to remove 'avg' again: %s (result=%d)\n", 
           result == 0 ? "not found" : "error", result);
    
    result = expr_context_remove_expression_function(ctx, "nonexistent");
    printf("   - Remove non-existent function: %s (result=%d)\n", 
           result == 0 ? "not found" : "error", result);
    
    // Test 5: Error handling
    printf("\n5. Error handling:\n");
    
    result = expr_context_add_expression_function(NULL, "test", "x", "x");
    printf("   - Add to NULL context: %s (result=%d)\n",
           result < 0 ? "error" : "unexpected", result);
    
    result = expr_context_add_expression_function(ctx, NULL, "x", "x");
    printf("   - Add with NULL name: %s (result=%d)\n",
           result < 0 ? "error" : "unexpected", result);
    
    result = expr_context_add_expression_function(ctx, "test", NULL, "x");
    printf("   - Add with NULL params: %s (result=%d)\n",
           result < 0 ? "error" : "unexpected", result);
    
    result = expr_context_add_expression_function(ctx, "test", "x", NULL);
    printf("   - Add with NULL expression: %s (result=%d)\n",
           result < 0 ? "error" : "unexpected", result);
    
    // Test 6: Expression functions with variables
    printf("\n6. Expression functions with variables:\n");
    
    result = expr_context_add_expression_function(ctx, "scale", "x,factor", "x*factor");
    expr_batch_add_variable(batch, "my_factor", 2.5);
    expr_batch_add_expression(batch, "scale(10, my_factor)");
    expr_batch_evaluate(batch, ctx);
    Real scaled = expr_batch_get_result(batch, 4);
    printf("   - scale(10, my_factor) where my_factor=2.5 = %.1f (expected 25.0)\n", scaled);
    
    // Test 7: Batch-local expression functions
    printf("\n7. Batch-local expression functions:\n");
    
    // Add a context function
    result = expr_context_add_expression_function(ctx, "double", "x", "x*2");
    printf("   - Added context function 'double': %s\n", result == 0 ? "success" : "failed");
    
    // Add a batch-local function with the same name (should override)
    result = expr_batch_add_expression_function(batch, "double", "x", "x*3");
    printf("   - Added batch function 'double' (overrides context): %s\n", result == 0 ? "success" : "failed");
    
    // Add a batch-only function
    result = expr_batch_add_expression_function(batch, "triple", "x", "x*3");
    printf("   - Added batch function 'triple': %s\n", result == 0 ? "success" : "failed");
    
    // Test that batch function overrides context function
    expr_batch_add_expression(batch, "double(5)");
    expr_batch_evaluate(batch, ctx);
    Real doubled = expr_batch_get_result(batch, 5);
    printf("   - double(5) = %.1f (expected 15.0, batch overrides context)\n", doubled);
    
    // Test batch-only function
    expr_batch_add_expression(batch, "triple(4)");
    expr_batch_evaluate(batch, ctx);
    Real tripled = expr_batch_get_result(batch, 6);
    printf("   - triple(4) = %.1f (expected 12.0)\n", tripled);
    
    // Test 8: Multiple batches with different functions
    printf("\n8. Multiple batches with different functions:\n");
    
    ExprBatch* batch2 = expr_batch_new(arena);
    
    // Add different function to batch2
    result = expr_batch_add_expression_function(batch2, "quadruple", "x", "x*4");
    printf("   - Added 'quadruple' to batch2: %s\n", result == 0 ? "success" : "failed");
    
    // batch2 should not have access to batch1's functions
    expr_batch_add_expression(batch2, "double(5)");
    expr_batch_evaluate(batch2, ctx);
    Real ctx_double = expr_batch_get_result(batch2, 0);
    printf("   - batch2: double(5) = %.1f (expected 10.0, uses context function)\n", ctx_double);
    
    // But batch2 has its own function
    expr_batch_add_expression(batch2, "quadruple(5)");
    expr_batch_evaluate(batch2, ctx);
    Real quad = expr_batch_get_result(batch2, 1);
    printf("   - batch2: quadruple(5) = %.1f (expected 20.0)\n", quad);
    
    // Test 9: Remove batch-local functions
    printf("\n9. Removing batch-local functions:\n");
    
    result = expr_batch_remove_expression_function(batch, "double");
    printf("   - Removed batch function 'double': %s (result=%d)\n", 
           result == 1 ? "found and removed" : "not found", result);
    
    // Now should use context function again
    expr_batch_add_expression(batch, "double(5)");
    expr_batch_evaluate(batch, ctx);
    Real ctx_double2 = expr_batch_get_result(batch, 7);
    printf("   - double(5) after removal = %.1f (expected 10.0, back to context)\n", ctx_double2);
    
    // Try to remove non-existent function
    result = expr_batch_remove_expression_function(batch, "nonexistent");
    printf("   - Remove non-existent batch function: %s (result=%d)\n", 
           result == 0 ? "not found" : "error", result);
    
    // Test 10: Error handling for batch functions
    printf("\n10. Batch function error handling:\n");
    
    result = expr_batch_add_expression_function(NULL, "test", "x", "x");
    printf("   - Add to NULL batch: %s (result=%d)\n",
           result < 0 ? "error" : "unexpected", result);
    
    result = expr_batch_add_expression_function(batch, NULL, "x", "x");
    printf("   - Add with NULL name: %s (result=%d)\n",
           result < 0 ? "error" : "unexpected", result);
    
    result = expr_batch_add_expression_function(batch, "test", NULL, "x");
    printf("   - Add with NULL params: %s (result=%d)\n",
           result < 0 ? "error" : "unexpected", result);
    
    result = expr_batch_add_expression_function(batch, "test", "x", NULL);
    printf("   - Add with NULL expression: %s (result=%d)\n",
           result < 0 ? "error" : "unexpected", result);
    
    // Clean up
    expr_batch_free(batch2);
    expr_batch_free(batch);
    expr_arena_free(arena);
    expr_context_free(ctx);
    
    printf("\n=== All Expression Function Tests Completed ===\n");
    
    return 0;
}