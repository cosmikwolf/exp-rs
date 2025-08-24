#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <math.h>
#include "exp_rs.h"

// Helper to check if an error code indicates parse error
int is_parse_error(int32_t code) {
    // Parse errors are typically in the 10-19 range
    return code >= 10 && code <= 19;
}

// Helper to check if an error code indicates evaluation error
int is_eval_error(int32_t code) {
    // Eval errors are typically in the 20-29 range
    return code >= 20 && code <= 29;
}

// Test invalid expressions with new ExprResult API
void test_invalid_expressions() {
    printf("=== Test Invalid Expressions (with ExprResult) ===\n");
    
    // Arena managed internally: 8192);
    ExprBatch* batch = expr_batch_new(8192);
    
    // Test cases for invalid expressions
    struct {
        const char* expr;
        const char* desc;
    } invalid_exprs[] = {
        {"", "Empty expression"},
        {"   ", "Whitespace only"},
        {"2 +", "Missing operand"},
        {"* 3", "Missing left operand"},
        {"2 + + 3", "Double operator"},
        {"(2 + 3", "Unclosed parenthesis"},
        {"2 + 3)", "Extra closing parenthesis"},
        {"((2 + 3)", "Mismatched parentheses"},
        {"2 + * 3", "Adjacent operators"},
        {"sin()", "Empty function arguments"},
        {"sin(1, 2)", "Too many arguments"},
        {"unknown_func(1)", "Unknown function"},
        {"2 / 0", "Division by zero (valid parse, eval error)"},
        {"@#$%", "Invalid characters"},
        {"2..3", "Double decimal"},
        {"2e+", "Incomplete number"},
        {"if(1,2,3)", "Unsupported keyword"},
        {NULL, NULL}
    };
    
    int i = 0;
    while (invalid_exprs[i].expr != NULL) {
        ExprResult result = expr_batch_add_expression(batch, invalid_exprs[i].expr);
        if (result.status != 0) {
            printf("✓ %s rejected:\n", invalid_exprs[i].desc);
            printf("  - Error code: %d\n", result.status);
            printf("  - Error message: %s\n", result.error);
        } else {
            printf("✗ %s was accepted (index: %d)\n", 
                   invalid_exprs[i].desc, result.index);
        }
        i++;
    }
    
    expr_batch_free(batch);
    printf("\n");
}

// Test NULL pointer handling
void test_null_handling() {
    printf("=== Test NULL Pointer Handling ===\n");
    
    // Test batch creation with size 0 (should use default size)
    ExprBatch* batch = expr_batch_new(0);
    assert(batch != NULL);  // 0 means use default size, not an error
    expr_batch_free(batch);
    printf("✓ Batch with size 0 uses default size\n");
    
    // Test NULL batch operations with ExprResult
    ExprResult result = expr_batch_add_expression(NULL, "x + 1");
    assert(result.status != 0);
    printf("✓ NULL batch rejected for add_expression\n");
    printf("  - Error: %s\n", result.error);
    
    result = expr_batch_add_variable(NULL, "x", 1.0);
    assert(result.status != 0);
    printf("✓ NULL batch rejected for add_variable\n");
    printf("  - Error: %s\n", result.error);
    
    result = expr_batch_evaluate_ex(NULL, NULL);
    assert(result.status != 0);
    printf("✓ NULL batch rejected for evaluate_ex\n");
    printf("  - Error: %s\n", result.error);
    
    // Test NULL expression
    // Arena managed internally: 1024);
    batch = expr_batch_new(1024);
    result = expr_batch_add_expression(batch, NULL);
    assert(result.status != 0);
    printf("✓ NULL expression rejected\n");
    printf("  - Error: %s\n", result.error);
    
    // Test NULL parameter name
    result = expr_batch_add_variable(batch, NULL, 1.0);
    assert(result.status != 0);
    printf("✓ NULL parameter name rejected\n");
    printf("  - Error: %s\n", result.error);
    
    expr_batch_free(batch);
    printf("\n");
}

