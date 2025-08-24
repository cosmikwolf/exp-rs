#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "exp_rs.h"

// Test that we can't read the magic number after free
// If Drop worked, the memory should be deallocated
void test_magic_after_free() {
    printf("=== Test Magic Number After Free ===\n");
    
    // Create a batch
    ExprBatch* batch = expr_batch_new(8192);
    assert(batch != NULL);
    printf("Batch created at %p\n", (void*)batch);
    
    // Verify it's valid
    ExprResult validity = expr_batch_is_valid(batch);
    assert(validity.status == 0);
    printf("✓ Batch is valid before free\n");
    
    // Store the pointer value
    void* saved_ptr = (void*)batch;
    
    // Free the batch
    expr_batch_free(batch);
    printf("Batch freed\n");
    
    // Now check if we can still read the magic
    // This is technically undefined behavior, but for testing...
    validity = expr_batch_is_valid(batch);
    
    if (validity.status == -5) {  // FFI_ERROR_INVALID_POINTER
        // Check the error message
        if (strstr(validity.error, "freed") != NULL) {
            printf("⚠️  WARNING: Magic number still readable as BATCH_FREED after free!\n");
            printf("This suggests memory might not be immediately deallocated.\n");
            printf("Error message: %s\n", validity.error);
            
            // This could mean:
            // 1. The memory allocator is caching freed memory
            // 2. The memory hasn't been unmapped yet
            // 3. Or there's a real memory leak
            
            // Let's allocate a new batch and see if it reuses the same address
            ExprBatch* new_batch = expr_batch_new(8192);
            printf("\nNew batch allocated at %p (old was %p)\n", (void*)new_batch, saved_ptr);
            
            if ((void*)new_batch == saved_ptr) {
                printf("✓ GOOD: Same address reused - memory was freed and reallocated\n");
            } else {
                printf("ℹ️  Different address used - allocator chose a different location\n");
            }
            
            expr_batch_free(new_batch);
        } else {
            printf("Got error: %s\n", validity.error);
        }
    } else {
        printf("Status: %d\n", validity.status);
    }
}

// Test memory allocation pattern
void test_allocation_pattern() {
    printf("\n=== Test Allocation Pattern ===\n");
    
    // Allocate many batches and track their addresses
    const int count = 5;
    ExprBatch* batches[5];
    void* addresses[5];
    
    printf("Allocating %d batches...\n", count);
    for (int i = 0; i < count; i++) {
        batches[i] = expr_batch_new(4096);
        addresses[i] = (void*)batches[i];
        printf("  Batch %d at %p\n", i, addresses[i]);
    }
    
    printf("\nFreeing all batches...\n");
    for (int i = 0; i < count; i++) {
        expr_batch_free(batches[i]);
    }
    
    printf("\nAllocating %d new batches...\n", count);
    for (int i = 0; i < count; i++) {
        ExprBatch* new_batch = expr_batch_new(4096);
        printf("  New batch %d at %p", i, (void*)new_batch);
        
        // Check if it reused an old address
        int reused = 0;
        for (int j = 0; j < count; j++) {
            if ((void*)new_batch == addresses[j]) {
                printf(" (reused from old batch %d)", j);
                reused = 1;
                break;
            }
        }
        if (!reused) {
            printf(" (new address)");
        }
        printf("\n");
        
        // Clean up
        expr_batch_free(new_batch);
    }
}

// Force allocator to show if memory is really freed
void test_memory_pressure() {
    printf("\n=== Test Memory Pressure ===\n");
    
    // Allocate a large batch
    const size_t large_size = 1024 * 1024;  // 1MB
    ExprBatch* large = expr_batch_new(large_size);
    void* large_addr = (void*)large;
    printf("Allocated 1MB batch at %p\n", large_addr);
    
    // Free it
    expr_batch_free(large);
    printf("Freed 1MB batch\n");
    
    // Allocate many small batches to see if memory is reclaimed
    const int small_count = 100;
    int reused_large = 0;
    
    printf("Allocating %d small batches...\n", small_count);
    for (int i = 0; i < small_count; i++) {
        ExprBatch* small = expr_batch_new(1024);
        
        // Check if any part of the large allocation was reused
        // (This is a heuristic - might not catch all reuse)
        if ((char*)small >= (char*)large_addr && 
            (char*)small < ((char*)large_addr + large_size)) {
            reused_large++;
        }
        
        expr_batch_free(small);
    }
    
    if (reused_large > 0) {
        printf("✓ %d small allocations reused memory from freed large batch\n", reused_large);
        printf("  This indicates the large batch memory was properly freed\n");
    } else {
        printf("ℹ️  No small allocations reused the large batch memory\n");
        printf("  This could mean:\n");
        printf("  - The allocator is using different pools for different sizes\n");
        printf("  - The memory is fragmented\n");
        printf("  - Or there might be a leak\n");
    }
}

int main() {
    printf("\n==== Drop and Memory Deallocation Verification ====\n\n");
    
    test_magic_after_free();
    test_allocation_pattern();
    test_memory_pressure();
    
    printf("\n==== Test Complete ====\n\n");
    return 0;
}