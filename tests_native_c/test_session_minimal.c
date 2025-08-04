#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
#include "exp_rs.h"

int main() {
    printf("Testing minimal session API...\n");
    
    // Initialize arena pool
    printf("1. Initializing arena pool...\n");
    if (!expr_pool_init(4)) {
        printf("ERROR: Failed to initialize arena pool\n");
        return 1;
    }
    printf("   OK: Arena pool initialized\n");
    
    // Create session
    printf("2. Creating session...\n");
    ExprSession* session = expr_session_new();
    if (!session) {
        printf("ERROR: Failed to create session\n");
        return 1;
    }
    printf("   OK: Session created\n");
    
    // Parse expression
    printf("3. Parsing expression...\n");
    int32_t result = expr_session_parse(session, "2 + 3");
    if (result != 0) {
        printf("ERROR: Failed to parse expression (code: %d)\n", result);
        expr_session_free(session);
        return 1;
    }
    printf("   OK: Expression parsed\n");
    
    // Evaluate
    printf("4. Evaluating expression...\n");
    Real value;
    result = expr_session_evaluate(session, NULL, &value);
    if (result != 0) {
        printf("ERROR: Failed to evaluate expression (code: %d)\n", result);
        expr_session_free(session);
        return 1;
    }
    printf("   OK: Result = %f (expected 5.0)\n", value);
    
    // Verify result
    if (value != 5.0) {
        printf("ERROR: Expected 5.0 but got %f\n", value);
        expr_session_free(session);
        return 1;
    }
    
    // Free session
    printf("5. Freeing session...\n");
    expr_session_free(session);
    printf("   OK: Session freed\n");
    
    printf("\nAll tests passed!\n");
    return 0;
}