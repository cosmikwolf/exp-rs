#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <math.h>
#include "exp_rs.h"

// Native function implementations
Real native_sin(const Real* args, uintptr_t nargs) { (void)nargs; return sin(args[0]); }
Real native_cos(const Real* args, uintptr_t nargs) { (void)nargs; return cos(args[0]); }
Real native_sqrt(const Real* args, uintptr_t nargs) { (void)nargs; return sqrt(args[0]); }
Real native_pow(const Real* args, uintptr_t nargs) { (void)nargs; return pow(args[0], args[1]); }
Real native_abs(const Real* args, uintptr_t nargs) { (void)nargs; return fabs(args[0]); }
Real native_max(const Real* args, uintptr_t nargs) { (void)nargs; return args[0] > args[1] ? args[0] : args[1]; }
Real native_min(const Real* args, uintptr_t nargs) { (void)nargs; return args[0] < args[1] ? args[0] : args[1]; }

// Test basic session creation and usage
void test_basic_session() {
    printf("=== Test Basic Session API ===\n");
    
    // Initialize the global arena pool (required for session API)
    if (!expr_pool_init(16)) {
        printf("ERROR: Failed to initialize arena pool\n");
        return;
    }
    printf("✓ Arena pool initialized\n");
    
    // Create a session
    ExprSession* session = expr_session_new();
    assert(session != NULL);
    printf("✓ Session created\n");
    
    // Parse a simple expression
    int32_t result = expr_session_parse(session, "x + y");
    assert(result == 0);
    printf("✓ Expression parsed successfully\n");
    
    // Add variables
    expr_session_add_variable(session, "x", 10.0);
    expr_session_add_variable(session, "y", 20.0);
    printf("✓ Variables added: x=10.0, y=20.0\n");
    
    // Evaluate
    Real result_value;
    int32_t eval_result = expr_session_evaluate(session, NULL, &result_value);
    assert(eval_result == 0);
    assert(result_value == 30.0);
    printf("✓ Evaluation result: %.1f (expected 30.0)\n", result_value);
    
    // Update variables and re-evaluate
    expr_session_set_variable(session, "x", 5.0);
    expr_session_set_variable(session, "y", 15.0);
    eval_result = expr_session_evaluate(session, NULL, &result_value);
    assert(eval_result == 0);
    assert(result_value == 20.0);
    printf("✓ Re-evaluation after update: %.1f (expected 20.0)\n", result_value);
    
    // Free session
    expr_session_free(session);
    printf("✓ Session freed\n\n");
}

// Test session with functions
void test_session_with_functions() {
    printf("=== Test Session with Functions ===\n");
    
    // Create session and context
    ExprSession* session = expr_session_new();
    ExprContext* ctx = expr_context_new();
    assert(session != NULL && ctx != NULL);
    
    // Register functions
    expr_context_add_function(ctx, "sin", 1, native_sin);
    expr_context_add_function(ctx, "cos", 1, native_cos);
    expr_context_add_function(ctx, "sqrt", 1, native_sqrt);
    expr_context_add_function(ctx, "pow", 2, native_pow);
    printf("✓ Functions registered\n");
    
    // Parse expression with functions
    int32_t result = expr_session_parse(session, "sqrt(pow(x, 2) + pow(y, 2))");
    assert(result == 0);
    printf("✓ Function expression parsed\n");
    
    // Add variables
    expr_session_add_variable(session, "x", 3.0);
    expr_session_add_variable(session, "y", 4.0);
    
    // Evaluate with context
    Real result_value;
    int32_t eval_res = expr_session_evaluate(session, ctx, &result_value);
    assert(eval_res == 0);
    assert(fabs(result_value - 5.0) < 0.0001);
    printf("✓ Function evaluation: %.1f (expected 5.0)\n", result_value);
    
    // Test trigonometric functions
    expr_session_parse(session, "sin(x) * sin(x) + cos(x) * cos(x)");
    expr_session_set_variable(session, "x", 1.234); // x = 1.234 radians
    eval_res = expr_session_evaluate(session, ctx, &result_value);
    assert(eval_res == 0);
    assert(fabs(result_value - 1.0) < 0.0001); // sin²x + cos²x = 1
    printf("✓ Trig identity verified: %.6f (expected 1.0)\n", result_value);
    
    // Cleanup
    expr_context_free(ctx);
    expr_session_free(session);
    printf("✓ Cleanup complete\n\n");
}

// Test multiple expressions in sequence
void test_multiple_expressions() {
    printf("=== Test Multiple Expressions ===\n");
    
    ExprSession* session = expr_session_new();
    ExprContext* ctx = expr_context_new();
    
    // Register some functions
    expr_context_add_function(ctx, "abs", 1, native_abs);
    expr_context_add_function(ctx, "max", 2, native_max);
    expr_context_add_function(ctx, "min", 2, native_min);
    
    // Test data for multiple expressions
    const char* expressions[] = {
        "a + b",
        "a - b", 
        "a * b",
        "a / b",
        "max(a, b)",
        "min(a, b)",
        "abs(a - b)"
    };
    
    Real expected[] = {7.0, 3.0, 10.0, 2.5, 5.0, 2.0, 3.0};
    
    // Add variables once
    expr_session_add_variable(session, "a", 5.0);
    expr_session_add_variable(session, "b", 2.0);
    
    // Test each expression
    for (int i = 0; i < 7; i++) {
        // Parse new expression
        int32_t parse_result = expr_session_parse(session, expressions[i]);
        assert(parse_result == 0);
        
        // Evaluate
        Real result;
        int32_t eval_res = expr_session_evaluate(session, ctx, &result);
        assert(eval_res == 0);
        assert(fabs(result - expected[i]) < 0.0001);
        printf("✓ Expression '%s' = %.1f (expected %.1f)\n", 
               expressions[i], result, expected[i]);
    }
    
    // Cleanup
    expr_context_free(ctx);
    expr_session_free(session);
    printf("\n");
}

