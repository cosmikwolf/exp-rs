#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
#include <unistd.h>
#include <sys/time.h>
#include <assert.h>
#include <stdint.h>
#include "exp_rs.h"

// Memory tracking structures
typedef struct allocation_info {
    void* ptr;
    size_t size;
    struct allocation_info* next;
    const char* type;  // For categorizing allocations
} allocation_info_t;

typedef struct {
    allocation_info_t* allocations;
    size_t total_allocated;
    size_t total_freed;
    size_t peak_usage;
    size_t current_usage;
    size_t allocation_count;
    size_t free_count;
    int tracking_enabled;
} memory_tracker_t;

// Global memory tracker
static memory_tracker_t g_tracker = {0};

// Original malloc/free functions
static void* (*original_malloc)(size_t) = NULL;
static void (*original_free)(void*) = NULL;

// Custom malloc wrapper
void* malloc(size_t size) {
    if (!original_malloc) {
        original_malloc = dlsym(RTLD_NEXT, "malloc");
    }
    
    void* ptr = original_malloc(size);
    
    if (g_tracker.tracking_enabled && ptr) {
        // Create allocation record
        allocation_info_t* info = original_malloc(sizeof(allocation_info_t));
        if (info) {
            info->ptr = ptr;
            info->size = size;
            info->next = g_tracker.allocations;
            info->type = "malloc";
            g_tracker.allocations = info;
            
            g_tracker.total_allocated += size;
            g_tracker.current_usage += size;
            g_tracker.allocation_count++;
            
            if (g_tracker.current_usage > g_tracker.peak_usage) {
                g_tracker.peak_usage = g_tracker.current_usage;
            }
        }
    }
    
    return ptr;
}

// Custom free wrapper
void free(void* ptr) {
    if (!original_free) {
        original_free = dlsym(RTLD_NEXT, "free");
    }
    
    if (g_tracker.tracking_enabled && ptr) {
        // Find and remove allocation record
        allocation_info_t** current = &g_tracker.allocations;
        while (*current) {
            if ((*current)->ptr == ptr) {
                allocation_info_t* to_remove = *current;
                g_tracker.total_freed += to_remove->size;
                g_tracker.current_usage -= to_remove->size;
                g_tracker.free_count++;
                
                *current = to_remove->next;
                original_free(to_remove);
                break;
            }
            current = &(*current)->next;
        }
    }
    
    original_free(ptr);
}

// Memory tracking utilities
void start_memory_tracking() {
    g_tracker.tracking_enabled = 1;
    g_tracker.allocations = NULL;
    g_tracker.total_allocated = 0;
    g_tracker.total_freed = 0;
    g_tracker.peak_usage = 0;
    g_tracker.current_usage = 0;
    g_tracker.allocation_count = 0;
    g_tracker.free_count = 0;
}

void stop_memory_tracking() {
    g_tracker.tracking_enabled = 0;
}

void reset_memory_tracking() {
    stop_memory_tracking();
    
    // Clean up any remaining allocation records
    allocation_info_t* current = g_tracker.allocations;
    while (current) {
        allocation_info_t* next = current->next;
        original_free(current);
        current = next;
    }
    
    memset(&g_tracker, 0, sizeof(g_tracker));
}

void print_memory_report(const char* test_name) {
    printf("\n=== Memory Report for %s ===\n", test_name);
    printf("Total allocated: %zu bytes (%zu allocations)\n", 
           g_tracker.total_allocated, g_tracker.allocation_count);
    printf("Total freed: %zu bytes (%zu frees)\n", 
           g_tracker.total_freed, g_tracker.free_count);
    printf("Peak usage: %zu bytes\n", g_tracker.peak_usage);
    printf("Current usage: %zu bytes\n", g_tracker.current_usage);
    
    size_t leaked = g_tracker.total_allocated - g_tracker.total_freed;
    if (leaked > 0) {
        printf("⚠️  POTENTIAL MEMORY LEAK: %zu bytes\n", leaked);
        
        // Print details of unfreed allocations
        printf("\nUnfreed allocations:\n");
        allocation_info_t* current = g_tracker.allocations;
        int leak_count = 0;
        while (current && leak_count < 10) {  // Limit output
            printf("  - %p: %zu bytes (%s)\n", current->ptr, current->size, current->type);
            current = current->next;
            leak_count++;
        }
        if (current) {
            printf("  ... and more\n");
        }
    } else if (leaked == 0) {
        printf("✅ No memory leaks detected\n");
    } else {
        printf("⚠️  More memory freed than allocated (%zd bytes) - possible double free\n", -leaked);
    }
    printf("=====================================\n");
}

