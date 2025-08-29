#include "exp_rs.h"
#include "common_allocator.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Simple native function for testing
static Real test_double_function(const Real *args, uintptr_t n_args) {
  return args[0] * 2.0;
}

// Helper function to track and print memory change
static void track_memory(const char *step_name, memory_stats_t *baseline) {
  memory_stats_t current = get_memory_stats();
  printf("Memory after %s: %zu bytes (+%zu)\n", step_name,
         current.current_bytes,
         current.current_bytes - baseline->current_bytes);
  *baseline = current;
}

// Helper function to cleanup and exit on error
static void cleanup_and_exit(ExprBatch *batch,
                             ExprContext *ctx, const char *error_msg) {
  printf("ERROR: %s\n", error_msg);
  if (batch)
    expr_batch_free(batch);
  if (ctx)
    expr_context_free(ctx);
  exit(1);
}

// Helper function to handle ExprResult errors
static void handle_expr_result(ExprResult result, const char *operation,
                               ExprBatch *batch,
                               ExprContext *ctx) {
  if (result.status != 0) {
    printf("ERROR: %s failed: %s\n", operation, result.error);
    cleanup_and_exit(batch, ctx, "ExprResult failure");
  }
}

// Helper function to handle integer result errors
static void handle_int_result(int32_t result, const char *operation,
                              ExprBatch *batch,
                              ExprContext *ctx) {
  if (result != 0) {
    printf("ERROR: %s failed with error code: %d\n", operation, result);
    cleanup_and_exit(batch, ctx, "Operation failure");
  }
}

int main() {
  printf("=== Batch Mode Memory Tracking Test ===\n");

  // Initialize memory tracking
  init_memory_tracking();
  enable_allocation_tracking();
  reset_memory_stats();
  memory_stats_t baseline = get_memory_stats();
  printf("Initial memory: %zu bytes\n", baseline.current_bytes);

  // 1. Create context
  printf("\n1. Creating context...\n");
  ExprContext *ctx = expr_context_new();
  if (!ctx)
    cleanup_and_exit(NULL, NULL, "Failed to create context");
  track_memory("context creation", &baseline);

  // 2. Create arena and batch
  printf("\n2. Creating arena and batch...\n");
  // Create batch with integrated arena (8KB)
  ExprBatch *batch = expr_batch_new(8192);
  if (!batch)
    cleanup_and_exit(NULL, ctx, "Failed to create batch");
  track_memory("batch/arena creation", &baseline);

  // 3. Register native function
  printf("\n3. Registering native function 'double_it'...\n");
  int32_t result =
      expr_context_add_function(ctx, "double_it", 1, test_double_function);
  handle_int_result(result, "native function registration", batch, ctx);
  track_memory("native function registration", &baseline);

  printf("\n3.5 Registering native function 'double_it'... a second time\n");
  result = expr_context_add_function(ctx, "double_it", 1, test_double_function);
  handle_int_result(result, "native function registration", batch, ctx);
  track_memory("native function registration", &baseline);

  // 4. Try to add expression function with same name (should fail)
  printf("\n4. Trying to register expression function with same name "
         "'double_it'...\n");
  int32_t expr_fn_result =
      expr_batch_add_expression_function(batch, "double_it", "y", "y * 3");
  if (expr_fn_result == 0) {
    printf("WARNING: Expected failure but expression function was registered "
           "successfully\n");
    track_memory("expression function registration (unexpected)", &baseline);
  } else {
    printf("Expected failure with error code: %d\n", expr_fn_result);
    memory_stats_t current = get_memory_stats();
    printf("Memory unchanged: %zu bytes\n", current.current_bytes);
  }

  printf("\n4.5 Trying to register expression function with same name "
         "'double_it' a second time...\n");
  expr_fn_result =
      expr_batch_add_expression_function(batch, "double_it", "y", "y * 3");
  if (expr_fn_result == 0) {
    printf("WARNING: Expected failure but expression function was registered "
           "successfully\n");
    track_memory("expression function registration (unexpected)", &baseline);
  } else {
    printf("Expected failure with error code: %d\n", expr_fn_result);
    memory_stats_t current = get_memory_stats();
    printf("Memory unchanged: %zu bytes\n", current.current_bytes);
  }

  // 11. Reset arena
  printf("\n11. Resetting arena...\n");
  // Arena reset is no longer available with integrated arena
  memory_stats_t after_reset = get_memory_stats();
  printf("Memory after arena reset: %zu bytes (%+ld)\n",
         after_reset.current_bytes,
         (long)(after_reset.current_bytes - baseline.current_bytes));
  baseline = after_reset;

  // Check if expressions were cleared
  printf("Checking if expressions were cleared...\n");
  int32_t eval3_result = expr_batch_evaluate(batch, ctx);
  if (eval3_result != 0) {
    printf("Expected: Batch evaluation failed after reset, error code: %d\n",
           eval3_result);
  } else {
    printf("WARNING: Batch evaluation unexpectedly succeeded after reset\n");
  }

  // 12. Re-add expressions after reset
  printf("\n12. Re-adding expressions after reset...\n");
  ExprResult expr3_result = expr_batch_add_expression(batch, "double_it(x)");
  handle_expr_result(expr3_result, "expression re-addition after reset", batch,
                     ctx);
  track_memory("expression re-addition after reset", &baseline);

  // Cleanup
  expr_batch_free(batch);
  // Arena is freed with batch
  expr_context_free(ctx);

  print_memory_stats("Final");
  printf("Test completed successfully!\n");
}
