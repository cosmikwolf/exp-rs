#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <time.h>
#include <dlfcn.h>
#include <math.h>
#include "exp_rs.h"

// Memory tracking globals
static size_t g_total_allocated = 0;
static size_t g_current_allocated = 0;
static size_t g_peak_allocated = 0;
static size_t g_allocation_count = 0;
static size_t g_free_count = 0;
static int g_tracking_enabled = 0;

// Original malloc/free function pointers
static void* (*real_malloc)(size_t) = NULL;
static void (*real_free)(void*) = NULL;
static void* (*real_calloc)(size_t, size_t) = NULL;
static void* (*real_realloc)(void*, size_t) = NULL;

// Allocation header to track size
typedef struct {
    size_t size;
    size_t magic;  // To verify we're freeing our own allocations
} alloc_header_t;

#define MAGIC_VALUE 0xDEADBEEF

// Initialize function pointers
static void init_memory_hooks() {
    static int initialized = 0;
    if (!initialized) {
        real_malloc = dlsym(RTLD_NEXT, "malloc");
        real_free = dlsym(RTLD_NEXT, "free");
        real_calloc = dlsym(RTLD_NEXT, "calloc");
        real_realloc = dlsym(RTLD_NEXT, "realloc");
        initialized = 1;
    }
}

// Custom malloc wrapper
void* malloc(size_t size) {
    init_memory_hooks();
    
    if (!g_tracking_enabled) {
        return real_malloc(size);
    }
    
    // Allocate extra space for our header
    void* ptr = real_malloc(sizeof(alloc_header_t) + size);
    if (!ptr) return NULL;
    
    alloc_header_t* header = (alloc_header_t*)ptr;
    header->size = size;
    header->magic = MAGIC_VALUE;
    
    g_total_allocated += size;
    g_current_allocated += size;
    g_allocation_count++;
    
    if (g_current_allocated > g_peak_allocated) {
        g_peak_allocated = g_current_allocated;
    }
    
    return (char*)ptr + sizeof(alloc_header_t);
}

// Custom calloc wrapper
void* calloc(size_t nmemb, size_t size) {
    init_memory_hooks();
    
    if (!g_tracking_enabled) {
        return real_calloc(nmemb, size);
    }
    
    size_t total_size = nmemb * size;
    void* ptr = malloc(total_size);
    if (ptr) {
        memset(ptr, 0, total_size);
    }
    return ptr;
}

// Custom realloc wrapper
void* realloc(void* ptr, size_t size) {
    init_memory_hooks();
    
    if (!g_tracking_enabled) {
        return real_realloc(ptr, size);
    }
    
    if (!ptr) {
        return malloc(size);
    }
    
    // Get the old size
    alloc_header_t* header = (alloc_header_t*)((char*)ptr - sizeof(alloc_header_t));
    if (header->magic != MAGIC_VALUE) {
        // Not our allocation, use real realloc
        return real_realloc(ptr, size);
    }
    
    size_t old_size = header->size;
    
    // Allocate new memory
    void* new_ptr = malloc(size);
    if (!new_ptr) return NULL;
    
    // Copy old data
    memcpy(new_ptr, ptr, old_size < size ? old_size : size);
    
    // Free old memory
    free(ptr);
    
    return new_ptr;
}

// Custom free wrapper
void free(void* ptr) {
    init_memory_hooks();
    
    if (!ptr) return;
    
    if (!g_tracking_enabled) {
        real_free(ptr);
        return;
    }
    
    // Check if this is one of our allocations
    alloc_header_t* header = (alloc_header_t*)((char*)ptr - sizeof(alloc_header_t));
    if (header->magic != MAGIC_VALUE) {
        // Not our allocation, use real free
        real_free(ptr);
        return;
    }
    
    g_current_allocated -= header->size;
    g_free_count++;
    
    // Clear magic to detect double-free
    header->magic = 0;
    
    real_free(header);
}

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