// Test utilities
double get_time() {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return tv.tv_sec + tv.tv_usec / 1000000.0;
}

// Test 1: Basic BatchBuilder lifecycle
void test_basic_lifecycle() {
    printf("\n--- Test 1: Basic BatchBuilder Lifecycle ---\n");
    
    start_memory_tracking();
    
    // Create context
    struct EvalContextOpaque* ctx = exp_rs_context_new();
    assert(ctx != NULL);
    
    // Create batch builder
    struct BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
    assert(builder != NULL);
    
    // Add simple expressions
    int expr1 = exp_rs_batch_builder_add_expression(builder, "2 + 3");
    int expr2 = exp_rs_batch_builder_add_expression(builder, "x * 2");
    assert(expr1 >= 0 && expr2 >= 0);
    
    // Add parameters
    int param1 = exp_rs_batch_builder_add_parameter(builder, "x", 5.0);
    assert(param1 >= 0);
    
    // Evaluate
    int result = exp_rs_batch_builder_eval(builder, ctx);
    assert(result == 0);
    
    // Check results
    double result1 = exp_rs_batch_builder_get_result(builder, expr1);
    double result2 = exp_rs_batch_builder_get_result(builder, expr2);
    printf("Results: expr1=%.1f, expr2=%.1f\n", result1, result2);
    
    // Clean up
    exp_rs_batch_builder_free(builder);
    exp_rs_context_free(ctx);
    
    stop_memory_tracking();
    print_memory_report("Basic Lifecycle");
}

// Test 2: Multiple create/destroy cycles
void test_repeated_cycles() {
    printf("\n--- Test 2: Repeated Create/Destroy Cycles ---\n");
    
    const int NUM_CYCLES = 100;
    
    start_memory_tracking();
    
    struct EvalContextOpaque* ctx = exp_rs_context_new();
    
    for (int i = 0; i < NUM_CYCLES; i++) {
        struct BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        // Add expressions
        exp_rs_batch_builder_add_expression(builder, "a + b");
        exp_rs_batch_builder_add_expression(builder, "a * b + c");
        exp_rs_batch_builder_add_expression(builder, "sin(a) + cos(b)");
        
        // Add parameters
        exp_rs_batch_builder_add_parameter(builder, "a", (double)i);
        exp_rs_batch_builder_add_parameter(builder, "b", (double)(i + 1));
        exp_rs_batch_builder_add_parameter(builder, "c", (double)(i + 2));
        
        // Evaluate
        exp_rs_batch_builder_eval(builder, ctx);
        
        // Clean up
        exp_rs_batch_builder_free(builder);
        
        if ((i + 1) % 20 == 0) {
            printf("Completed %d cycles, current usage: %zu bytes\n", 
                   i + 1, g_tracker.current_usage);
        }
    }
    
    exp_rs_context_free(ctx);
    
    stop_memory_tracking();
    print_memory_report("Repeated Cycles");
}

// Test 3: Complex expressions with nested functions
void test_complex_expressions() {
    printf("\n--- Test 3: Complex Expressions ---\n");
    
    start_memory_tracking();
    
    struct EvalContextOpaque* ctx = exp_rs_context_new();
    struct BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
    
    // Add complex expressions
    exp_rs_batch_builder_add_expression(builder, "sqrt(x^2 + y^2)");
    exp_rs_batch_builder_add_expression(builder, "sin(pi * x) + cos(pi * y)");
    exp_rs_batch_builder_add_expression(builder, "max(min(x, y), abs(x - y))");
    exp_rs_batch_builder_add_expression(builder, "x > 0 ? log(x) : -log(-x)");
    exp_rs_batch_builder_add_expression(builder, "((x + y) * (x - y)) / (x^2 + y^2 + 1)");
    
    // Add parameters
    exp_rs_batch_builder_add_parameter(builder, "x", 3.14);
    exp_rs_batch_builder_add_parameter(builder, "y", 2.71);
    
    // Evaluate multiple times
    for (int i = 0; i < 10; i++) {
        exp_rs_batch_builder_set_param_by_name(builder, "x", 3.14 + i * 0.1);
        exp_rs_batch_builder_set_param_by_name(builder, "y", 2.71 + i * 0.1);
        
        int result = exp_rs_batch_builder_eval(builder, ctx);
        assert(result == 0);
    }
    
    printf("Complex expressions evaluated successfully\n");
    
    exp_rs_batch_builder_free(builder);
    exp_rs_context_free(ctx);
    
    stop_memory_tracking();
    print_memory_report("Complex Expressions");
}

