#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/wait.h>
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

// Test function that should trigger a panic
void test_panic_trigger() {
    ExprContext* ctx = expr_context_new();
    ExprArena* arena = expr_arena_new(8192);
    ExprBatch* batch = expr_batch_new(arena);
    
    // Create an expression that should cause issues
    // For now, let's try something that might overflow or cause internal errors
    
    // Try 1: Create a recursive expression function that should hit recursion limits
    expr_context_add_expression_function(ctx, "recurse", "x", "recurse(x+1)");
    expr_batch_add_expression(batch, "recurse(1)");
    
    int status = expr_batch_evaluate(batch, ctx);
    
    // If we get here, no panic occurred
    printf("   - Expression evaluation returned: %d\n", status);
    
    expr_batch_free(batch);
    expr_arena_free(arena);
    expr_context_free(ctx);
}

int main() {
    printf("=== Panic Handler Test ===\n\n");
    
    // Test 1: Register panic handler
    printf("1. Registering panic handler:\n");
    exp_rs_register_panic_handler(&panic_flag, (void*)panic_logger);
    printf("   - Panic handler registered\n");
    printf("   - Initial panic flag: %d\n", panic_flag);
    
    // Test 2: Normal operation (no panic)
    printf("\n2. Testing normal operation:\n");
    panic_flag = 0;
    
    ExprContext* ctx = expr_context_new();
    ExprArena* arena = expr_arena_new(8192);
    ExprBatch* batch = expr_batch_new(arena);
    expr_batch_add_expression(batch, "2 + 3");
    int status = expr_batch_evaluate(batch, ctx);
    Real result = expr_batch_get_result(batch, 0);
    
    printf("   - Expression evaluated: %s\n", status == 0 ? "success" : "failed");
    printf("   - Result: %.1f\n", result);
    printf("   - Panic flag: %d (expected 0)\n", panic_flag);
    
    expr_batch_free(batch);
    expr_arena_free(arena);
    expr_context_free(ctx);
    
    // Test 3: Try to trigger panic in a subprocess
    printf("\n3. Testing panic trigger in subprocess:\n");
    
    pid_t pid = fork();
    if (pid == 0) {
        // Child process - try to trigger panic
        printf("   - Child process attempting to trigger panic...\n");
        
        // Re-register panic handler in child
        exp_rs_register_panic_handler(&panic_flag, (void*)panic_logger);
        
        // Try various things that might panic
        test_panic_trigger();
        
        // If we're still here, try the debug-only panic trigger
        #ifdef DEBUG
        printf("   - Child: Calling exp_rs_test_trigger_panic()...\n");
        exp_rs_test_trigger_panic();
        #else
        printf("   - Child: exp_rs_test_trigger_panic() not available (release build)\n");
        #endif
        
        printf("   - Child process: No panic triggered\n");
        exit(0);
    } else if (pid > 0) {
        // Parent process - wait for child
        int child_status;
        waitpid(pid, &child_status, 0);
        
        if (WIFEXITED(child_status)) {
            printf("   - Child exited normally with status: %d\n", WEXITSTATUS(child_status));
        } else if (WIFSIGNALED(child_status)) {
            printf("   - Child terminated by signal: %d\n", WTERMSIG(child_status));
            if (WTERMSIG(child_status) == SIGABRT) {
                printf("   - SIGABRT indicates a panic may have occurred\n");
            }
        }
    } else {
        perror("fork");
        return 1;
    }
    
    // Test 4: Check panic handler state after subprocess
    printf("\n4. Final state:\n");
    printf("   - Parent panic flag: %d (should be 0 - not affected by child)\n", panic_flag);
    
    // Test 5: Unregister panic handler
    printf("\n5. Unregistering panic handler:\n");
    exp_rs_register_panic_handler(NULL, NULL);
    printf("   - Panic handler unregistered\n");
    
    printf("\n=== Panic Handler Test Completed ===\n");
    printf("Note: The panic handler is designed for no_std ARM targets.\n");
    printf("In this development environment (with std), normal Rust panic handling applies.\n");
    printf("The exp_rs_register_panic_handler() function can be called but won't intercept panics.\n");
    printf("On ARM targets without std, the handler would set the flag and call the logger.\n");
    
    return 0;
}