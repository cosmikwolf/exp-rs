/**
 * Batch evaluation performance benchmark for the expr_batch_* API.
 *
 * Measures, in CMSDK timer ticks:
 *   1. Batch setup (arena + parse of 6 expressions, 10 variables)
 *   2. Pure evaluation of the batch
 *   3. Parameter update only
 *   4. Full cycle: parameter update + evaluation
 *
 * Then verifies every expression result against values computed in C.
 *
 * QEMU is not cycle-accurate. Use these numbers only to compare builds
 * against each other.
 */
#include "exp_rs.h"
#include "qemu_test_harness.h"
#include "register_test_functions.h"
#include <math.h>
#include <stdint.h>
#include <string.h>

#define NUM_PARAMETERS 10
#define NUM_EXPRESSIONS 6
#define SETUP_ITERATIONS 100
#define EVAL_ITERATIONS 2000
#define PARAM_ITERATIONS 2000
#define CYCLE_ITERATIONS 1000
#define ARENA_SIZE 32768

#if defined(DEF_USE_F32)
#define VERIFY_EPSILON 1e-4
#else
#define VERIFY_EPSILON 1e-9
#endif

// The Rust library uses its own TlsfHeap; give it memory before first use.
extern int32_t exp_rs_heap_init(uint8_t *heap_ptr, uintptr_t heap_size);
static uint8_t heap_memory[2 * 1024 * 1024];

// Accumulate results so the compiler cannot remove the evaluation calls.
static volatile Real g_sink = 0;