// Setup expressions and parameters - called after context/builder creation
int embedded_pool_setup_expressions(void) {
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
    
    // Context and batch builder should already be created
    if (!g_pool.eval_context || !g_pool.batch_builder) {
        printf("Context or batch builder not initialized\n");
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
    
    // Reset memory tracking
    g_total_allocated = 0;
    g_current_allocated = 0;
    g_peak_allocated = 0;
    g_allocation_count = 0;
    g_free_count = 0;
    
    // Enable memory tracking
    g_tracking_enabled = 1;
    
    // Track memory during initialization phases
    printf("=== Memory Allocation Tracking ===\n\n");
    
    // Save initial state
    size_t pre_init_allocated = g_total_allocated;
    size_t pre_init_count = g_allocation_count;
    
    // Phase 1: Create context
    printf("Phase 1: Creating context...\n");
    g_pool.eval_context = exp_rs_context_new();
    if (!g_pool.eval_context) {
        printf("Failed to create context\n");
        return 1;
    }
    
    size_t context_allocated = g_total_allocated - pre_init_allocated;
    size_t context_count = g_allocation_count - pre_init_count;
    printf("  Context creation: %zu bytes in %zu allocations\n", 
           context_allocated, context_count);
    printf("  Total so far: %zu bytes\n\n", g_total_allocated);
    
    // Register required math functions
    printf("Phase 1b: Registering math functions...\n");
    size_t pre_register_allocated = g_total_allocated;
    size_t pre_register_count = g_allocation_count;
    
    EvalResult reg_result;
    reg_result = exp_rs_context_register_native_function(g_pool.eval_context, "sin", 1, native_sin);
    if (reg_result.status != 0) {
        printf("Failed to register sin function: %s\n", reg_result.error);
        exp_rs_free_error((char*)reg_result.error);
        return 1;
    }
    
    reg_result = exp_rs_context_register_native_function(g_pool.eval_context, "sqrt", 1, native_sqrt);
    if (reg_result.status != 0) {
        printf("Failed to register sqrt function: %s\n", reg_result.error);
        exp_rs_free_error((char*)reg_result.error);
        return 1;
    }
    
    size_t register_allocated = g_total_allocated - pre_register_allocated;
    size_t register_count = g_allocation_count - pre_register_count;
    printf("  Function registration: %zu bytes in %zu allocations\n", 
           register_allocated, register_count);
    printf("  Total so far: %zu bytes\n\n", g_total_allocated);
    
    // Phase 2: Create batch builder
    printf("Phase 2: Creating batch builder...\n");
    size_t pre_builder_allocated = g_total_allocated;
    size_t pre_builder_count = g_allocation_count;
    
    g_pool.batch_builder = exp_rs_batch_builder_new();
    if (!g_pool.batch_builder) {
        printf("Failed to create batch builder\n");
        exp_rs_context_free(g_pool.eval_context);
        return 1;
    }
    
    size_t builder_allocated = g_total_allocated - pre_builder_allocated;
    size_t builder_count = g_allocation_count - pre_builder_count;
    printf("  Batch builder creation: %zu bytes in %zu allocations\n", 
           builder_allocated, builder_count);
    printf("  Total so far: %zu bytes\n\n", g_total_allocated);
    
    // Phase 3: Add expressions and parameters
    printf("Phase 3: Adding expressions and parameters...\n");
    size_t pre_expr_allocated = g_total_allocated;
    size_t pre_expr_count = g_allocation_count;
    
    if (embedded_pool_setup_expressions() != 0) {
        printf("Failed to setup expressions\n");
        return 1;
    }
    
    size_t expr_allocated = g_total_allocated - pre_expr_allocated;
    size_t expr_count = g_allocation_count - pre_expr_count;
    printf("  Expression/parameter setup: %zu bytes in %zu allocations\n", 
           expr_allocated, expr_count);
    printf("  Total so far: %zu bytes\n\n", g_total_allocated);
    
    printf("=== Initialization Complete ===\n");
    printf("Total memory allocated: %zu bytes in %zu allocations\n", 
           g_total_allocated, g_allocation_count);
    printf("Current memory usage: %zu bytes\n\n", g_current_allocated);
    
    // Reset counters for runtime tracking
    size_t init_allocated = g_total_allocated;
    size_t init_current = g_current_allocated;
    g_total_allocated = 0;
    g_allocation_count = 0;
    g_free_count = 0;
    
    printf("All expressions parsed, tracking runtime allocations...\n\n");
    
    // Simulate 10 seconds of operation
    int iterations = UPDATE_RATE_HZ * 10;
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
        
        // Every 1000 iterations (1 second), print status
        if (i % 1000 == 999) {
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
    
    // Print memory statistics
    printf("\n=== Memory Usage Statistics ===\n");
    printf("Initial allocation during setup: %zu bytes\n", init_allocated);
    printf("Runtime allocations: %zu bytes in %zu allocations\n", 
           g_total_allocated, g_allocation_count);
    printf("Runtime deallocations: %zu frees\n", g_free_count);
    printf("Peak memory usage: %zu bytes\n", g_peak_allocated);
    printf("Current memory allocated: %zu bytes\n", g_current_allocated);
    printf("Memory leaked: %zu bytes\n", 
           g_current_allocated > init_current ? g_current_allocated - init_current : 0);
    
    // Verify final results
    embedded_pool_get_results(0, results);
    printf("\nFinal results:\n");
    for (int i = 0; i < NUM_EXPRESSIONS; i++) {
        printf("  Expression %d: %.6f\n", i, results[i]);
    }
    
    // Disable tracking before cleanup
    g_tracking_enabled = 0;
    
    // Cleanup
    embedded_pool_cleanup();
    printf("\nTest complete\n");
    
    return 0;
}