// Test 4: Error conditions and cleanup
void test_error_conditions() {
    printf("\n--- Test 4: Error Conditions ---\n");
    
    start_memory_tracking();
    
    struct EvalContextOpaque* ctx = exp_rs_context_new();
    struct BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
    
    // Add expressions that will cause parse errors
    int result1 = exp_rs_batch_builder_add_expression(builder, "invalid syntax +++");
    int result2 = exp_rs_batch_builder_add_expression(builder, "unclosed_function(");
    int result3 = exp_rs_batch_builder_add_expression(builder, "unknown_function(x)");
    
    printf("Parse error results: %d, %d, %d\n", result1, result2, result3);
    
    // Add valid expression
    int valid_expr = exp_rs_batch_builder_add_expression(builder, "x + 1");
    assert(valid_expr >= 0);
    
    exp_rs_batch_builder_add_parameter(builder, "x", 5.0);
    
    // Try to evaluate (should handle errors gracefully)
    int eval_result = exp_rs_batch_builder_eval(builder, ctx);
    printf("Evaluation result with errors: %d\n", eval_result);
    
    exp_rs_batch_builder_free(builder);
    exp_rs_context_free(ctx);
    
    stop_memory_tracking();
    print_memory_report("Error Conditions");
}

// Test 5: Large batch operations
void test_large_batch() {
    printf("\n--- Test 5: Large Batch Operations ---\n");
    
    const int NUM_EXPRESSIONS = 50;
    const int NUM_PARAMETERS = 20;
    
    start_memory_tracking();
    
    struct EvalContextOpaque* ctx = exp_rs_context_new();
    struct BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
    
    // Add parameters
    for (int i = 0; i < NUM_PARAMETERS; i++) {
        char param_name[32];
        snprintf(param_name, sizeof(param_name), "p%d", i);
        exp_rs_batch_builder_add_parameter(builder, param_name, (double)i);
    }
    
    // Add expressions using the parameters
    for (int i = 0; i < NUM_EXPRESSIONS; i++) {
        char expr[256];
        snprintf(expr, sizeof(expr), "p%d + p%d * p%d", 
                 i % NUM_PARAMETERS, 
                 (i + 1) % NUM_PARAMETERS, 
                 (i + 2) % NUM_PARAMETERS);
        exp_rs_batch_builder_add_expression(builder, expr);
    }
    
    printf("Added %d expressions and %d parameters\n", NUM_EXPRESSIONS, NUM_PARAMETERS);
    
    // Evaluate
    double start_time = get_time();
    int result = exp_rs_batch_builder_eval(builder, ctx);
    double end_time = get_time();
    
    printf("Large batch evaluation result: %d\n", result);
    if (result != 0) {
        printf("⚠️  Large batch evaluation failed, but continuing to test memory cleanup\n");
    }
    printf("Large batch evaluation took %.3f ms\n", (end_time - start_time) * 1000);
    
    exp_rs_batch_builder_free(builder);
    exp_rs_context_free(ctx);
    
    stop_memory_tracking();
    print_memory_report("Large Batch");
}

int main() {
    printf("BatchBuilder Memory Leak Detection Test\n");
    printf("=======================================\n");
    
    // Initialize original function pointers
    original_malloc = dlsym(RTLD_NEXT, "malloc");
    original_free = dlsym(RTLD_NEXT, "free");
    
    if (!original_malloc || !original_free) {
        printf("Failed to get original malloc/free functions\n");
        return 1;
    }
    
    // Run all tests
    test_basic_lifecycle();
    reset_memory_tracking();
    
    test_repeated_cycles();
    reset_memory_tracking();
    
    test_complex_expressions();
    reset_memory_tracking();
    
    test_error_conditions();
    reset_memory_tracking();
    
    test_large_batch();
    reset_memory_tracking();
    
    printf("\n🎉 All memory leak tests completed!\n");
    printf("Check the reports above for any memory leaks.\n");
    
    return 0;
}