static const char *expressions[NUM_EXPRESSIONS] = {
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

static const char *param_names[NUM_PARAMETERS] = {"a", "b", "c", "d", "e",
                                                  "f", "g", "h", "i", "j"};

static Real param_base(int p) { return (Real)((p + 1) * 1.5); }

// C reference implementations of the 6 expressions, computed in double.
static double c_sign(double x) { return (x > 0) - (x < 0); }

static double expected_value(int e, const double *v) {
  double a = v[0], b = v[1], c = v[2], d = v[3], ee = v[4];
  double f = v[5], g = v[6], h = v[7], i = v[8], j = v[9];
  switch (e) {
  case 0:
    return a * sin(b * 3.14159 / 180) + c * cos(d * 3.14159 / 180) +
           sqrt(ee * ee + f * f);
  case 1:
    return exp(g / 10) * log(h + 1) + pow(i, 0.5) * j;
  case 2:
    return (double)((a > 5) && (b < 10)) * c +
           (double)((d >= ee) || (f != g)) * h + (i < j ? i : j);
  case 3:
    return sqrt(pow(a - ee, 2) + pow(b - f, 2)) +
           atan2(c - g, d - h) * (i + j) / 2;
  case 4:
    return fabs(a - b) * c_sign(c - d) + (ee > f ? ee : f) * (g < h ? g : h) +
           fmod(i * j, 10);
  case 5:
    return (a + b + c) / 3 * sin((d + ee + f) * 3.14159 / 6) +
           log10(g * h + 1) - exp(-i * j / 100);
  default:
    return 0;
  }
}

static int build_batch(ExprBatch *batch) {
  for (int p = 0; p < NUM_PARAMETERS; p++) {
    ExprResult r = expr_batch_add_variable(batch, param_names[p], param_base(p));
    if (r.status != 0) {
      qemu_printf("FAIL: add_variable %s: %s\n", param_names[p], r.error);
      return 0;
    }
  }
  for (int e = 0; e < NUM_EXPRESSIONS; e++) {
    ExprResult r = expr_batch_add_expression(batch, expressions[e]);
    if (r.status != 0) {
      qemu_printf("FAIL: add_expression %d: %s\n", e, r.error);
      return 0;
    }
  }
  return 1;
}

test_result_t test_batch_performance(void) {
  qemu_printf("\n=== Batch Evaluation Performance (expr_batch API) ===\n");
  qemu_printf("Parameters: %d, Expressions: %d, Real size: %d bytes\n",
              NUM_PARAMETERS, NUM_EXPRESSIONS, (int)sizeof(Real));

  ExprContext *ctx = create_test_context();
  if (!ctx) {
    qemu_printf("FAIL: cannot create context\n");
    return TEST_FAIL;
  }

  init_hardware_timer();

  // --- Test 1: setup cost (batch creation + variables + parse) ---
  qemu_printf("\nTest 1: Batch setup (new + %d vars + %d exprs), %d rounds\n",
              NUM_PARAMETERS, NUM_EXPRESSIONS, SETUP_ITERATIONS);
  benchmark_start();
  for (int i = 0; i < SETUP_ITERATIONS; i++) {
    ExprBatch *b = expr_batch_new(ARENA_SIZE);
    if (!b || !build_batch(b)) {
      qemu_printf("FAIL: setup round %d\n", i);
      expr_batch_free(b);
      expr_context_free(ctx);
      return TEST_FAIL;
    }
    expr_batch_free(b);
  }
  uint32_t setup_ticks = benchmark_stop();
  qemu_printf("  Total: %u ticks, per setup: %u ticks\n", setup_ticks,
              setup_ticks / SETUP_ITERATIONS);

  // --- Persistent batch for the evaluation tests ---
  ExprBatch *batch = expr_batch_new(ARENA_SIZE);
  if (!batch || !build_batch(batch)) {
    qemu_printf("FAIL: cannot build persistent batch\n");
    expr_batch_free(batch);
    expr_context_free(ctx);
    return TEST_FAIL;
  }

  // Warm up
  if (expr_batch_evaluate(batch, ctx) != 0) {
    qemu_printf("FAIL: warmup evaluation\n");
    expr_batch_free(batch);
    expr_context_free(ctx);
    return TEST_FAIL;
  }

  // --- Test 2: pure evaluation ---
  qemu_printf("\nTest 2: Pure evaluation, %d iterations\n", EVAL_ITERATIONS);
  reset_timer();
  benchmark_start();
  for (int i = 0; i < EVAL_ITERATIONS; i++) {
    expr_batch_evaluate(batch, ctx);
    for (int e = 0; e < NUM_EXPRESSIONS; e++) {
      g_sink += expr_batch_get_result(batch, e);
    }
  }
  uint32_t eval_ticks = benchmark_stop();
  qemu_printf("  Total: %u ticks, per batch: %u ticks, per expression: %u "
              "ticks\n",
              eval_ticks, eval_ticks / EVAL_ITERATIONS,
              eval_ticks / (EVAL_ITERATIONS * NUM_EXPRESSIONS));

  // --- Test 3: parameter update only ---
  qemu_printf("\nTest 3: Parameter update only (%d params), %d iterations\n",
              NUM_PARAMETERS, PARAM_ITERATIONS);
  reset_timer();
  benchmark_start();
  for (int i = 0; i < PARAM_ITERATIONS; i++) {
    for (int p = 0; p < NUM_PARAMETERS; p++) {
      expr_batch_set_variable(batch, p, param_base(p) + (Real)i * (Real)0.001);
    }
  }
  uint32_t param_ticks = benchmark_stop();
  qemu_printf("  Total: %u ticks, per update cycle: %u ticks, per parameter: "
              "%u ticks\n",
              param_ticks, param_ticks / PARAM_ITERATIONS,
              param_ticks / (PARAM_ITERATIONS * NUM_PARAMETERS));

  // --- Test 4: full cycle (update + evaluate) ---
  qemu_printf("\nTest 4: Full cycle (update %d params + evaluate), %d "
              "iterations\n",
              NUM_PARAMETERS, CYCLE_ITERATIONS);
  reset_timer();
  benchmark_start();
  for (int i = 0; i < CYCLE_ITERATIONS; i++) {
    for (int p = 0; p < NUM_PARAMETERS; p++) {
      expr_batch_set_variable(batch, p, param_base(p) + (Real)i * (Real)0.001);
    }
    expr_batch_evaluate(batch, ctx);
    for (int e = 0; e < NUM_EXPRESSIONS; e++) {
      g_sink += expr_batch_get_result(batch, e);
    }
  }
  uint32_t cycle_ticks = benchmark_stop();
  qemu_printf("  Total: %u ticks, per cycle: %u ticks\n", cycle_ticks,
              cycle_ticks / CYCLE_ITERATIONS);

  // --- Verification against C reference values ---
  qemu_printf("\nVerifying results against C reference...\n");
  int verification_passed = 1;
  double check_sets[3] = {0.0, 1.7, 4.2};
  for (int s = 0; s < 3; s++) {
    double v[NUM_PARAMETERS];
    for (int p = 0; p < NUM_PARAMETERS; p++) {
      v[p] = (double)param_base(p) + check_sets[s];
      expr_batch_set_variable(batch, p, (Real)v[p]);
    }
    int status = expr_batch_evaluate(batch, ctx);
    if (status != 0) {
      qemu_printf("  FAIL: evaluation status %d in set %d\n", status, s);
      verification_passed = 0;
      continue;
    }
    for (int e = 0; e < NUM_EXPRESSIONS; e++) {
      double got = (double)expr_batch_get_result(batch, e);
      double expected = expected_value(e, v);
      double tolerance = VERIFY_EPSILON * (1.0 + fabs(expected));
      if (fabs(got - expected) > tolerance) {
        qemu_printf("  FAIL: set %d expr %d: got %.9f, expected %.9f\n", s, e,
                    got, expected);
        qemu_printf("    Expression: %s\n", expressions[e]);
        verification_passed = 0;
      }
    }
  }
  if (verification_passed) {
    qemu_printf("  PASS: all %d checks match\n", 3 * NUM_EXPRESSIONS);
  }

  qemu_printf("\n=== Summary (ticks) ===\n");
  qemu_printf("Setup per batch:        %u\n", setup_ticks / SETUP_ITERATIONS);
  qemu_printf("Eval per batch:         %u\n", eval_ticks / EVAL_ITERATIONS);
  qemu_printf("Eval per expression:    %u\n",
              eval_ticks / (EVAL_ITERATIONS * NUM_EXPRESSIONS));
  qemu_printf("Param update per cycle: %u\n", param_ticks / PARAM_ITERATIONS);
  qemu_printf("Full cycle:             %u\n", cycle_ticks / CYCLE_ITERATIONS);

  expr_batch_free(batch);
  expr_context_free(ctx);

  return verification_passed ? TEST_PASS : TEST_FAIL;
}

int main(void) {
  qemu_printf("=== Batch Processing Performance Test ===\n");

  if (exp_rs_heap_init(heap_memory, sizeof(heap_memory)) != 0) {
    qemu_printf("ERROR: heap initialization failed\n");
    qemu_exit(1);
  }

  test_result_t result = test_batch_performance();

  // Read the sink so the accumulation loops stay observable.
  qemu_printf("\n(Optimization prevention: %d)\n", (int)g_sink);

  if (result == TEST_PASS) {
    qemu_printf("\nTest PASSED\n");
    qemu_exit(0);
  } else {
    qemu_printf("\nTest FAILED\n");
    qemu_exit(1);
  }

  return 0;
}
