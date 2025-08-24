#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "exp_rs.h"
#include "common_allocator.h"

// Test that memory is actually freed
void test_memory_actually_freed() {
    printf("=== Test Memory Actually Freed ===\n");
    
    init_memory_tracking();
    reset_memory_stats();
    enable_allocation_tracking();
    
    memory_stats_t before = get_memory_stats();
    printf("Initial state: %zu bytes in use, %zu total allocated\n", 
           before.current_bytes, before.total_allocated_bytes);
    
    // Create multiple batches
    const int num_batches = 10;
    ExprBatch* batches[10];
    
    for (int i = 0; i < num_batches; i++) {
        batches[i] = expr_batch_new(8192);  // 8KB each
        assert(batches[i] != NULL);
        
        // Add some data to ensure arena is used
        expr_batch_add_expression(batches[i], "x + y * 2");
        expr_batch_add_variable(batches[i], "x", 1.0);
        expr_batch_add_variable(batches[i], "y", 2.0);
    }
    
    memory_stats_t after_create = get_memory_stats();
    printf("After creating %d batches: %zu bytes in use, %zu total allocated\n", 
           num_batches, after_create.current_bytes, after_create.total_allocated_bytes);
    
    size_t bytes_used = after_create.current_bytes - before.current_bytes;
    printf("Memory used by %d batches: %zu bytes (%.2f KB per batch)\n", 
           num_batches, bytes_used, (double)bytes_used / num_batches / 1024.0);
    
    // Free all batches
    for (int i = 0; i < num_batches; i++) {
        expr_batch_free(batches[i]);
    }
    
    memory_stats_t after_free = get_memory_stats();
    printf("After freeing all batches: %zu bytes in use, %zu total deallocated\n", 
           after_free.current_bytes, after_free.total_deallocated_bytes);
    
    // Check that memory was actually freed
    size_t leaked_bytes = after_free.current_bytes - before.current_bytes;
    printf("\nMemory leak check:\n");
    printf("  Bytes before: %zu\n", before.current_bytes);
    printf("  Bytes after: %zu\n", after_free.current_bytes);
    printf("  Leaked: %zu bytes\n", leaked_bytes);
    
    if (leaked_bytes > 0) {
        printf("❌ MEMORY LEAK DETECTED: %zu bytes not freed!\n", leaked_bytes);
        
        // Let's also check the allocation/deallocation counts
        printf("\nAllocation analysis:\n");
        printf("  Total allocations: %zu\n", after_free.total_allocs - before.total_allocs);
        printf("  Total deallocations: %zu\n", after_free.total_deallocs - before.total_deallocs);
        printf("  Leaked allocations: %zu\n", 
               (after_free.total_allocs - before.total_allocs) - 
               (after_free.total_deallocs - before.total_deallocs));
    } else {
        printf("✅ No memory leak detected - all memory freed!\n");
    }
    
    disable_allocation_tracking();
}

// Test that Drop is actually being called
void test_drop_is_called() {
    printf("\n=== Test Drop Is Being Called ===\n");
    
    init_memory_tracking();
    reset_memory_stats();
    enable_allocation_tracking();
    
    // Create and immediately free a batch
    memory_stats_t before = get_memory_stats();
    
    ExprBatch* batch = expr_batch_new(65536);  // Large 64KB batch
    assert(batch != NULL);
    
    memory_stats_t after_new = get_memory_stats();
    size_t bytes_allocated = after_new.current_bytes - before.current_bytes;
    printf("Batch allocated %zu bytes\n", bytes_allocated);
    
    // Free it
    expr_batch_free(batch);
    
    memory_stats_t after_free = get_memory_stats();
    size_t bytes_after_free = after_free.current_bytes - before.current_bytes;
    
    printf("After free: %zu bytes still in use (should be 0)\n", bytes_after_free);
    
    if (bytes_after_free == bytes_allocated) {
        printf("❌ CRITICAL: Memory was NOT freed! Drop may not be working!\n");
    } else if (bytes_after_free > 0) {
        printf("⚠️  WARNING: Partial memory leak - %zu of %zu bytes leaked\n", 
               bytes_after_free, bytes_allocated);
    } else {
        printf("✅ Memory properly freed\n");
    }
    
    disable_allocation_tracking();
}

// Test create/free cycle multiple times
void test_repeated_alloc_free() {
    printf("\n=== Test Repeated Allocation/Free Cycles ===\n");
    
    init_memory_tracking();
    reset_memory_stats();
    enable_allocation_tracking();
    
    memory_stats_t initial = get_memory_stats();
    size_t initial_bytes = initial.current_bytes;
    
    const int cycles = 100;
    for (int i = 0; i < cycles; i++) {
        ExprBatch* batch = expr_batch_new(4096);
        assert(batch != NULL);
        
        // Use it
        expr_batch_add_expression(batch, "sin(x)");
        expr_batch_add_variable(batch, "x", 3.14159);
        
        // Free it
        expr_batch_free(batch);
        
        if (i % 10 == 9) {
            memory_stats_t current = get_memory_stats();
            size_t current_leak = current.current_bytes - initial_bytes;
            printf("After %d cycles: %zu bytes leaked\n", i + 1, current_leak);
            
            if (current_leak > (i + 1) * 100) {  // Allow small overhead
                printf("⚠️  WARNING: Memory appears to be growing! Possible leak.\n");
            }
        }
    }
    
    memory_stats_t final = get_memory_stats();
    size_t final_leak = final.current_bytes - initial_bytes;
    
    printf("\nFinal result after %d create/free cycles:\n", cycles);
    printf("  Memory leaked: %zu bytes\n", final_leak);
    printf("  Average leak per cycle: %.2f bytes\n", (double)final_leak / cycles);
    
    if (final_leak > cycles * 10) {  // Allow very small overhead
        printf("❌ FAIL: Significant memory leak detected!\n");
    } else if (final_leak > 0) {
        printf("⚠️  Minor leak detected (may be acceptable)\n");
    } else {
        printf("✅ PASS: No memory leak\n");
    }
    
    disable_allocation_tracking();
}

int main() {
    printf("\n==== Memory Deallocation Verification Tests ====\n\n");
    
    test_memory_actually_freed();
    test_drop_is_called();
    test_repeated_alloc_free();
    
    printf("\n==== Test Complete ====\n\n");
    return 0;
}