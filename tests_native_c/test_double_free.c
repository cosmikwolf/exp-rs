#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
#include <signal.h>
#include <setjmp.h>
#include "exp_rs.h"

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
    
    // Add some data to make it a valid batch
    expr_batch_add_expression(batch, "x + 1");
    expr_batch_add_variable(batch, "x", 5.0);
    
    // First free - should work
    printf("Freeing batch for the first time...\n");
    expr_batch_free(batch);
    printf("✓ First free succeeded\n");
    
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
    ExprBatch* fake_batch = (ExprBatch*)fake_data;
    
    printf("Testing operations on invalid pointer %p...\n", (void*)fake_batch);
    
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
    printf("\n==== Double-Free Protection Tests ====\n\n");
    
    test_double_free_protection();
    test_use_after_free_protection();
    test_null_pointer_handling();
    test_invalid_pointer_detection();
    
    printf("\n==== All Protection Tests Passed! ====\n\n");
    return 0;
}