/*
 * Regression test for unbounded arena growth across expr_batch_clear().
 *
 * Why this exists alongside test_batch_clear_iterations.c: that test watches the *global
 * allocator*. bumpalo only calls the global allocator when it needs a new chunk, so with a
 * 16 KB size hint and a few KB of AST per load, the first several reloads grow the arena
 * completely invisibly to an allocator-traffic test. The growth only shows up at a chunk
 * boundary.
 *
 * This test watches expr_batch_arena_bytes() instead, which reads bumpalo's own cumulative
 * chunk capacity. It sees the growth on the second reload, not the seventh.
 *
 * The invariant: reloading identical content must leave the arena identically sized. If
 * expr_batch_clear() stops resetting the arena, this fails on iteration 2.
 */

#include "exp_rs.h"
#include <assert.h>
#include <stdio.h>

/* Deliberately small, so growth crosses a chunk boundary fast if the reset regresses. */
#define ARENA_HINT     2048
#define ITERATIONS     20

/* Content is identical on every iteration, so the arena size must be too.
 *
 * Deliberately arithmetic only: the native test build registers no math functions, and
 * this test is about arena lifetime, not function coverage. The expression function below
 * covers the part that needs a callable — it is also what populates expr_func_cache, whose
 * arena references are the other half of what a missing reset leaves behind. */
static const char *const EXPRESSIONS[] = {
    "shape(t) * power",
    "t * speed + channel_id",
    "(t_ms / 1000.0) + multi_adjust",
    "shape(t + 0.25) - shape(t - 0.25)",
};
#define EXPRESSION_COUNT (sizeof(EXPRESSIONS) / sizeof(EXPRESSIONS[0]))

static const char *const VARIABLES[] = {
    "t", "t_ms", "power", "multi_adjust", "speed", "channel_id",
};
#define VARIABLE_COUNT (sizeof(VARIABLES) / sizeof(VARIABLES[0]))

/* Rebuild the batch contents exactly as a pattern load does: variables, then the
 * expression function, then the expressions that call it. */
static void load_content(ExprBatch *batch)
{
    for (size_t i = 0; i < VARIABLE_COUNT; i++) {
        ExprResult r = expr_batch_add_variable(batch, VARIABLES[i], 0.5);
        assert(r.status == 0 && "failed to add variable");
    }

    int fn = expr_batch_add_expression_function(batch, "shape", "x", "x * x * 4.0 - x");
    assert(fn == 0 && "failed to add expression function");

    for (size_t i = 0; i < EXPRESSION_COUNT; i++) {
        ExprResult r = expr_batch_add_expression(batch, EXPRESSIONS[i]);
        assert(r.status == 0 && "failed to add expression");
    }
}

int main(void)
{
    printf("=== Batch Clear Arena Stability Test ===\n\n");

    ExprContext *ctx = expr_context_new();
    if (!ctx) {
        printf("ERROR: Failed to create context\n");
        return 1;
    }

    ExprBatch *batch = expr_batch_new(ARENA_HINT);
    if (!batch) {
        printf("ERROR: Failed to create batch\n");
        expr_context_free(ctx);
        return 1;
    }

    /* Warm-up load. The batch starts with only the size hint, so the first load makes
     * bumpalo add chunks and land on its final chunk size. Measuring from cold would
     * compare a growing arena against a settled one and fail for the wrong reason.
     * Steady state begins after the first clear. */
    load_content(batch);
    {
        ExprResult warm = expr_batch_evaluate_ex(batch, ctx);
        if (warm.status != 0) {
            printf("ERROR: warm-up evaluation failed: status=%d %s\n", warm.status,
                   warm.error);
            expr_batch_free(batch);
            expr_context_free(ctx);
            return 1;
        }
    }
    expr_batch_clear(batch);

    size_t baseline = 0;
    int failures = 0;

    for (int i = 0; i < ITERATIONS; i++) {
        load_content(batch);

        /* Evaluate too — evaluation populates expr_func_cache and the engine stacks,
         * which also live in the arena. A reset that skipped them would show up here. */
        ExprResult eval = expr_batch_evaluate_ex(batch, ctx);
        if (eval.status != 0) {
            printf("ERROR: evaluation failed on iteration %d: status=%d %s\n", i,
                   eval.status, eval.error);
            expr_batch_free(batch);
            expr_context_free(ctx);
            return 1;
        }

        size_t bytes = expr_batch_arena_bytes(batch);

        if (i == 0) {
            baseline = bytes;
            printf("  iteration %2d: arena = %zu bytes  (baseline)\n", i, bytes);
        } else {
            printf("  iteration %2d: arena = %zu bytes  (%+zd)\n", i, bytes,
                   (ssize_t)bytes - (ssize_t)baseline);
            if (bytes != baseline) {
                failures++;
            }
        }

        expr_batch_clear(batch);
    }

    printf("\n");
    if (failures == 0) {
        printf("PASS: arena stayed at %zu bytes across %d reloads\n", baseline, ITERATIONS);
    } else {
        printf("FAIL: arena size changed on %d of %d reloads — expr_batch_clear is not "
               "resetting the arena\n",
               failures, ITERATIONS - 1);
    }

    expr_batch_free(batch);
    expr_context_free(ctx);

    assert(failures == 0
           && "expr_batch_clear must leave the arena identically sized across reloads");

    printf("\n=== Test Complete ===\n");
    return failures == 0 ? 0 : 1;
}
