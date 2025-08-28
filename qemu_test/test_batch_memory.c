/**
 * Memory allocation test for exp-rs batch API
 * Tests memory allocation and deallocation on embedded targets
 */
#include "exp_rs.h"
#include "qemu_harness/qemu_test_harness.h"
#include "register_test_functions.h"
#include <assert.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Try to include dladdr for symbol resolution
#ifdef __GLIBC__
#define _GNU_SOURCE
#include <dlfcn.h>
#define HAS_DLADDR 1
#else
#define HAS_DLADDR 0
#endif

// Memory tracking for embedded using allocation table
static size_t total_allocated = 0;
static size_t total_freed = 0;
static size_t current_allocated = 0;
static size_t peak_allocated = 0;
static size_t allocation_count = 0;
static size_t free_count = 0;

// Allocation table to track individual allocations with caller info
#define MAX_ALLOCATIONS 2000
static struct {
  void *ptr;
  size_t size;
  void *caller;  // Who allocated this memory
} allocation_table[MAX_ALLOCATIONS];
static int allocation_table_size = 0;

// Helper function to format caller address with symbol name if available
static void format_caller(void *addr, char *buffer, size_t buffer_size) {
#if HAS_DLADDR
    Dl_info info;
    if (dladdr(addr, &info) && info.dli_sname) {
        snprintf(buffer, buffer_size, "%s+%p", info.dli_sname, 
                 (void*)((char*)addr - (char*)info.dli_saddr));
    } else {
        snprintf(buffer, buffer_size, "%p", addr);
    }
#else
    snprintf(buffer, buffer_size, "%p", addr);
#endif
}

// Rust allocation tracking functions  
extern size_t exp_rs_get_current_allocated(void);
extern size_t exp_rs_get_allocation_count(void);

// Helper to show arena usage with detailed information
void show_arena_usage(ExprBatch *batch, const char *label) {
  if (!batch)
    return;

  uintptr_t arena_bytes = expr_batch_arena_bytes(batch);
  size_t rust_allocated = exp_rs_get_current_allocated();
  size_t rust_count = exp_rs_get_allocation_count();
  qemu_printf("%s: arena=%d bytes, rust_mem=%d bytes, rust_count=%d, sys_mem=%d bytes, sys_count=%d\n", label,
              (int)arena_bytes, (int)rust_allocated, (int)rust_count, (int)current_allocated, (int)allocation_count);
}
#define FFI_ERROR_NULL_POINTER -1
#define FFI_ERROR_INVALID_POINTER -5

// Custom allocation functions for tracking
// These now use standard malloc/free and are used only for memory tracking
void *exp_rs_malloc(size_t size) {
  void *ptr = malloc(size);

  if (ptr) {
    // Track allocation in table with caller info
    if (allocation_table_size < MAX_ALLOCATIONS) {
      allocation_table[allocation_table_size].ptr = ptr;
      allocation_table[allocation_table_size].size = size;
      allocation_table[allocation_table_size].caller = __builtin_return_address(0);
      allocation_table_size++;
    }
    
    // Update counters
    total_allocated += size;
    current_allocated += size;
    allocation_count++;
    if (current_allocated > peak_allocated) {
      peak_allocated = current_allocated;
    }
    qemu_printf("[ALLOC] %d bytes at %p (total: %d, count: %d, table: %d)\n", 
                (int)size, ptr, (int)current_allocated, (int)allocation_count, allocation_table_size);
  }
  return ptr;
}

