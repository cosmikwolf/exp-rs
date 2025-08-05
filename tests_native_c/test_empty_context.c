#include "exp_rs.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef __APPLE__
#include <malloc/malloc.h>
#else
#include <malloc.h>
#endif

typedef struct {
  void *ptr;
  size_t size;
} Allocation;

#define MAX_ALLOCATIONS 1000
static Allocation allocations[MAX_ALLOCATIONS];
static size_t allocation_count = 0;
static size_t total_allocated = 0;

// Hook malloc to track memory usage
void *malloc_hook(size_t size) {
  void *ptr = malloc(size);
  if (ptr && allocation_count < MAX_ALLOCATIONS) {
    allocations[allocation_count].ptr = ptr;
    allocations[allocation_count].size = size;
    allocation_count++;
    total_allocated += size;
  }
  return ptr;
}

void free_hook(void *ptr) {
  if (ptr) {
    for (size_t i = 0; i < allocation_count; i++) {
      if (allocations[i].ptr == ptr) {
        total_allocated -= allocations[i].size;
        // Move last element to this position
        if (i < allocation_count - 1) {
          allocations[i] = allocations[allocation_count - 1];
        }
        allocation_count--;
        break;
      }
    }
  }
  free(ptr);
}

size_t get_context_memory_usage(ExprContext *ctx) {
  // Reset tracking
  allocation_count = 0;
  total_allocated = 0;

  // This is a rough estimate since we can't hook into Rust's allocator
  // We'll use the number of functions as a proxy for memory usage
  size_t native_count = expr_context_native_function_count(ctx);
  size_t expr_count = expr_context_expression_function_count(ctx);

  // Estimate: each function takes about 64 bytes (name + metadata)
  // Plus base context overhead of ~256 bytes
  return 256 + (native_count + expr_count) * 64;
}

int main() {
  printf("\n==== Empty Context Test ====\n\n");

  // Create empty context
  ExprContext *empty_ctx = expr_context_new_empty();
  assert(empty_ctx != NULL);

  // Create normal context
  ExprContext *normal_ctx = expr_context_new();
  assert(normal_ctx != NULL);

  // Count functions in each context
  size_t empty_native = expr_context_native_function_count(empty_ctx);
  size_t empty_expr = expr_context_expression_function_count(empty_ctx);
  size_t empty_total = empty_native + empty_expr;

  size_t normal_native = expr_context_native_function_count(normal_ctx);
  size_t normal_expr = expr_context_expression_function_count(normal_ctx);
  size_t normal_total = normal_native + normal_expr;

  // Print results
  printf("Empty Context:\n");
  printf("  Native functions:     %zu\n", empty_native);
  printf("  Expression functions: %zu\n", empty_expr);
  printf("  Total functions:      %zu\n", empty_total);
  printf("  Estimated memory:     ~%zu bytes\n",
         get_context_memory_usage(empty_ctx));

  printf("\nNormal Context:\n");
  printf("  Native functions:     %zu\n", normal_native);
  printf("  Expression functions: %zu\n", normal_expr);
  printf("  Total functions:      %zu\n", normal_total);
  printf("  Estimated memory:     ~%zu bytes\n",
         get_context_memory_usage(normal_ctx));

  printf("\nDifference:\n");
  printf("  Functions saved:      %zu\n", normal_total - empty_total);
  printf("  Memory saved:         ~%zu bytes\n",
         get_context_memory_usage(normal_ctx) -
             get_context_memory_usage(empty_ctx));

  printf("\n==== Test Passed ====\n\n");
  return 0;
}