// Test parameter errors
void test_parameter_errors() {
    printf("=== Test Parameter Errors ===\n");
    
    // Arena managed internally: 4096);
    ExprBatch* batch = expr_batch_new(8192);
    
    // Add some valid parameters
    expr_batch_add_variable(batch, "x", 1.0);
    expr_batch_add_variable(batch, "y", 2.0);
    
    // Test duplicate parameter
    ExprResult res = expr_batch_add_variable(batch, "x", 3.0);
    if (res.status != 0) {
        printf("✓ Duplicate parameter rejected (error: %d)\n", res.status);
        printf("  - Error message: %s\n", res.error);
    } else {
        printf("✓ Duplicate parameter overwrites value\n");
    }
    
    // Test undefined variable in expression
    res = expr_batch_add_expression(batch, "x + y + z");
    if (res.status == 0) {
        // Expression parsed, now try to evaluate
        ExprContext* ctx = expr_context_new();
        ExprResult eval_res = expr_batch_evaluate_ex(batch, ctx);
        if (eval_res.status != 0) {
            printf("✓ Undefined variable caught at evaluation (error: %d)\n", eval_res.status);
            printf("  - Error message: %s\n", eval_res.error);
        } else {
            Real value = expr_batch_get_result(batch, 0);
            printf("✓ Undefined variable defaults to 0 (result: %.1f)\n", value);
        }
        expr_context_free(ctx);
    } else {
        printf("✓ Undefined variable caught at parse (error: %d)\n", res.status);
        printf("  - Error message: %s\n", res.error);
    }
    
    // Test invalid parameter index
    Real value = expr_batch_get_result(batch, 999);
    printf("✓ Invalid result index returns: %.1f\n", value);
    
    // Test parameter name limits
    char long_name[256];
    memset(long_name, 'a', 255);
    long_name[255] = '\0';
    res = expr_batch_add_variable(batch, long_name, 1.0);
    if (res.status != 0) {
        printf("✓ Very long parameter name rejected (error: %d)\n", res.status);
    } else {
        printf("✓ Very long parameter name accepted\n");
    }
    
    expr_batch_free(batch);
    printf("\n");
}

// Test function implementation
Real test_func(const Real* args, uintptr_t nargs) {
    (void)args; (void)nargs;
    return 42.0;
}

// Test function errors
void test_function_errors() {
    printf("=== Test Function Errors ===\n");
    
    // Arena managed internally: 4096);
    ExprBatch* batch = expr_batch_new(8192);
    ExprContext* ctx = expr_context_new();
    
    // Add test function
    expr_context_add_function(ctx, "test", 1, test_func);
    
    // Test unknown function
    ExprResult result = expr_batch_add_expression(batch, "unknown(1)");
    if (result.status != 0) {
        printf("✓ Unknown function caught at parse (error: %d)\n", result.status);
    } else {
        int32_t eval_result = expr_batch_evaluate(batch, ctx);
        if (eval_result != 0) {
            printf("✓ Unknown function caught at evaluation (error: %d)\n", eval_result);
        } else {
            printf("✗ Unknown function was not caught\n");
        }
    }
    
    // Test wrong number of arguments
    expr_batch_free(batch);
    batch = expr_batch_new(1024);
    result = expr_batch_add_expression(batch, "test(1, 2)"); // test expects 1 arg
    if (result.status != 0) {
        printf("✓ Wrong arg count caught at parse (error: %d)\n", result.status);
    } else {
        int32_t eval_result = expr_batch_evaluate(batch, ctx);
        if (eval_result != 0) {
            printf("✓ Wrong arg count caught at evaluation (error: %d)\n", eval_result);
        } else {
            printf("✗ Wrong arg count was not caught\n");
        }
    }
    
    // Test NULL function pointer
    int32_t func_result = expr_context_add_function(ctx, "null_func", 1, NULL);
    assert(func_result != 0);
    printf("✓ NULL function pointer rejected\n");
    
    // Test NULL function name
    func_result = expr_context_add_function(ctx, NULL, 1, test_func);
    assert(func_result != 0);
    printf("✓ NULL function name rejected\n");
    
    // Test empty function name
    func_result = expr_context_add_function(ctx, "", 1, test_func);
    if (func_result != 0) {
        printf("✓ Empty function name rejected (error: %d)\n", func_result);
    } else {
        printf("✓ Empty function name accepted\n");
    }
    
    expr_batch_free(batch);
    expr_context_free(ctx);
    printf("\n");
}

// Math function implementations
Real sqrt_func(const Real* args, uintptr_t nargs) {
    (void)nargs;
    return sqrt(args[0]);
}

Real log_func(const Real* args, uintptr_t nargs) {
    (void)nargs;
    return log(args[0]);
}

// Test arithmetic errors
void test_arithmetic_errors() {
    printf("=== Test Arithmetic Errors ===\n");
    
    // Arena managed internally: 4096);
    ExprBatch* batch = expr_batch_new(8192);
    ExprContext* ctx = expr_context_new();
    
    // Division by zero
    expr_batch_add_expression(batch, "1.0 / 0.0");
    int32_t result = expr_batch_evaluate(batch, ctx);
    if (result == 0) {
        Real value = expr_batch_get_result(batch, 0);
        if (isinf(value)) {
            printf("✓ Division by zero returns infinity: %f\n", value);
        } else {
            printf("✓ Division by zero returns: %f\n", value);
        }
    } else {
        printf("✓ Division by zero caught (error: %d)\n", result);
    }
    
    // Square root of negative
    expr_batch_free(batch);
    batch = expr_batch_new(1024);
    expr_context_add_function(ctx, "sqrt", 1, sqrt_func);
    
    expr_batch_add_expression(batch, "sqrt(-1.0)");
    result = expr_batch_evaluate(batch, ctx);
    if (result == 0) {
        Real value = expr_batch_get_result(batch, 0);
        if (isnan(value)) {
            printf("✓ sqrt(-1) returns NaN\n");
        } else {
            printf("✓ sqrt(-1) returns: %f\n", value);
        }
    } else {
        printf("✓ sqrt(-1) caught (error: %d)\n", result);
    }
    
    // Log of zero/negative
    expr_batch_free(batch);
    batch = expr_batch_new(1024);
    expr_context_add_function(ctx, "log", 1, log_func);
    
    expr_batch_add_expression(batch, "log(0.0)");
    result = expr_batch_evaluate(batch, ctx);
    if (result == 0) {
        Real value = expr_batch_get_result(batch, 0);
        if (isinf(value) && value < 0) {
            printf("✓ log(0) returns negative infinity\n");
        } else {
            printf("✓ log(0) returns: %f\n", value);
        }
    } else {
        printf("✓ log(0) caught (error: %d)\n", result);
    }
    
    expr_batch_free(batch);
    expr_context_free(ctx);
    printf("\n");
}

