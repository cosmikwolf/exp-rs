#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
#include <signal.h>
#include <setjmp.h>
#include "exp_rs.h"
#include "common_allocator.h"

// FFI error codes from exp_rs.h
#define FFI_ERROR_NULL_POINTER -1
#define FFI_ERROR_INVALID_POINTER -5

// Signal handling for catching segfaults
static jmp_buf jump_buffer;
static int signal_caught = 0;

void segfault_handler(int sig) {
    (void)sig;
    signal_caught = 1;
    longjmp(jump_buffer, 1);
}

// Test that double-free is handled safely
void test_double_free_protection() {
    printf("=== Test Double-Free Protection ===\n");
    
    // Create a batch
    ExprBatch* batch = expr_batch_new(8192);
    assert(batch != NULL);
    printf("✓ Batch created at %p\n", (void*)batch);
    
    // Check that new batch is valid
    ExprResult validity = expr_batch_is_valid(batch);
    printf("Batch validity before use: status=%d, value=%.1f\n", validity.status, validity.value);
    if (validity.status != 0) {
        printf("Error: %s\n", validity.error);
    }
    assert(validity.status == 0 && validity.value == 1.0);
    
    // Add some data to make it a valid batch
    expr_batch_add_expression(batch, "x + 1");
    expr_batch_add_variable(batch, "x", 5.0);
    
    // Check validity after adding data
    validity = expr_batch_is_valid(batch);
    printf("Batch validity after adding data: status=%d, value=%.1f\n", validity.status, validity.value);
    assert(validity.status == 0 && validity.value == 1.0);
    
    // First free - should work
    printf("Freeing batch for the first time...\n");
    expr_batch_free(batch);
    printf("✓ First free succeeded\n");
    
    // Check validity after free
    validity = expr_batch_is_valid(batch);
    printf("Batch validity after free: status=%d\n", validity.status);
    if (validity.status != 0) {
        printf("Expected error message: %s\n", validity.error);
    }
    assert(validity.status == FFI_ERROR_INVALID_POINTER);
    
    // Second free - should be safely ignored (not crash)
    printf("Attempting to free the same batch again...\n");
    
    // Set up signal handler to catch potential segfault
    signal_caught = 0;
    signal(SIGSEGV, segfault_handler);
    
    if (setjmp(jump_buffer) == 0) {
        // This should NOT crash - double-free protection should handle it
        expr_batch_free(batch);
        
        if (!signal_caught) {
            printf("✓ Double-free handled safely (no crash)\n");
        } else {
            printf("✗ FAILED: Segfault occurred on double-free\n");
            exit(1);
        }
    } else {
        printf("✗ FAILED: Segfault caught on double-free\n");
        exit(1);
    }
    
    // Restore default signal handler
    signal(SIGSEGV, SIG_DFL);
}

// Test that operations on freed batch are handled safely
void test_use_after_free_protection() {
    printf("\n=== Test Use-After-Free Protection ===\n");
    
    // Create and free a batch
    ExprBatch* batch = expr_batch_new(8192);
    assert(batch != NULL);
    printf("✓ Batch created at %p\n", (void*)batch);
    
    expr_batch_add_expression(batch, "x * 2");
    expr_batch_add_variable(batch, "x", 3.0);
    
    printf("Freeing batch...\n");
    expr_batch_free(batch);
    printf("✓ Batch freed\n");
    
    // Try to use the freed batch - should be handled safely
    printf("Attempting to clear freed batch...\n");
    
    signal_caught = 0;
    signal(SIGSEGV, segfault_handler);
    
    if (setjmp(jump_buffer) == 0) {
        int result = expr_batch_clear(batch);
        
        if (!signal_caught) {
            if (result != 0) {
                printf("✓ Clear on freed batch returned error code: %d\n", result);
            } else {
                printf("⚠ Clear on freed batch returned success (0) - may need investigation\n");
            }
        } else {
            printf("✗ FAILED: Segfault occurred on use-after-free\n");
            exit(1);
        }
    } else {
        printf("✗ FAILED: Segfault caught on use-after-free\n");
        exit(1);
    }
    
    // Restore default signal handler
    signal(SIGSEGV, SIG_DFL);
}

// Test NULL pointer handling
void test_null_pointer_handling() {
    printf("\n=== Test NULL Pointer Handling ===\n");
    
    // Check validity of NULL pointer
    printf("Testing is_valid on NULL pointer...\n");
    ExprResult validity = expr_batch_is_valid(NULL);
    printf("NULL pointer validity: status=%d\n", validity.status);
    if (validity.status != 0) {
        printf("Expected error message: %s\n", validity.error);
    }
    assert(validity.status == FFI_ERROR_NULL_POINTER);
    
    // Double-free on NULL should be safe
    printf("Testing double-free on NULL pointer...\n");
    expr_batch_free(NULL);
    expr_batch_free(NULL);
    printf("✓ NULL pointer double-free handled safely\n");
    
    // Clear on NULL should return error
    printf("Testing clear on NULL pointer...\n");
    int result = expr_batch_clear(NULL);
    if (result != 0) {
        printf("✓ Clear on NULL returned error code: %d\n", result);
    } else {
        printf("✗ FAILED: Clear on NULL returned success\n");
        exit(1);
    }
}

// Test invalid pointer detection
void test_invalid_pointer_detection() {
    printf("\n=== Test Invalid Pointer Detection ===\n");
    
    // Create a fake pointer that wasn't allocated by expr_batch_new
    char fake_data[1024];
    // Fill with non-magic data
    for (int i = 0; i < 1024; i++) {
        fake_data[i] = (char)(i % 256);
    }
    ExprBatch* fake_batch = (ExprBatch*)fake_data;
    
    printf("Testing operations on invalid pointer %p...\n", (void*)fake_batch);
    
    // Check validity of invalid pointer
    ExprResult validity = expr_batch_is_valid(fake_batch);
    printf("Invalid pointer validity: status=%d\n", validity.status);
    if (validity.status != 0) {
        printf("Expected error message: %s\n", validity.error);
    }
    assert(validity.status == FFI_ERROR_INVALID_POINTER);
    
    signal_caught = 0;
    signal(SIGSEGV, segfault_handler);
    
    // Try to free an invalid pointer
    if (setjmp(jump_buffer) == 0) {
        expr_batch_free(fake_batch);
        
        if (!signal_caught) {
            printf("✓ Invalid pointer free handled safely\n");
        } else {
            printf("✗ FAILED: Segfault on invalid pointer\n");
            exit(1);
        }
    } else {
        printf("✗ FAILED: Segfault caught on invalid pointer\n");
        exit(1);
    }
    
    // Try to clear an invalid pointer
    if (setjmp(jump_buffer) == 0) {
        int result = expr_batch_clear(fake_batch);
        
        if (!signal_caught) {
            if (result != 0) {
                printf("✓ Clear on invalid pointer returned error: %d\n", result);
            } else {
                printf("⚠ Clear on invalid pointer returned success\n");
            }
        } else {
            printf("✗ FAILED: Segfault on invalid pointer clear\n");
            exit(1);
        }
    } else {
        printf("✗ FAILED: Segfault caught on invalid pointer clear\n");
        exit(1);
    }
    
    // Restore default signal handler
    signal(SIGSEGV, SIG_DFL);
}

int main() {
    init_memory_tracking();
    printf("\n==== Double-Free Protection Tests ====\n\n");
    
    test_double_free_protection();
    test_use_after_free_protection();
    test_null_pointer_handling();
    test_invalid_pointer_detection();
    
    printf("\n==== All Protection Tests Passed! ====\n\n");
    return 0;
}