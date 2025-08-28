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

// Simple memory tracking for embedded
static size_t total_allocated = 0;
static size_t total_freed = 0;
static size_t current_allocated = 0;
static size_t peak_allocated = 0;
static size_t allocation_count = 0;
static size_t free_count = 0;

// Simple allocation tracking - just counters (no headers to avoid Bumpalo
// conflicts) Bumpalo overwrites any headers we add, so we use simple
// counter-based tracking

// Helper to show arena usage with detailed information
void show_arena_usage(ExprBatch *batch, const char *label) {
  if (!batch)
    return;

  uintptr_t arena_bytes = expr_batch_arena_bytes(batch);
  qemu_printf("%s: arena=%d bytes, sys_mem=%d bytes, sys_count=%d\n", label,
              (int)arena_bytes, (int)current_allocated, (int)allocation_count);
}
#define FFI_ERROR_NULL_POINTER -1
#define FFI_ERROR_INVALID_POINTER -5

// Custom allocation functions for tracking
// Must match the exact Rust FFI signatures and use standard calling convention
void *exp_rs_malloc(size_t size) {
  void *ptr = malloc(size);

  if (ptr) {
    // Simple counter-based tracking (no headers that Bumpalo can overwrite)
    total_allocated += size;
    current_allocated += size;
    allocation_count++;
    if (current_allocated > peak_allocated) {
      peak_allocated = current_allocated;
    }
    qemu_printf("[ALLOC] %d bytes at %p (total: %d, count: %d)\n", (int)size,
                ptr, (int)current_allocated, (int)allocation_count);
  }
  return ptr;
}

void exp_rs_free(void *ptr) {
  if (ptr) {
    free_count++;
    qemu_printf("[FREE] ptr %p (free count: %d)\n", ptr, (int)free_count);
    free(ptr);
    // Note: We can't accurately track freed bytes because Bumpalo overwrites
    // our tracking headers when it does internal bump allocation
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

// Test 6: Memory stress test
void test_memory_stress(ExprContext *ctx) {
  qemu_printf("\n=== Test 6: Memory Stress Test ===\n");

  size_t start_allocated = current_allocated;
  const int iterations = 20;

  qemu_printf("Running %d allocation/free cycles...\n", iterations);

  for (int i = 0; i < iterations; i++) {
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
      qemu_printf("  Iteration %d: current memory = %d bytes\n", i,
                  (int)current_allocated);
    }
  }

  qemu_printf("\nStress test complete\n");
  qemu_printf("Final memory: %d bytes\n", (int)current_allocated);

  size_t leaked = current_allocated - start_allocated;
  if (leaked > 0) {
    int avg_leak =
        (leaked > 0 && iterations > 0) ? (int)(leaked / iterations) : 0;
    qemu_printf("WARNING: %d bytes leaked over %d cycles\n", (int)leaked,
                iterations);
    qemu_printf("Average leak per cycle: %d bytes\n", avg_leak);
  } else {
    qemu_printf("SUCCESS: No memory leak detected\n");
  }
}

// Main test runner
int main(void) {
  qemu_printf("\n");
  qemu_printf("========================================\n");
  qemu_printf("   exp-rs Batch Memory Test (QEMU)\n");
  qemu_printf("========================================\n");

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

  // Final report
  qemu_printf("\n");
  qemu_printf("========================================\n");
  qemu_printf("           MEMORY SUMMARY\n");
  qemu_printf("========================================\n");
  qemu_printf("Total allocated:     %d bytes\n", (int)total_allocated);
  qemu_printf("Total freed (est):   %d bytes\n", (int)total_freed);
  qemu_printf("Peak allocated:      %d bytes\n", (int)peak_allocated);
  qemu_printf("Allocation count:    %d\n", (int)allocation_count);
  qemu_printf("Free count:          %d\n", (int)free_count);
  qemu_printf("Current allocated:   %d bytes\n", (int)current_allocated);

  if (current_allocated > 0) {
    qemu_printf("\n*** MEMORY LEAK DETECTED: %d bytes ***\n",
                (int)current_allocated);
    qemu_exit(1); // Exit with failure
  } else {
    qemu_printf("\n*** ALL TESTS PASSED - NO LEAKS ***\n");
    qemu_exit(0); // Exit with success
  }

  return 0;
}