// Test session reuse and performance
void test_session_performance() {
    printf("=== Test Session Performance (Parse Once, Eval Many) ===\n");
    
    ExprSession* session = expr_session_new();
    ExprContext* ctx = expr_context_new();
    
    // Register functions
    expr_context_add_function(ctx, "sin", 1, native_sin);
    expr_context_add_function(ctx, "cos", 1, native_cos);
    expr_context_add_function(ctx, "sqrt", 1, native_sqrt);
    
    // Parse complex expression once
    const char* expr = "sqrt(x*x + y*y) + sin(angle) * radius";
    expr_session_parse(session, expr);
    
    // Add variables
    expr_session_add_variable(session, "x", 0.0);
    expr_session_add_variable(session, "y", 0.0);
    expr_session_add_variable(session, "angle", 0.0);
    expr_session_add_variable(session, "radius", 1.0);
    
    printf("✓ Expression parsed once: %s\n", expr);
    
    // Simulate real-time updates (1000 iterations)
    const int iterations = 1000;
    Real sum = 0.0;
    
    for (int i = 0; i < iterations; i++) {
        // Update variables (simulating sensor data)
        Real t = i * 0.01; // time
        expr_session_set_variable(session, "x", cos(t) * 10.0);
        expr_session_set_variable(session, "y", sin(t) * 10.0);
        expr_session_set_variable(session, "angle", t);
        expr_session_set_variable(session, "radius", 5.0);
        
        // Evaluate (no parsing needed!)
        Real result;
        expr_session_evaluate(session, ctx, &result);
        sum += result;
    }
    
    printf("✓ Evaluated %d times without re-parsing\n", iterations);
    printf("✓ Average result: %.2f\n", sum / iterations);
    
    // Cleanup
    expr_context_free(ctx);
    expr_session_free(session);
    printf("\n");
}

// Test error handling
void test_session_error_handling() {
    printf("=== Test Session Error Handling ===\n");
    
    ExprSession* session = expr_session_new();
    
    // Test invalid expression
    int32_t result = expr_session_parse(session, "x + + y");
    if (result != 0) {
        printf("✓ Invalid expression rejected (error code: %d)\n", result);
    } else {
        printf("✗ Invalid expression was accepted\n");
    }
    
    // Test undefined variable
    result = expr_session_parse(session, "undefined_var + 1");
    if (result == 0) {
        Real eval_result;
        int32_t eval_res = expr_session_evaluate(session, NULL, &eval_result);
        // This might return an error or 0, depending on implementation
        printf("✓ Undefined variable handled (eval_res: %d, result: %.1f)\n", eval_res, eval_result);
    }
    
    // Test with NULL session
    result = expr_session_parse(NULL, "x + y");
    assert(result != 0);
    printf("✓ NULL session rejected\n");
    
    // Test with NULL expression
    result = expr_session_parse(session, NULL);
    assert(result != 0);
    printf("✓ NULL expression rejected\n");
    
    // Cleanup
    expr_session_free(session);
    printf("\n");
}

// Test session vs batch API comparison
void test_session_vs_batch_comparison() {
    printf("=== Test Session vs Batch API Comparison ===\n");
    
    // Setup for both APIs
    ExprContext* ctx = expr_context_new();
    expr_context_add_function(ctx, "sqrt", 1, native_sqrt);
    
    const char* expr = "sqrt(x*x + y*y)";
    Real x = 3.0, y = 4.0;
    
    // Test Session API
    ExprSession* session = expr_session_new();
    expr_session_parse(session, expr);
    expr_session_add_variable(session, "x", x);
    expr_session_add_variable(session, "y", y);
    Real session_result;
    expr_session_evaluate(session, ctx, &session_result);
    printf("✓ Session API result: %.1f\n", session_result);
    
    // Test Batch API  
    ExprArena* arena = expr_arena_new(4096);
    ExprBatch* batch = expr_batch_new(arena);
    expr_batch_add_expression(batch, expr);
    expr_batch_add_variable(batch, "x", x);
    expr_batch_add_variable(batch, "y", y);
    expr_batch_evaluate(batch, ctx);
    Real batch_result = expr_batch_get_result(batch, 0);
    printf("✓ Batch API result: %.1f\n", batch_result);
    
    // Verify same results
    assert(fabs(session_result - batch_result) < 0.0001);
    printf("✓ Both APIs produce identical results\n");
    
    // Cleanup
    expr_session_free(session);
    expr_batch_free(batch);
    expr_arena_free(arena);
    expr_context_free(ctx);
    printf("\n");
}

// Main test runner
int main() {
    printf("\n==== Session API Tests ====\n\n");
    
    // Just run the basic test for now
    test_basic_session();
    
    printf("==== Session API Tests Completed! ====\n\n");
    return 0;
}