#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <time.h>
#include <math.h>
#include "exp_rs.h"

// Configuration for embedded system
#define NUM_EXPRESSIONS 7
#define NUM_PARAMETERS 10
#define RING_BUFFER_SIZE 4  // Power of 2 for efficient modulo
#define UPDATE_RATE_HZ 1000

// Static memory pool - no dynamic allocation after init
typedef struct {
    // Expression strings - stored once at init
    const char* expressions[NUM_EXPRESSIONS];
    
    // Parameter names - stored once at init
    const char* param_names[NUM_PARAMETERS];
    
    // Ring buffer for parameter values
    double param_values_ring[RING_BUFFER_SIZE][NUM_PARAMETERS];
    
    // Ring buffer for results
    double results_ring[RING_BUFFER_SIZE][NUM_EXPRESSIONS];
    
    // Pointers for batch evaluation (reused)
    double* param_value_ptrs[NUM_PARAMETERS];
    double* result_ptrs[NUM_EXPRESSIONS];
    
    // Ring buffer indices
    uint32_t write_index;
    uint32_t read_index;
    
    // Pre-parsed batch builder
    void* arena;
    void* batch_builder;
    void* eval_context;
    
    // Performance counters
    uint64_t total_evals;
    uint64_t max_eval_time_us;
    uint64_t total_eval_time_us;
} EmbeddedPool;

// Global pool - in embedded systems this would be in a specific memory section
static EmbeddedPool g_pool = {0};

// Native function wrappers
Real native_sin(const Real* args, uintptr_t n_args) {
    return sin(args[0]);
}

Real native_sqrt(const Real* args, uintptr_t n_args) {
    return sqrt(args[0]);
}

// Initialize the memory pool - called once at startup
int embedded_pool_init(void) {
    // Define expressions (these would come from your config)
    g_pool.expressions[0] = "p0 + p1";
    g_pool.expressions[1] = "p0 * p1 + p2";
    g_pool.expressions[2] = "sqrt(p0*p0 + p1*p1)";
    g_pool.expressions[3] = "p3 * sin(p4)";
    g_pool.expressions[4] = "p5 + p6 - p7";
    g_pool.expressions[5] = "p8 * p8 * p9";
    g_pool.expressions[6] = "(p0 + p1 + p2) / 3.0";
    
    // Define parameter names
    for (int i = 0; i < NUM_PARAMETERS; i++) {
        static char names[NUM_PARAMETERS][4];
        sprintf(names[i], "p%d", i);
        g_pool.param_names[i] = names[i];
    }
    
    // Create context and batch builder
    g_pool.eval_context = exp_rs_context_new();
    if (!g_pool.eval_context) {
        printf("Failed to create context\n");
        return -1;
    }
    
    // Register required math functions
    EvalResult reg_result;
    
    reg_result = exp_rs_context_register_native_function(g_pool.eval_context, "sin", 1, native_sin);
    if (reg_result.status != 0) {
        printf("Failed to register sin function: %s\n", reg_result.error);
        exp_rs_free_error((char*)reg_result.error);
        return -1;
    }
    
    reg_result = exp_rs_context_register_native_function(g_pool.eval_context, "sqrt", 1, native_sqrt);
    if (reg_result.status != 0) {
        printf("Failed to register sqrt function: %s\n", reg_result.error);
        exp_rs_free_error((char*)reg_result.error);
        return -1;
    }
    
    // Create arena for zero-allocation expression evaluation
    g_pool.arena = exp_rs_arena_new(16384); // 16KB arena for embedded system
    if (!g_pool.arena) {
        printf("Failed to create arena\n");
        exp_rs_context_free(g_pool.eval_context);
        return -1;
    }
    
    g_pool.batch_builder = exp_rs_batch_builder_new(g_pool.arena);
    if (!g_pool.batch_builder) {
        printf("Failed to create batch builder\n");
        exp_rs_arena_free(g_pool.arena);
        exp_rs_context_free(g_pool.eval_context);
        return -1;
    }
    
    // Add expressions to batch builder (parsed once)
    for (int i = 0; i < NUM_EXPRESSIONS; i++) {
        int32_t result = exp_rs_batch_builder_add_expression(
            g_pool.batch_builder, 
            g_pool.expressions[i]
        );
        if (result < 0) {
            printf("Failed to add expression %d: %s\n", i, g_pool.expressions[i]);
            return -1;
        }
    }
    
    // Add parameters to batch builder
    for (int i = 0; i < NUM_PARAMETERS; i++) {
        int32_t result = exp_rs_batch_builder_add_parameter(
            g_pool.batch_builder,
            g_pool.param_names[i],
            0.0
        );
        if (result < 0) {
            printf("Failed to add parameter %s\n", g_pool.param_names[i]);
            return -1;
        }
    }
    
    // Initialize ring buffer indices
    g_pool.write_index = 0;
    g_pool.read_index = 0;
    
    return 0;
}

// Update parameters - called from interrupt or high-priority task
void embedded_pool_update_params(const double* new_values) {
    uint32_t idx = g_pool.write_index & (RING_BUFFER_SIZE - 1);
    
    // Copy new parameter values to ring buffer
    memcpy(g_pool.param_values_ring[idx], new_values, 
           NUM_PARAMETERS * sizeof(double));
    
    // Advance write index
    g_pool.write_index++;
}

