#include "qemu_test_harness.h"
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Include the generated header
#include "exp_rs.h"
#include "register_test_functions.h"

// Define common types and utilities for our tests
#if defined(DEF_USE_F32) || (defined(USE_F32) && !defined(USE_F64))
#define FABS fabsf
#define TEST_NAME "F32"
#define FORMAT_SPEC "%.6f"
#elif defined(DEF_USE_F64) || defined(USE_F64)
#define FABS fabs
#define TEST_NAME "F64"
#define FORMAT_SPEC "%.12f"
#else
#error "Neither USE_F32 nor USE_F64 is defined."
#endif

static int approx_eq(Real a, Real b, Real eps) {
  return FABS(a - b) < eps;
}

static test_result_t test_simple_eval(void) {
  qemu_printf("Testing basic batch FFI functions with %s mode\n", TEST_NAME);

  // Create context and batch
  struct ExprContext* ctx = create_test_context();
  if (!ctx) {
    qemu_print("Failed to create test context\n");
    return TEST_FAIL;
  }

  struct ExprBatch* batch = expr_batch_new(8192);
  if (!batch) {
    qemu_print("Failed to create batch\n");
    expr_context_free(ctx);
    return TEST_FAIL;
  }

  // Test basic arithmetic
  struct ExprResult eval_expr = expr_batch_add_expression(batch, "2+2*2");
  if (eval_expr.status != 0) {
    qemu_print("Failed to add expression\n");
    qemu_printf("Error: %s\n", eval_expr.error);
    expr_batch_free(batch);
    expr_context_free(ctx);
    return TEST_FAIL;
  }

  int32_t eval_status = expr_batch_evaluate(batch, ctx);
  if (eval_status != 0) {
    qemu_print("Failed to evaluate batch\n");
    expr_batch_free(batch);
    expr_context_free(ctx);
    return TEST_FAIL;
  }

  Real eval_value = expr_batch_get_result(batch, eval_expr.index);
  qemu_printf("Expression \"2+2*2\" = " FORMAT_SPEC "\n", eval_value);

  if (FABS(eval_value - 6.0) > TEST_PRECISION) {
    qemu_printf("Test failed: expected 6.0, got " FORMAT_SPEC "\n", eval_value);
    expr_batch_free(batch);
    expr_context_free(ctx);
    return TEST_FAIL;
  }

  qemu_print("Basic arithmetic test passed\n");

  // Clean up
  expr_batch_free(batch);
  expr_context_free(ctx);
  
  return TEST_PASS;
}

static test_result_t test_math_functions(void) {
  qemu_printf("Testing math functions with %s mode\n", TEST_NAME);

  struct ExprContext* ctx = create_test_context();
  if (!ctx) {
    qemu_print("Failed to create test context\n");
    return TEST_FAIL;
  }

  struct ExprBatch* batch = expr_batch_new(8192);
  if (!batch) {
    qemu_print("Failed to create batch\n");
    expr_context_free(ctx);
    return TEST_FAIL;
  }

  // Test sin function
  struct ExprResult sin_expr = expr_batch_add_expression(batch, "sin(0.5)");
  if (sin_expr.status != 0) {
    qemu_print("Failed to add sin expression\n");
    expr_batch_free(batch);
    expr_context_free(ctx);
    return TEST_FAIL;
  }

  if (expr_batch_evaluate(batch, ctx) != 0) {
    qemu_print("Failed to evaluate sin batch\n");
    expr_batch_free(batch);
    expr_context_free(ctx);
    return TEST_FAIL;
  }

  Real sin_result = expr_batch_get_result(batch, sin_expr.index);
  Real expected_sin = sin(0.5);
  qemu_printf("sin(0.5) = " FORMAT_SPEC " (expected " FORMAT_SPEC ")\n", sin_result, expected_sin);

  if (!approx_eq(sin_result, expected_sin, TEST_PRECISION)) {
    qemu_print("Sin test failed\n");
    expr_batch_free(batch);
    expr_context_free(ctx);
    return TEST_FAIL;
  }

  qemu_print("Math functions test passed\n");
  
  // Clean up
  expr_batch_free(batch);
  expr_context_free(ctx);
  
  return TEST_PASS;
}

test_result_t test_ffi(void) {
  qemu_printf("Testing FFI with %s precision\n\n", TEST_NAME);

  test_result_t simple_result = test_simple_eval();
  if (simple_result != TEST_PASS) {
    return simple_result;
  }

  test_result_t math_result = test_math_functions();
  if (math_result != TEST_PASS) {
    return math_result;
  }

  qemu_print("\nAll FFI tests passed!\n");
  return TEST_PASS;
}

// Test case definition
static const test_case_t tests[] = {
    {"ffi", test_ffi},
};

int main(void) {
    int failed = run_tests(tests, sizeof(tests) / sizeof(tests[0]));
    qemu_exit(failed ? EXIT_FAILURE : EXIT_SUCCESS);
    return failed ? 1 : 0;
}