void exp_rs_free(void *ptr) {
  if (ptr) {
    size_t freed_size = 0;
    
    // Find allocation in table and remove it
    for (int i = 0; i < allocation_table_size; i++) {
      if (allocation_table[i].ptr == ptr) {
        freed_size = allocation_table[i].size;
        void *alloc_caller = allocation_table[i].caller;
        void *free_caller = __builtin_return_address(0);
        
        char alloc_caller_str[256];
        char free_caller_str[256];
        format_caller(alloc_caller, alloc_caller_str, sizeof(alloc_caller_str));
        format_caller(free_caller, free_caller_str, sizeof(free_caller_str));
        
        qemu_printf("[FREE] ptr %p allocated by %s, freed by %s\n", 
                    ptr, alloc_caller_str, free_caller_str);
        
        // Remove entry by moving last entry to this position
        allocation_table[i] = allocation_table[allocation_table_size - 1];
        allocation_table_size--;
        break;
      }
    }
    
    // Update counters
    if (freed_size > 0) {
      current_allocated -= freed_size;
      total_freed += freed_size;
    }
    free_count++;
    
    qemu_printf("[FREE] ptr %p (freed: %d bytes, free count: %d, table: %d)\n", 
                ptr, (int)freed_size, (int)free_count, allocation_table_size);
    free(ptr);
  }
}

// Helper function to dump remaining allocations
void dump_remaining_allocations(void) {
  if (allocation_table_size > 0) {
    qemu_printf("\n=== REMAINING ALLOCATIONS ===\n");
    for (int i = 0; i < allocation_table_size; i++) {
      char caller_str[256];
      format_caller(allocation_table[i].caller, caller_str, sizeof(caller_str));
      qemu_printf("LEAK: %d bytes at %p allocated by %s\n", 
                  (int)allocation_table[i].size, 
                  allocation_table[i].ptr, 
                  caller_str);
    }
    qemu_printf("=== END REMAINING ALLOCATIONS ===\n");
  }
}

// Helper function to populate batch with test data
// Returns 1 on success, 0 on failure
int populate_batch_with_test_data(ExprBatch *batch, ExprContext *ctx) {
  show_arena_usage(batch, "Before adding variables");
  // Add 10 parameters (p0-p9)
  for (int i = 0; i < 10; i++) {
    char param_name[8];
    snprintf(param_name, sizeof(param_name), "p%d", i);
    ExprResult result =
        expr_batch_add_variable(batch, param_name, (Real)(i + 1));
    if (result.status != 0) {
      return 0;
    }
  }
  show_arena_usage(batch, "After adding variables");

  // Add fewer, simpler expression functions for small arenas
  const char *functions[][3] = {{"f0", "x", "x * 2"}, {"f1", "x,y", "x + y"}};

  for (int i = 0; i < 2; i++) {
    int result = expr_batch_add_expression_function(
        batch, functions[i][0], functions[i][1], functions[i][2]);
    if (result != 0) {
      return 0;
    }
  }
  show_arena_usage(batch, "After adding functions");

  // Add fewer, simpler test expressions
  const char *test_expressions[] = {"f0(p0) + p1", "f1(p2, p3)"};

  for (int i = 0; i < 2; i++) {
    ExprResult result = expr_batch_add_expression(batch, test_expressions[i]);
    if (result.status != 0) {
      return 0;
    }
  }
  show_arena_usage(batch, "After adding expressions");

  // Evaluate the batch
  int eval_result = expr_batch_evaluate(batch, ctx);
  if (eval_result != 0) {
    return 0;
  }
  show_arena_usage(batch, "After evaluation");

  // Validate results (basic sanity check - not NaN)
  for (int i = 0; i < 2; i++) {
    Real result = expr_batch_get_result(batch, i);
    if (result != result) { // NaN check
      return 0;
    }
  }

  return 1;
}

// Simple helper for stress test - uses minimal data to fit in small arenas
int populate_batch_simple(ExprBatch *batch, ExprContext *ctx) {
  // Add just 2 variables
  ExprResult result = expr_batch_add_variable(batch, "x", 5.0);
  if (result.status != 0) {
    return 0;
  }

  result = expr_batch_add_variable(batch, "y", 3.0);
  if (result.status != 0) {
    return 0;
  }

  // Add 1 simple expression
  result = expr_batch_add_expression(batch, "x + y * 2");
  if (result.status != 0) {
    return 0;
  }

  // Evaluate
  int eval_result = expr_batch_evaluate(batch, ctx);
  if (eval_result != 0) {
    return 0;
  }

  return 1;
}

