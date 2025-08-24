#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
#include "exp_rs.h"

// Test that clear followed by free works correctly
void test_clear_then_free() {
    printf("=== Test Clear Then Free ===\n");
    
    // Create a batch
    ExprBatch* batch = expr_batch_new(8192);
    assert(batch != NULL);
    printf("✓ Batch created at %p\n", (void*)batch);
    
    // Add some data
    expr_batch_add_expression(batch, "x + y * 2");
    expr_batch_add_variable(batch, "x", 5.0);
    expr_batch_add_variable(batch, "y", 3.0);
    printf("✓ Added expression and variables\n");
    
    // Check it's valid
    ExprResult validity = expr_batch_is_valid(batch);
    assert(validity.status == 0 && validity.value == 1.0);
    printf("✓ Batch is valid before clear\n");
    
    // Clear the batch
    int clear_result = expr_batch_clear(batch);
    assert(clear_result == 0);
    printf("✓ Batch cleared successfully\n");
    
    // Check it's still valid after clear
    validity = expr_batch_is_valid(batch);
    assert(validity.status == 0 && validity.value == 1.0);
    printf("✓ Batch is still valid after clear\n");
    
    // Now free it - this should work fine
    expr_batch_free(batch);
    printf("✓ Batch freed successfully after clear\n");
    
    // Check it's now invalid (freed)
    validity = expr_batch_is_valid(batch);
    assert(validity.status == FFI_ERROR_INVALID_POINTER);
    printf("✓ Batch correctly marked as freed\n");
    printf("  Error message: %s\n", validity.error);
}

// Test multiple clear operations
void test_multiple_clears() {
    printf("\n=== Test Multiple Clears ===\n");
    
    ExprBatch* batch = expr_batch_new(8192);
    assert(batch != NULL);
    
    // Clear multiple times - all should succeed
    for (int i = 0; i < 5; i++) {
        // Add data
        expr_batch_add_expression(batch, "x * 2");
        expr_batch_add_variable(batch, "x", (Real)i);
        
        // Clear
        int result = expr_batch_clear(batch);
        assert(result == 0);
        printf("✓ Clear #%d succeeded\n", i + 1);
        
        // Verify still valid
        ExprResult validity = expr_batch_is_valid(batch);
        assert(validity.status == 0 && validity.value == 1.0);
    }
    
    // Finally free it
    expr_batch_free(batch);
    printf("✓ Batch freed after multiple clears\n");
}

// Test clear on freed batch (should fail safely)
void test_clear_on_freed_batch() {
    printf("\n=== Test Clear on Freed Batch ===\n");
    
    ExprBatch* batch = expr_batch_new(8192);
    assert(batch != NULL);
    
    // Free the batch
    expr_batch_free(batch);
    printf("✓ Batch freed\n");
    
    // Try to clear the freed batch - should return error
    int result = expr_batch_clear(batch);
    if (result != 0) {
        printf("✓ Clear on freed batch returned error code: %d\n", result);
        assert(result == FFI_ERROR_INVALID_POINTER);
    } else {
        printf("✗ FAILED: Clear on freed batch returned success\n");
        exit(1);
    }
}

// Test that clear doesn't affect the magic number
void test_clear_preserves_validity() {
    printf("\n=== Test Clear Preserves Validity ===\n");
    
    ExprBatch* batch = expr_batch_new(16384);
    assert(batch != NULL);
    
    // Add and clear many times
    for (int i = 0; i < 10; i++) {
        // Check valid
        ExprResult before = expr_batch_is_valid(batch);
        assert(before.status == 0 && before.value == 1.0);
        
        // Add expression
        char expr[32];
        snprintf(expr, sizeof(expr), "x + %d", i);
        expr_batch_add_expression(batch, expr);
        expr_batch_add_variable(batch, "x", (Real)i);
        
        // Clear
        int clear_result = expr_batch_clear(batch);
        assert(clear_result == 0);
        
        // Check still valid
        ExprResult after = expr_batch_is_valid(batch);
        assert(after.status == 0 && after.value == 1.0);
    }
    
    printf("✓ Batch remained valid through %d clear cycles\n", 10);
    
    // Free and verify it's now invalid
    expr_batch_free(batch);
    ExprResult freed = expr_batch_is_valid(batch);
    assert(freed.status == FFI_ERROR_INVALID_POINTER);
    printf("✓ Batch correctly invalidated after free\n");
}

int main() {
    printf("\n==== Clear and Free Interaction Tests ====\n\n");
    
    // FFI error codes
    #define FFI_ERROR_NULL_POINTER -1
    #define FFI_ERROR_INVALID_POINTER -5
    
    test_clear_then_free();
    test_multiple_clears();
    test_clear_on_freed_batch();
    test_clear_preserves_validity();
    
    printf("\n==== All Clear/Free Tests Passed! ====\n\n");
    return 0;
}