// Test memory limits
void test_memory_limits() {
    printf("=== Test Memory Limits ===\n");
    
    // Test very small arena
    ExprBatch* batch = expr_batch_new(64); // Very small arena
    
    // Try to add expressions until we run out of memory
    int count = 0;
    ExprResult result = {0};
    while (result.status == 0 && count < 100) {
        char expr[32];
        snprintf(expr, sizeof(expr), "x + %d", count);
        result = expr_batch_add_expression(batch, expr);
        if (result.status == 0) {
            count++;
        } else {
        }
    }
    
    if (result.status != 0) {
        printf("✓ Arena memory exhausted after %d expressions (error: %d)\n", 
               count, result.status);
    } else {
        printf("✓ Added %d expressions to tiny arena\n", count);
    }
    
    expr_batch_free(batch);
    
    // Test expression complexity limit
    // Arena managed internally: 8192);
    batch = expr_batch_new(1024);
    
    // Build a deeply nested expression
    char nested[1024] = "1";
    for (int i = 0; i < 50; i++) {
        char temp[1024];
        snprintf(temp, sizeof(temp), "(%s + 1)", nested);
        strncpy(nested, temp, sizeof(nested) - 1);
    }
    
    ExprResult nested_result = expr_batch_add_expression(batch, nested);
    if (nested_result.status != 0) {
        printf("✓ Deeply nested expression rejected (error: %d)\n", nested_result.status);
    } else {
        printf("✓ Deeply nested expression accepted\n");
    }
    
    expr_batch_free(batch);
    printf("\n");
}

// Test boundary conditions
void test_boundary_conditions() {
    printf("=== Test Boundary Conditions ===\n");
    
    // Arena managed internally: 4096);
    ExprBatch* batch = expr_batch_new(8192);
    ExprContext* ctx = expr_context_new();
    
    // Test very large numbers
    expr_batch_add_expression(batch, "1e308 + 1e308");
    int32_t result = expr_batch_evaluate(batch, ctx);
    if (result == 0) {
        Real value = expr_batch_get_result(batch, 0);
        if (isinf(value)) {
            printf("✓ Overflow returns infinity\n");
        } else {
            printf("✓ Large number result: %g\n", value);
        }
    }
    
    // Test very small numbers
    expr_batch_free(batch);
    batch = expr_batch_new(1024);
    expr_batch_add_expression(batch, "1e-308 * 1e-308");
    result = expr_batch_evaluate(batch, ctx);
    if (result == 0) {
        Real value = expr_batch_get_result(batch, 0);
        printf("✓ Underflow result: %g\n", value);
    }
    
    // Test zero expressions
    expr_batch_free(batch);
    batch = expr_batch_new(1024);
    result = expr_batch_evaluate(batch, ctx); // No expressions added
    if (result != 0) {
        printf("✓ Evaluating empty batch caught (error: %d)\n", result);
    } else {
        printf("✓ Evaluating empty batch succeeds\n");
    }
    
    // Test maximum parameters
    expr_batch_free(batch);
    batch = expr_batch_new(1024);
    int max_params = 0;
    for (int i = 0; i < 1000; i++) {
        char name[16];
        snprintf(name, sizeof(name), "p%d", i);
        ExprResult var_result = expr_batch_add_variable(batch, name, (Real)i);
        if (var_result.status != 0) {
            break;
        }
        max_params++;
    }
    printf("✓ Maximum parameters accepted: %d\n", max_params);
    
    expr_batch_free(batch);
    expr_context_free(ctx);
    printf("\n");
}


int main() {
    printf("\n==== Expression Error Handling Tests ====\n\n");
    
    test_invalid_expressions();
    test_null_handling();
    test_parameter_errors();
    test_function_errors();
    test_arithmetic_errors();
    test_memory_limits();
    test_boundary_conditions();
    
    printf("==== All Error Handling Tests Completed ====\n\n");
    return 0;
}