// Test 1: Basic batch lifecycle
void test_batch_lifecycle(ExprContext *ctx) {
  qemu_printf("\n=== Test 1: Basic Batch Lifecycle ===\n");

  size_t start_allocated = current_allocated;
  size_t start_alloc_count = allocation_count;
  size_t start_free_count = free_count;
  qemu_printf("Memory before batch creation: %d bytes\n", (int)start_allocated);

  // Create a batch with 8KB arena
  ExprBatch *batch = expr_batch_new(8192);
  if (!batch) {
    qemu_printf("ERROR: Failed to create batch\n");
    return;
  }

  qemu_printf("Batch created at %p\n", (void *)batch);
  qemu_printf("Memory after batch creation: %d bytes (delta: %d)\n",
              (int)current_allocated,
              (int)(current_allocated - start_allocated));

  // Check initial arena allocation
  size_t arena_bytes = expr_batch_arena_bytes(batch);
  qemu_printf("Initial arena bytes: %d\n", (int)arena_bytes);

  // Populate batch with test data
  int populate_result = populate_batch_with_test_data(batch, ctx);
  if (!populate_result) {
    qemu_printf("ERROR: Failed to populate batch with test data\n");
  } else {
    qemu_printf("Batch populated with test data successfully\n");
  }

  qemu_printf("Memory after adding data: %d bytes\n", (int)current_allocated);

  // Check arena usage after adding data
  arena_bytes = expr_batch_arena_bytes(batch);
  qemu_printf("Final arena bytes: %d\n", (int)arena_bytes);

  // Free the batch
  expr_batch_free(batch);
  qemu_printf("Batch freed\n");
  qemu_printf("Memory after free: %d bytes\n", (int)current_allocated);

  size_t end_allocated = current_allocated;
  size_t alloc_delta = allocation_count - start_alloc_count;
  size_t free_delta = free_count - start_free_count;

  qemu_printf("Allocations in this test: %d, Frees: %d\n", (int)alloc_delta,
              (int)free_delta);
  if (alloc_delta == free_delta) {
    qemu_printf("SUCCESS: All allocations freed (count-based)\n");
  } else {
    qemu_printf("WARNING: %d allocations not freed!\n",
                (int)(alloc_delta - free_delta));
  }
}

// Test 2: Multiple batches
void test_multiple_batches(ExprContext *ctx) {
  qemu_printf("\n=== Test 2: Multiple Batches ===\n");

  size_t start_allocated = current_allocated;
  qemu_printf("Starting memory: %d bytes\n", (int)start_allocated);

  // Create multiple batches
  const int num_batches = 5;
  ExprBatch *batches[5];

  for (int i = 0; i < num_batches; i++) {
    batches[i] = expr_batch_new(4096); // 4KB each
    if (!batches[i]) {
      qemu_printf("ERROR: Failed to create batch %d\n", i);
      // Clean up already created batches
      for (int j = 0; j < i; j++) {
        expr_batch_free(batches[j]);
      }
      return;
    }
    qemu_printf("Created batch %d at %p\n", i, (void *)batches[i]);

    // Populate each batch with test data
    int populate_result = populate_batch_with_test_data(batches[i], ctx);
    if (!populate_result) {
      qemu_printf("ERROR: Failed to populate batch %d with test data\n", i);
    }
  }

  qemu_printf("Memory after creating %d batches: %d bytes\n", num_batches,
              (int)current_allocated);

  // Free all batches
  for (int i = 0; i < num_batches; i++) {
    expr_batch_free(batches[i]);
    qemu_printf("Freed batch %d\n", i);
  }

  qemu_printf("Memory after freeing all: %d bytes\n", (int)current_allocated);

  size_t leaked = current_allocated - start_allocated;
  if (leaked > 0) {
    qemu_printf("WARNING: %d bytes leaked across %d batches!\n", (int)leaked,
                num_batches);
  } else {
    qemu_printf("SUCCESS: All memory freed\n");
  }
}

