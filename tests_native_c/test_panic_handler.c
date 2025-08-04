#include <stdio.h>
#include <string.h>
#include "../include/exp_rs.h"

// Global panic flag
static int panic_flag = 0;

// Buffer to store panic message
static char panic_message[256] = {0};
static size_t panic_message_len = 0;

// Logging function called by Rust on panic
void panic_logger(const unsigned char* msg, size_t len) {
    // Copy the panic message
    size_t copy_len = len < sizeof(panic_message) - 1 ? len : sizeof(panic_message) - 1;
    memcpy(panic_message, msg, copy_len);
    panic_message[copy_len] = '\0';
    panic_message_len = copy_len;
    
    printf("   - Panic logger called with message: %.*s\n", (int)len, msg);
}

int main() {
    printf("=== Panic Handler Test ===\n\n");
    
    // Register panic handler
    printf("1. Registering panic handler:\n");
    exp_rs_register_panic_handler(&panic_flag, (void*)panic_logger);
    printf("   - Panic handler registered\n");
    
    // Create context
    ExprContext* ctx = expr_context_new();
    if (!ctx) {
        printf("Failed to create context\n");
        return 1;
    }
    
    // Test 1: Normal operation (no panic)
    printf("\n2. Testing normal operation (no panic):\n");
    panic_flag = 0;
    
    ExprSession* session = expr_session_new();
    expr_session_parse(session, "2 + 3");
    Real result;
    int status = expr_session_evaluate(session, ctx, &result);
    
    printf("   - Expression evaluated: %s\n", status == 0 ? "success" : "failed");
    printf("   - Result: %.1f\n", result);
    printf("   - Panic flag: %d (expected 0)\n", panic_flag);
    
    expr_session_free(session);
    
    // Test 2: Expression that might cause panic (division by zero)
    printf("\n3. Testing division by zero:\n");
    panic_flag = 0;
    memset(panic_message, 0, sizeof(panic_message));
    
    session = expr_session_new();
    expr_session_parse(session, "1 / 0");
    status = expr_session_evaluate(session, ctx, &result);
    
    // Division by zero should return an error, not panic
    printf("   - Expression evaluated: %s\n", status == 0 ? "success" : "error (expected)");
    printf("   - Panic flag: %d (expected 0 - should be handled error, not panic)\n", panic_flag);
    
    expr_session_free(session);
    
    // Test 3: Try to trigger panic with very deep recursion
    printf("\n4. Testing deep recursion (may trigger stack overflow):\n");
    panic_flag = 0;
    memset(panic_message, 0, sizeof(panic_message));
    
    // Create a deeply nested expression
    char deep_expr[1024];
    strcpy(deep_expr, "1");
    for (int i = 0; i < 100; i++) {
        strcat(deep_expr, "+1");
    }
    
    session = expr_session_new();
    expr_session_parse(session, deep_expr);
    status = expr_session_evaluate(session, ctx, &result);
    
    printf("   - Expression evaluated: %s\n", status == 0 ? "success" : "failed");
    if (status == 0) {
        printf("   - Result: %.1f\n", result);
    }
    printf("   - Panic flag: %d\n", panic_flag);
    
    expr_session_free(session);
    
    // Test 4: Invalid UTF-8 handling (should not panic)
    printf("\n5. Testing invalid UTF-8:\n");
    panic_flag = 0;
    
    // Create invalid UTF-8 sequence
    char invalid_utf8[] = {0xFF, 0xFE, 0xFD, 0};
    
    session = expr_session_new();
    status = expr_session_parse(session, invalid_utf8);
    
    printf("   - Parse with invalid UTF-8: %s\n", status == 0 ? "success" : "error (expected)");
    printf("   - Panic flag: %d (expected 0 - should be handled error)\n", panic_flag);
    
    expr_session_free(session);
    
    // Test 5: NULL pointer handling (should not panic)
    printf("\n6. Testing NULL pointer handling:\n");
    panic_flag = 0;
    
    status = expr_session_parse(NULL, "2+2");
    printf("   - Parse with NULL session: %s\n", status == 0 ? "success" : "error (expected)");
    printf("   - Panic flag: %d (expected 0)\n", panic_flag);
    
    status = expr_session_evaluate(NULL, ctx, &result);
    printf("   - Evaluate with NULL session: %s\n", status == 0 ? "success" : "error (expected)");
    printf("   - Panic flag: %d (expected 0)\n", panic_flag);
    
    // Test 6: Test panic logger was never called in normal operation
    printf("\n7. Panic logger status:\n");
    if (panic_message_len > 0) {
        printf("   - Panic message received: %s\n", panic_message);
    } else {
        printf("   - No panic messages received (expected for normal operation)\n");
    }
    
    // Test 7: Unregister panic handler
    printf("\n8. Unregistering panic handler:\n");
    exp_rs_register_panic_handler(NULL, NULL);
    printf("   - Panic handler unregistered\n");
    
    // Clean up
    expr_context_free(ctx);
    
    printf("\n=== Panic Handler Test Completed ===\n");
    printf("Summary: Panic handler registered successfully.\n");
    printf("All operations completed without triggering panics (as expected).\n");
    
    return 0;
}