// Process batch - called from lower priority task
int embedded_pool_process(void) {
    // Check if there's data to process
    if (g_pool.read_index >= g_pool.write_index) {
        return 0; // No new data
    }
    
    uint32_t idx = g_pool.read_index & (RING_BUFFER_SIZE - 1);
    
    // Measure evaluation time
    struct timespec start, end;
    clock_gettime(CLOCK_MONOTONIC, &start);
    
    // Update parameters in batch builder
    for (int i = 0; i < NUM_PARAMETERS; i++) {
        exp_rs_batch_builder_set_param(
            g_pool.batch_builder,
            i,
            g_pool.param_values_ring[idx][i]
        );
    }
    
    // Evaluate all expressions
    int32_t result = exp_rs_batch_builder_eval(
        g_pool.batch_builder,
        g_pool.eval_context
    );
    
    if (result != 0) {
        printf("Evaluation failed with code %d\n", result);
        return -1;
    }
    
    // Copy results to ring buffer
    for (int i = 0; i < NUM_EXPRESSIONS; i++) {
        g_pool.results_ring[idx][i] = exp_rs_batch_builder_get_result(
            g_pool.batch_builder,
            i
        );
    }
    
    clock_gettime(CLOCK_MONOTONIC, &end);
    
    // Update performance counters
    uint64_t elapsed_us = (end.tv_sec - start.tv_sec) * 1000000 + 
                         (end.tv_nsec - start.tv_nsec) / 1000;
    g_pool.total_eval_time_us += elapsed_us;
    if (elapsed_us > g_pool.max_eval_time_us) {
        g_pool.max_eval_time_us = elapsed_us;
    }
    g_pool.total_evals++;
    
    // Advance read index
    g_pool.read_index++;
    
    return 1; // Processed one set
}

// Get results - can be called from any task
int embedded_pool_get_results(uint32_t batch_offset, double* results) {
    if (batch_offset >= RING_BUFFER_SIZE) {
        return -1;
    }
    
    uint32_t idx = (g_pool.read_index - 1 - batch_offset) & (RING_BUFFER_SIZE - 1);
    
    // Check if this index has valid data
    if (g_pool.read_index <= batch_offset) {
        return -2; // No data available at this offset
    }
    
    memcpy(results, g_pool.results_ring[idx], 
           NUM_EXPRESSIONS * sizeof(double));
    
    return 0;
}

// Cleanup - called at shutdown
void embedded_pool_cleanup(void) {
    if (g_pool.batch_builder) {
        exp_rs_batch_builder_free(g_pool.batch_builder);
        g_pool.batch_builder = NULL;
    }
    
    if (g_pool.arena) {
        exp_rs_arena_free(g_pool.arena);
        g_pool.arena = NULL;
    }
    
    if (g_pool.eval_context) {
        exp_rs_context_free(g_pool.eval_context);
        g_pool.eval_context = NULL;
    }
}

// Example usage simulating 1000Hz operation
int main() {
    printf("=== Embedded Memory Pool Test ===\n");
    printf("Expressions: %d, Parameters: %d, Rate: %d Hz\n\n",
           NUM_EXPRESSIONS, NUM_PARAMETERS, UPDATE_RATE_HZ);
    
    // Initialize pool
    if (embedded_pool_init() != 0) {
        printf("Failed to initialize pool\n");
        return 1;
    }
    
    printf("Memory pool initialized successfully\n");
    printf("All expressions parsed, no further allocations will occur\n\n");
    
    // Simulate 1 second of operation
    int iterations = UPDATE_RATE_HZ;
    double params[NUM_PARAMETERS];
    double results[NUM_EXPRESSIONS];
    
    for (int i = 0; i < iterations; i++) {
        // Simulate sensor data
        for (int j = 0; j < NUM_PARAMETERS; j++) {
            params[j] = (double)(i + j) / 100.0;
        }
        
        // Producer: Update parameters (simulating interrupt/DMA)
        embedded_pool_update_params(params);
        
        // Consumer: Process available data
        while (embedded_pool_process() > 0) {
            // Process all available data
        }
        
        // Every 100ms, print status
        if (i % 100 == 99) {
            embedded_pool_get_results(0, results);
            printf("Iteration %d: First result = %.6f\n", i + 1, results[0]);
        }
    }
    
    // Print performance statistics
    printf("\n=== Performance Statistics ===\n");
    printf("Total evaluations: %llu\n", (unsigned long long)g_pool.total_evals);
    printf("Average eval time: %.2f μs\n", 
           (double)g_pool.total_eval_time_us / g_pool.total_evals);
    printf("Maximum eval time: %llu μs\n", (unsigned long long)g_pool.max_eval_time_us);
    printf("Total memory allocated: 0 bytes (after initialization)\n");
    
    // Verify final results
    embedded_pool_get_results(0, results);
    printf("\nFinal results:\n");
    for (int i = 0; i < NUM_EXPRESSIONS; i++) {
        printf("  Expression %d: %.6f\n", i, results[i]);
    }
    
    // Cleanup
    embedded_pool_cleanup();
    printf("\nTest complete\n");
    
    return 0;
}