// Test 3: Clear and reuse
void test_clear_and_reuse(ExprContext *ctx) {
  qemu_printf("\n=== Test 3: Clear and Reuse ===\n");

  size_t start_allocated = current_allocated;

  ExprBatch *batch = expr_batch_new(8192);
  if (!batch) {
    qemu_printf("ERROR: Failed to create batch\n");
    return;
  }

  qemu_printf("Initial batch created, memory: %d bytes\n",
              (int)current_allocated);

  // Use and clear multiple times
  for (int i = 0; i < 10; i++) {
    qemu_printf("\nIteration %d:\n", i + 1);

    // Populate batch with test data
    int populate_result = populate_batch_with_test_data(batch, ctx);
    if (!populate_result) {
      qemu_printf("ERROR: Failed to populate batch with test data\n");
    }

    qemu_printf("  After adding data: %d bytes\n", (int)current_allocated);

    // Show arena usage
    size_t arena_bytes = expr_batch_arena_bytes(batch);
    qemu_printf("  Arena bytes: %d\n", (int)arena_bytes);

    // Clear the batch
    int clear_result = expr_batch_clear(batch);
    if (clear_result != 0) {
      qemu_printf("ERROR: Failed to clear batch: %d\n", clear_result);
    }

    qemu_printf("  After clear: %d bytes\n", (int)current_allocated);
  }

  expr_batch_free(batch);
  qemu_printf("\nBatch freed, final memory: %d bytes\n",
              (int)current_allocated);

  size_t leaked = current_allocated - start_allocated;
  if (leaked > 0) {
    qemu_printf("WARNING: %d bytes leaked!\n", (int)leaked);
  } else {
    qemu_printf("SUCCESS: No memory leak after clear/reuse cycles\n");
  }
}

