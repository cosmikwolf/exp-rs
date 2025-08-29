#include "exp_rs.h"
#include "qemu_test_harness.h"
#include "register_test_functions.h"
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Test configuration
#define NUM_PARAMETERS 10
#define NUM_EXPRESSIONS 6
#define NUM_ITERATIONS 100
#define BATCH_SIZE 50

// Helper function to check if two doubles are approximately equal
static int approx_equal(double a, double b) { return fabs(a - b) < 1e-10; }

test_result_t test_batch_performance(void) {
  qemu_printf("\n=== Batch Processing Performance Test ===\n");
  qemu_printf("Parameters: %d, Expressions: %d\n", NUM_PARAMETERS,
              NUM_EXPRESSIONS);
  qemu_printf("Iterations: %d, Batch size: %d\n", NUM_ITERATIONS, BATCH_SIZE);
  qemu_printf("Total evaluations: %d\n\n",
              NUM_EXPRESSIONS * NUM_ITERATIONS * BATCH_SIZE);

  // Create moderately complex expressions using all 10 parameters
  const char *expressions[NUM_EXPRESSIONS] = {
      // Expression 1: Mixed arithmetic and trig
      "a*sin(b*3.14159/180) + c*cos(d*3.14159/180) + sqrt(e*e + f*f)",

      // Expression 2: Exponential and logarithmic
      "exp(g/10) * log(h+1) + pow(i, 0.5) * j",

      // Expression 3: Conditional and comparison
      "((a > 5) && (b < 10)) * c + ((d >= e) || (f != g)) * h + min(i, j)",

      // Expression 4: Nested functions
      "sqrt(pow(a-e, 2) + pow(b-f, 2)) + atan2(c-g, d-h) * (i+j)/2",

      // Expression 5: Mathematical operations
      "abs(a-b) * sign(c-d) + max(e, f) * min(g, h) + fmod(i*j, 10)",

      // Expression 6: Combined operations
      "(a+b+c)/3 * sin((d+e+f)*3.14159/6) + log10(g*h+1) - exp(-i*j/100)"};

  // Parameter names
  const char *param_names[NUM_PARAMETERS] = {"a", "b", "c", "d", "e",
                                             "f", "g", "h", "i", "j"};

  // Generate test data for batch processing
  // Note: FFI expects param_values[param_idx][batch_idx]
  // So we organize as param_arrays[param][batch]
  double **param_arrays = (double **)malloc(NUM_PARAMETERS * sizeof(double *));
  for (int p = 0; p < NUM_PARAMETERS; p++) {
    param_arrays[p] = (double *)malloc(BATCH_SIZE * sizeof(double));
    for (int b = 0; b < BATCH_SIZE; b++) {
      // Generate varied but deterministic test data
      param_arrays[p][b] = (p + 1) * 1.5 + (b + 1) * 0.1;
    }
  }

  // Create context with registered math functions
  void *ctx = create_test_context();
  if (!ctx) {
    qemu_printf("FAIL: Failed to create context\n");
    return TEST_FAIL;
  }

  // Warm up the cache
  qemu_printf("Warming up...\n");
  for (int p = 0; p < NUM_PARAMETERS; p++) {
    exp_rs_context_set_parameter(ctx, param_names[p], param_arrays[p][0]);
  }
  for (int e = 0; e < NUM_EXPRESSIONS; e++) {
    struct EvalResult result = exp_rs_context_eval(expressions[e], ctx);
    if (result.status != 0) {
      qemu_printf("Warmup failed for expression %d\n", e);
      if (result.error) {
        qemu_printf("Error: %s\n", result.error);
        exp_rs_free_error(result.error);
      }
    }
  }

  // Test 1: Individual evaluation
  qemu_printf("\nTest 1: Individual evaluation\n");

  init_hardware_timer();

  benchmark_start();

  for (int iter = 0; iter < NUM_ITERATIONS; iter++) {
    for (int batch = 0; batch < BATCH_SIZE; batch++) {
      // Set parameters for this batch item
      for (int p = 0; p < NUM_PARAMETERS; p++) {
        exp_rs_context_set_parameter(ctx, param_names[p],
                                     param_arrays[p][batch]);
      }

      // Evaluate all expressions
      for (int e = 0; e < NUM_EXPRESSIONS; e++) {
        struct EvalResult result = exp_rs_context_eval(expressions[e], ctx);
        if (result.status != 0) {
          qemu_printf("Error in expression %d, iter %d, batch %d\n", e, iter,
                      batch);
          if (result.error) {
            exp_rs_free_error(result.error);
          }
        }
      }
    }
  }

  uint32_t individual_ticks = benchmark_stop();
  qemu_printf("  Ticks: %u\n", individual_ticks);
  qemu_printf("  Total evaluations: %d\n",
              NUM_EXPRESSIONS * NUM_ITERATIONS * BATCH_SIZE);

  // Test 2: Batch evaluation
  qemu_printf("\nTest 2: Batch evaluation with engine reuse\n");

  // Reset timer between tests
  reset_timer();

  // Pre-allocate results
  double **results = (double **)calloc(NUM_EXPRESSIONS, sizeof(double *));
  for (int i = 0; i < NUM_EXPRESSIONS; i++) {
    results[i] = (double *)calloc(BATCH_SIZE, sizeof(double));
  }

  BatchEvalRequest request = {
      .expressions = expressions,
      .expression_count = NUM_EXPRESSIONS,
      .param_names = param_names,
      .param_count = NUM_PARAMETERS,
      .param_values =
          (const double **)param_arrays, // Use the correctly laid out array
      .batch_size = BATCH_SIZE,
      .results = results,
      .stop_on_error = 0,
      .statuses = NULL};

  // Measure batch evaluation time
  benchmark_start();

  for (int iter = 0; iter < NUM_ITERATIONS; iter++) {
    int status = exp_rs_batch_eval(&request, ctx);
    if (status != 0) {
      qemu_printf("Batch evaluation failed with status %d in iteration %d\n",
                  status, iter);
    }
  }

  uint32_t batch_ticks = benchmark_stop();
  qemu_printf("  Ticks: %u\n", batch_ticks);
  qemu_printf("  Total evaluations: %d\n",
              NUM_EXPRESSIONS * NUM_ITERATIONS * BATCH_SIZE);

  // Debug: Print first few batch results and expected values
  qemu_printf("\nDebug - First few batch results:\n");
  for (int b = 0; b < 3; b++) {
    qemu_printf("Batch %d: expr0=%.3f, expr2=%.3f\n", b, results[0][b],
                results[2][b]);
    // Calculate expected value for expression 0 manually
    double a = param_arrays[0][b];
    double b_val = param_arrays[1][b];
    double c = param_arrays[2][b];
    double d = param_arrays[3][b];
    double e = param_arrays[4][b];
    double f = param_arrays[5][b];
    double expected = a * sin(b_val * 3.14159 / 180) +
                      c * cos(d * 3.14159 / 180) + sqrt(e * e + f * f);
    qemu_printf("  Expected expr0: %.3f (a=%.1f, b=%.1f, c=%.1f, d=%.1f, "
                "e=%.1f, f=%.1f)\n",
                expected, a, b_val, c, d, e, f);
  }

  // Calculate performance improvement
  double speedup = (double)individual_ticks / batch_ticks;
  double improvement =
      ((double)individual_ticks - batch_ticks) / individual_ticks * 100.0;

  qemu_printf("\n=== Performance Results ===\n");
  qemu_printf("Individual evaluation: %u ticks\n", individual_ticks);
  qemu_printf("Batch evaluation: %u ticks\n", batch_ticks);
  qemu_printf("Speedup: %.2fx faster\n", speedup);
  qemu_printf("Performance improvement: %.1f%%\n", improvement);

  // Verify correctness by spot-checking some results
  qemu_printf("\nVerifying results...\n");
  int verification_passed = 1;

  // Check a few random samples
  for (int sample = 0; sample < 3; sample++) {
    int batch_idx = sample * BATCH_SIZE / 3;

    // Debug: Print parameter values and check what batch evaluation actually
    // used
    if (sample == 0) {
      qemu_printf("\nDebug - Parameters for batch %d:\n", batch_idx);
      qemu_printf("Individual uses: a=%.3f, Batch result suggests a≈%.3f\n",
                  param_arrays[0][batch_idx],
                  results[2][batch_idx] -
                      4.0); // Expression 2 is "((a > 5) && (b < 10)) * c +
                            // ...", so if a>5, result includes c
      for (int p = 0; p < 3; p++) {
        qemu_printf("  %s = %.3f\n", param_names[p],
                    param_arrays[p][batch_idx]);
      }
    }

    // Set parameters for verification
    for (int p = 0; p < NUM_PARAMETERS; p++) {
      exp_rs_context_set_parameter(ctx, param_names[p],
                                   param_arrays[p][batch_idx]);
    }

    // Check each expression
    for (int e = 0; e < NUM_EXPRESSIONS; e++) {
      struct EvalResult individual = exp_rs_context_eval(expressions[e], ctx);
      double batch_result = results[e][batch_idx];

      // Skip comparison if individual evaluation failed
      if (individual.status != 0) {
        if (batch_result != 0.0) {
          qemu_printf("  FAIL: Individual failed but batch succeeded for expr "
                      "%d, batch %d\n",
                      e, batch_idx);
          verification_passed = 0;
        }
        continue;
      }

      if (!approx_equal(individual.value, batch_result)) {
        qemu_printf("  FAIL: Mismatch in expr %d, batch %d: individual=%.6f, "
                    "batch=%.6f\n",
                    e, batch_idx, individual.value, batch_result);
        if (sample == 0 && e == 0) {
          qemu_printf("  Expression: %s\n", expressions[e]);
        }
        verification_passed = 0;
      }
    }
  }

  if (verification_passed) {
    qemu_printf("  PASS: All checked results match!\n");
  }

  // Cleanup
  for (int i = 0; i < NUM_EXPRESSIONS; i++) {
    free(results[i]);
  }
  free(results);

  // Free parameter arrays
  for (int p = 0; p < NUM_PARAMETERS; p++) {
    free(param_arrays[p]);
  }
  free(param_arrays);

  exp_rs_context_free(ctx);

  // Determine pass/fail based on performance and correctness
  if (!verification_passed) {
    qemu_printf("\nFAIL: Results do not match\n");
    return TEST_FAIL;
  }

  if (improvement < 10.0) {
    qemu_printf("\nWARNING: Performance improvement less than 10%%\n");
  } else if (improvement >= 30.0 && improvement <= 60.0) {
    qemu_printf(
        "\nEXCELLENT: Performance improvement in target range (30-60%%)\n");
  }

  return TEST_PASS;
}

// Test case definition
static test_case_t batch_perf_test = {.name = "batch_performance",
                                      .func = test_batch_performance};

// Main function for standalone execution
int main(void) {
  qemu_printf("=== Batch Processing Performance Test ===\n");

  test_result_t result = test_batch_performance();

  if (result == TEST_PASS) {
    qemu_printf("\nTest PASSED\n");
    qemu_exit(0);
  } else {
    qemu_printf("\nTest FAILED\n");
    qemu_exit(1);
  }

  return 0;
}
