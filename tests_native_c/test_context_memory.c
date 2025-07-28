#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <time.h>
#include <dlfcn.h>
#include "include/exp_rs.h"

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

void reset_memory_tracking() {
    g_total_allocated = 0;
    g_current_allocated = 0;
    g_peak_allocated = 0;
    g_allocation_count = 0;
    g_free_count = 0;
}

void print_memory_stats(const char* phase) {
    printf("%-50s: %8zu bytes in %4zu allocations\n", phase, g_total_allocated, g_allocation_count);
}

int main() {
    printf("=== Context Memory Allocation Analysis ===\n\n");
    
    // Enable memory tracking
    g_tracking_enabled = 1;
    
    // Test 1: Create bare context (if we had a way to do it without default functions)
    printf("Phase 1: Basic Context Structure\n");
    reset_memory_tracking();
    void* ctx = exp_rs_context_new();
    print_memory_stats("Context with default functions");
    
    // Show breakdown
    printf("\nBreakdown estimate:\n");
    printf("  - Base context structure (Rc wrapper)\n");
    printf("  - Variable map (heapless IndexMap)\n");
    printf("  - Constant map (heapless IndexMap)\n");
    printf("  - Array map (heapless IndexMap)\n");
    printf("  - Attribute map (heapless IndexMap)\n");
    printf("  - Nested array map (heapless IndexMap)\n");
    printf("  - Function registry (Rc<FunctionRegistry>)\n");
    printf("  - Default math functions (~30-45 functions)\n");
    printf("\n");
    
    // Test 2: Add a parameter
    printf("Phase 2: Adding Parameters\n");
    reset_memory_tracking();
    exp_rs_context_set_parameter(ctx, "x", 1.0);
    print_memory_stats("Adding one parameter");
    
    // Test 3: Add multiple parameters
    reset_memory_tracking();
    for (int i = 0; i < 10; i++) {
        char name[8];
        sprintf(name, "p%d", i);
        exp_rs_context_set_parameter(ctx, name, (double)i);
    }
    print_memory_stats("Adding 10 more parameters");
    
    // Test 4: Register a custom function
    printf("\nPhase 3: Custom Functions\n");
    reset_memory_tracking();
    // Note: Native function registration requires a function pointer
    // which would add complexity to this test
    print_memory_stats("(Native function registration skipped)");
    
    // Test 5: Register an expression function
    reset_memory_tracking();
    const char* params[] = {"x", "y"};
    exp_rs_context_register_expression_function(ctx, "add", params, 2, "x + y");
    print_memory_stats("Register one expression function");
    
    // Show totals
    printf("\n=== Summary ===\n");
    printf("Total allocations: %zu\n", g_allocation_count);
    printf("Total deallocations: %zu\n", g_free_count);
    printf("Peak memory usage: %zu bytes\n", g_peak_allocated);
    printf("Current memory usage: %zu bytes\n", g_current_allocated);
    
    // Cleanup
    g_tracking_enabled = 0;
    exp_rs_context_free(ctx);
    
    return 0;
}