// Test 4: Verify batch validity checking and double-free protection
void test_batch_validity(ExprContext *ctx) {
  qemu_printf("\n=== Test 4: Batch Validity & Double-Free Protection ===\n");

  int tests_passed = 0;
  int tests_failed = 0;

  // Test 1: Valid batch should be detected as valid
  ExprBatch *batch = expr_batch_new(4096);
  if (!batch) {
    qemu_printf("ERROR: Failed to create batch\n");
    return;
  }

  // Add some data to the batch
  int populate_result = populate_batch_with_test_data(batch, ctx);
  if (!populate_result) {
    qemu_printf("ERROR: Failed to populate batch with test data\n");
  }

  ExprResult validity = expr_batch_is_valid(batch);
  if (validity.status == 0 && validity.value == 1.0) {
    qemu_printf("✓ Test 1: Valid batch correctly detected\n");
    tests_passed++;
  } else {
    qemu_printf("✗ Test 1: Failed to detect valid batch\n");
    tests_failed++;
  }

  // Store the pointer value before freeing
  void *batch_ptr = batch;

  // Test 2: Free the batch
  expr_batch_free(batch);
  qemu_printf("Batch freed at %p\n", batch_ptr);

  // Test 3: Check validity after free (should detect double-free)
  validity = expr_batch_is_valid(batch);
  if (validity.status == FFI_ERROR_INVALID_POINTER) {
    qemu_printf("✓ Test 2: Freed batch correctly detected\n");
    qemu_printf("  Message: %s\n", validity.error);
    tests_passed++;
  } else {
    qemu_printf("✗ Test 2: Failed to detect freed batch\n");
    tests_failed++;
  }

  // Test 4: Attempt double-free (should be safe)
  qemu_printf("Attempting double-free...\n");
  expr_batch_free(batch); // This should safely do nothing
  qemu_printf("✓ Test 3: Double-free protection worked (no crash)\n");
  tests_passed++;

  // Test 5: NULL pointer handling
  validity = expr_batch_is_valid(NULL);
  if (validity.status == FFI_ERROR_NULL_POINTER) {
    qemu_printf("✓ Test 4: NULL correctly detected\n");
    tests_passed++;
  } else {
    qemu_printf("✗ Test 4: Failed to detect NULL\n");
    tests_failed++;
  }

  // Test 6: Create multiple batches and verify independence
  ExprBatch *batch1 = expr_batch_new(2048);
  ExprBatch *batch2 = expr_batch_new(2048);

  if (batch1 && batch2) {
    // Both should be valid
    ExprResult v1 = expr_batch_is_valid(batch1);
    ExprResult v2 = expr_batch_is_valid(batch2);

    if (v1.status == 0 && v2.status == 0) {
      qemu_printf("✓ Test 5: Multiple batches independently valid\n");
      tests_passed++;

      // Free first batch
      expr_batch_free(batch1);

      // Second should still be valid
      v2 = expr_batch_is_valid(batch2);
      if (v2.status == 0) {
        qemu_printf("✓ Test 6: Batch2 still valid after batch1 freed\n");
        tests_passed++;
      } else {
        qemu_printf("✗ Test 6: Batch2 incorrectly invalidated\n");
        tests_failed++;
      }

      // Clean up
      expr_batch_free(batch2);
    } else {
      qemu_printf("✗ Test 5: Failed to create valid batches\n");
      tests_failed++;
      if (batch1)
        expr_batch_free(batch1);
      if (batch2)
        expr_batch_free(batch2);
    }
  } else {
    qemu_printf("✗ Test 5-6: Failed to create test batches\n");
    tests_failed += 2;
  }

  // Summary
  qemu_printf("\nValidity test summary: %d passed, %d failed\n", tests_passed,
              tests_failed);

  if (tests_failed > 0) {
    qemu_printf("ERROR: Some validity tests failed!\n");
  }
}

// Test 5: Static batch pointer test
void test_static_batch_pointer(ExprContext *ctx) {
  qemu_printf("\n=== Test 5: Static Batch Pointer Test ===\n");

  // Test the scenario the user described
  static ExprBatch *batch_ = NULL;

  size_t start_allocated = current_allocated;
  qemu_printf("Starting memory: %d bytes\n", (int)start_allocated);

  // First allocation
  batch_ = expr_batch_new(4096);
  if (!batch_) {
    qemu_printf("ERROR: Failed to create static batch\n");
    return;
  }
  qemu_printf("Static batch created at %p\n", (void *)batch_);

  // Populate with data
  int populate_result = populate_batch_simple(batch_, ctx);
  if (!populate_result) {
    qemu_printf("ERROR: Failed to populate static batch\n");
  }

  qemu_printf("Memory after first batch: %d bytes\n", (int)current_allocated);

  // Free the batch but DON'T set pointer to NULL (this simulates the user's
  // bug)
  expr_batch_free(batch_);
  // batch_ = NULL; // <-- User forgot this!
  qemu_printf("Batch freed, but pointer not set to NULL!\n");
  qemu_printf("batch_ still points to: %p\n", (void *)batch_);

  size_t after_free = current_allocated;
  qemu_printf("Memory after free: %d bytes\n", (int)after_free);

  // Test validity check on freed batch
  ExprResult validity = expr_batch_is_valid(batch_);
  if (validity.status == FFI_ERROR_INVALID_POINTER) {
    qemu_printf("✓ Freed batch correctly detected as invalid\n");
  } else {
    qemu_printf("✗ Failed to detect freed batch\n");
  }

  // Now create a new batch (this simulates reusing the static pointer)
  // First free the old pointer if it exists (defensive programming)
  if (batch_) {
    qemu_printf("Attempting to free already-freed batch (should be safe)...\n");
    expr_batch_free(batch_); // Should be safe due to double-free protection
  }

  // Create new batch and properly set pointer
  batch_ = expr_batch_new(4096);
  if (!batch_) {
    qemu_printf("ERROR: Failed to create second static batch\n");
    return;
  }
  qemu_printf("Second static batch created at %p\n", (void *)batch_);

  // Populate with data again
  populate_result = populate_batch_simple(batch_, ctx);
  if (!populate_result) {
    qemu_printf("ERROR: Failed to populate second static batch\n");
  }

  qemu_printf("Memory after second batch: %d bytes\n", (int)current_allocated);

  // Properly clean up this time
  expr_batch_free(batch_);
  batch_ = NULL; // <-- Proper cleanup!
  qemu_printf("Second batch properly freed and pointer set to NULL\n");

  size_t final_allocated = current_allocated;
  qemu_printf("Final memory: %d bytes\n", (int)final_allocated);

  size_t leaked = final_allocated - start_allocated;
  if (leaked > 0) {
    qemu_printf("WARNING: %d bytes leaked in static pointer test!\n",
                (int)leaked);
  } else {
    qemu_printf(
        "SUCCESS: No memory leak with proper static pointer management\n");
  }
}

// Helper function to print detailed heap tracking
void print_heap_stats(const char *label) {
  extern size_t exp_rs_get_total_allocated(void);
  extern size_t exp_rs_get_total_freed(void);  
  extern size_t exp_rs_get_free_count(void);
  
  size_t rust_current = exp_rs_get_current_allocated();
  size_t rust_total_alloc = exp_rs_get_total_allocated();
  size_t rust_total_freed = exp_rs_get_total_freed();
  size_t rust_alloc_count = exp_rs_get_allocation_count();
  size_t rust_free_count = exp_rs_get_free_count();
  
  qemu_printf("%s:\n", label);
  qemu_printf("  Total allocated:     %d bytes\n", (int)rust_total_alloc);
  qemu_printf("  Total freed:         %d bytes\n", (int)rust_total_freed);
  qemu_printf("  Current allocated:   %d bytes\n", (int)rust_current);
  qemu_printf("  Allocation count:    %d\n", (int)rust_alloc_count);
  qemu_printf("  Free count:          %d\n", (int)rust_free_count);
  qemu_printf("  Net leaked:          %d bytes\n", (int)(rust_total_alloc - rust_total_freed));
  qemu_printf("  Count difference:    %d\n", (int)(rust_alloc_count - rust_free_count));
}

// Test 6: Memory stress test
void test_memory_stress(ExprContext *ctx) {
  qemu_printf("\n=== Test 6: Memory Stress Test ===\n");

  size_t start_allocated = current_allocated;

  qemu_printf("Running 20 allocation/free cycles...\n");
  print_heap_stats("Initial heap state");

  for (int i = 0; i < 20; i++) {
    // Vary the size to stress different allocation patterns
    size_t size = 1024 * (1 + (i % 8)); // 1KB to 8KB

    ExprBatch *batch = expr_batch_new(size);
    if (!batch) {
      qemu_printf("ERROR: Failed to create batch %d with size %d\n", i,
                  (int)size);
      break;
    }

    // Add some data using simple helper function (less memory intensive)
    int populate_result = populate_batch_simple(batch, ctx);
    if (!populate_result) {
      qemu_printf("ERROR: Failed to populate batch %d with test data\n", i);
    }

    // Free immediately
    expr_batch_free(batch);

    if (i % 5 == 0) {
      size_t rust_current = exp_rs_get_current_allocated();
      qemu_printf("  Iteration %d: sys_mem = %d bytes, rust_mem = %d bytes\n", i,
                  (int)current_allocated, (int)rust_current);
    }
  }

  qemu_printf("\nAfter first 20 iterations:\n");
  print_heap_stats("Heap state after 20 iterations");

  qemu_printf("\nRunning 80 more allocation/free cycles...\n");

  for (int i = 20; i < 100; i++) {
    // Vary the size to stress different allocation patterns
    size_t size = 1024 * (1 + (i % 8)); // 1KB to 8KB

    ExprBatch *batch = expr_batch_new(size);
    if (!batch) {
      qemu_printf("ERROR: Failed to create batch %d with size %d\n", i,
                  (int)size);
      break;
    }

    // Add some data using simple helper function (less memory intensive)
    int populate_result = populate_batch_simple(batch, ctx);
    if (!populate_result) {
      qemu_printf("ERROR: Failed to populate batch %d with test data\n", i);
    }

    // Free immediately
    expr_batch_free(batch);

    if (i % 10 == 0) {
      size_t rust_current = exp_rs_get_current_allocated();
      qemu_printf("  Iteration %d: sys_mem = %d bytes, rust_mem = %d bytes\n", i,
                  (int)current_allocated, (int)rust_current);
    }
  }

  qemu_printf("\nAfter all 100 iterations:\n");
  print_heap_stats("Final heap state after 100 iterations");

  size_t leaked = current_allocated - start_allocated;
  if (leaked > 0) {
    int avg_leak =
        (leaked > 0 && 100 > 0) ? (int)(leaked / 100) : 0;
    qemu_printf("WARNING: %d bytes leaked over %d cycles\n", (int)leaked,
                100);
    qemu_printf("Average leak per cycle: %d bytes\n", avg_leak);
  } else {
    qemu_printf("SUCCESS: No memory leak detected\n");
  }
}

// Main test runner
// Initialize the Rust heap allocator
extern int32_t exp_rs_heap_init(uintptr_t heap_size);
extern uintptr_t exp_rs_get_heap_size(void);
extern uintptr_t exp_rs_get_max_heap_size(void);

int main(void) {
  qemu_printf("\n");
  qemu_printf("========================================\n");
  qemu_printf("   exp-rs Batch Memory Test (QEMU)\n");
  qemu_printf("========================================\n");

  // Initialize the Rust heap allocator BEFORE any Rust code runs
  // Test with a specific heap size (2MB for testing)
  uintptr_t test_heap_size = 2 * 1024 * 1024; // 2MB
  int32_t init_result = exp_rs_heap_init(test_heap_size);
  if (init_result != 0) {
    qemu_printf("ERROR: Heap initialization failed with code %d\n", (int)init_result);
    qemu_printf("Max heap size: %d bytes\n", (int)exp_rs_get_max_heap_size());
    return -1;
  }
  uintptr_t actual_heap_size = exp_rs_get_heap_size();
  qemu_printf("Rust heap initialized: %d bytes (max: %d bytes)\n", 
              (int)actual_heap_size, (int)exp_rs_get_max_heap_size());
  
  // Verify heap size was set correctly
  if (actual_heap_size != test_heap_size) {
    qemu_printf("WARNING: Actual heap size (%d) differs from requested (%d)\n",
                (int)actual_heap_size, (int)test_heap_size);
  }

  // Reset tracking FIRST before any allocations
  total_allocated = 0;
  total_freed = 0;
  current_allocated = 0;
  peak_allocated = 0;
  allocation_count = 0;
  free_count = 0;

  // Initialize custom functions (this will be tracked now)
  ExprContext *ctx = expr_context_new();
  if (ctx) {
    register_test_math_functions(ctx);
  }

  // Run tests with shared context
  test_batch_lifecycle(ctx);
  test_multiple_batches(ctx);
  test_clear_and_reuse(ctx);
  test_batch_validity(ctx);
  test_static_batch_pointer(ctx);
  test_memory_stress(ctx);

  // Free context after all tests
  if (ctx) {
    expr_context_free(ctx);
  }

  // Dump any remaining allocations
  dump_remaining_allocations();

  // Final report
  qemu_printf("\n");
  qemu_printf("========================================\n");
  qemu_printf("           MEMORY SUMMARY\n");
  qemu_printf("========================================\n");
  
  // System malloc/free tracking (should be 0 with TlsfHeap)
  qemu_printf("System malloc tracking:\n");
  qemu_printf("  Total allocated:     %d bytes\n", (int)total_allocated);
  qemu_printf("  Total freed (est):   %d bytes\n", (int)total_freed);
  qemu_printf("  Peak allocated:      %d bytes\n", (int)peak_allocated);
  qemu_printf("  Allocation count:    %d\n", (int)allocation_count);
  qemu_printf("  Free count:          %d\n", (int)free_count);
  qemu_printf("  Current allocated:   %d bytes\n", (int)current_allocated);
  
  // Rust TlsfHeap tracking (should show real allocations)
  extern size_t exp_rs_get_total_allocated(void);
  extern size_t exp_rs_get_total_freed(void);  
  extern size_t exp_rs_get_free_count(void);
  
  size_t rust_current = exp_rs_get_current_allocated();
  size_t rust_total_alloc = exp_rs_get_total_allocated();
  size_t rust_total_freed = exp_rs_get_total_freed();
  size_t rust_alloc_count = exp_rs_get_allocation_count();
  size_t rust_free_count = exp_rs_get_free_count();
  
  qemu_printf("\nRust TlsfHeap tracking:\n");
  qemu_printf("  Total allocated:     %d bytes\n", (int)rust_total_alloc);
  qemu_printf("  Total freed:         %d bytes\n", (int)rust_total_freed);
  qemu_printf("  Current allocated:   %d bytes\n", (int)rust_current);
  qemu_printf("  Allocation count:    %d\n", (int)rust_alloc_count);
  qemu_printf("  Free count:          %d\n", (int)rust_free_count);

  // Check for leaks using Rust heap tracking (more accurate)
  if (rust_current > 0) {
    qemu_printf("\n*** MEMORY LEAK DETECTED: %d bytes in Rust heap ***\n",
                (int)rust_current);
    
    // Use new detailed allocation tracking functions to identify leaks
    extern size_t exp_rs_get_remaining_allocation_count(void);
    extern size_t exp_rs_get_remaining_allocations(struct CAllocationInfo* buffer, size_t buffer_size);
    
    size_t remaining_count = exp_rs_get_remaining_allocation_count();
    qemu_printf("Detailed allocation tracking found %d remaining allocations:\n", 
                (int)remaining_count);
    
    if (remaining_count > 0) {
      // Allocate buffer for allocation info (limit to reasonable number)  
      const size_t max_allocs = remaining_count < 100 ? remaining_count : 100;
      struct CAllocationInfo alloc_buffer[100];
      
      size_t copied = exp_rs_get_remaining_allocations(alloc_buffer, max_allocs);
      
      qemu_printf("\n=== DETAILED LEAK ANALYSIS ===\n");
      for (size_t i = 0; i < copied; i++) {
        qemu_printf("%d. %d bytes at line %d in %s (caller: 0x%08x, caller2: 0x%08x)\n",
                    (int)(i + 1),
                    (int)alloc_buffer[i].size,
                    alloc_buffer[i].line,
                    alloc_buffer[i].file_ptr ? alloc_buffer[i].file_ptr : "unknown",
                    (unsigned int)alloc_buffer[i].caller_addr,
                    (unsigned int)alloc_buffer[i].caller2_addr);
      }
      qemu_printf("=== END LEAK ANALYSIS ===\n\n");
    } else {
      qemu_printf("(No detailed allocation tracking available - build with --features alloc_tracking)\n");
    }
    
    qemu_exit(1); // Exit with failure
  } else if (current_allocated > 0) {
    qemu_printf("\n*** MEMORY LEAK DETECTED: %d bytes in system malloc ***\n",
                (int)current_allocated);
    qemu_exit(1); // Exit with failure
  } else {
    qemu_printf("\n*** ALL TESTS PASSED - NO LEAKS ***\n");
    qemu_exit(0); // Exit with success
  }

  return 0;
}
