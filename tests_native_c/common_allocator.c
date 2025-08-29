#include "common_allocator.h"
#include <stdlib.h>
#include <stdio.h>
#include <stdatomic.h>
#include <stdbool.h>

// ============================================================================
// Memory Tracking System (Advanced)
// ============================================================================

// Memory tracking statistics
static atomic_size_t total_allocations = 0;
static atomic_size_t total_deallocations = 0;
static atomic_size_t current_bytes = 0;
static atomic_size_t peak_bytes = 0;
static atomic_size_t total_allocated_bytes = 0;
static atomic_size_t total_deallocated_bytes = 0;
static atomic_bool tracking_enabled = false;

// Memory allocation tracking structure
typedef struct {
    size_t size;
    size_t magic; // For corruption detection
} alloc_header_t;

#define ALLOC_MAGIC 0xDEADBEEF
#define HEADER_SIZE sizeof(alloc_header_t)

// Store original malloc/free for fallback
static void* (*original_malloc)(size_t) = NULL;
static void (*original_free)(void*) = NULL;
static bool tracking_initialized = false;

#ifdef EXP_RS_CUSTOM_ALLOC
// Heap memory for Rust allocator (allocated once, reused)
static void* rust_heap_memory = NULL;
static size_t rust_heap_size = 1024 * 1024; // 1MB
#endif

// Forward declarations for Rust allocator tracking functions
// These are always available regardless of custom_cbindgen_alloc feature
extern size_t exp_rs_get_total_allocated(void);
extern size_t exp_rs_get_total_freed(void);
extern size_t exp_rs_get_allocation_count(void);
extern size_t exp_rs_get_free_count(void);
extern size_t exp_rs_get_current_allocated(void);

// Track whether we're using custom allocator or system allocator
// We detect this by checking if our exp_rs_malloc is being called
static atomic_bool custom_allocator_used = false;

// Check if we're using custom allocator
bool using_custom_allocator() {
    return atomic_load(&custom_allocator_used);
}

// ============================================================================
// Core Allocator Implementation
// ============================================================================

// Initialize memory tracking
void init_memory_tracking() {
    if (!tracking_initialized) {
        #ifdef EXP_RS_CUSTOM_ALLOC
        // Allocate heap memory for Rust allocator if not already allocated
        if (rust_heap_memory == NULL) {
            rust_heap_memory = malloc(rust_heap_size);
            if (rust_heap_memory == NULL) {
                // Failed to allocate heap memory
                printf("ERROR: Failed to allocate heap memory\n");
                return;
            }
        }
        
        // Initialize the TlsfHeap if custom allocator will be used
        // We do this early since any malloc call might trigger Rust allocations
        exp_rs_heap_init((uint8_t*)rust_heap_memory, rust_heap_size);
        #endif
        
        // Use dlsym to get the real malloc/free functions
        // This bypasses any potential symbol conflicts
        #ifdef __APPLE__
        original_malloc = malloc;  // On macOS, this should work
        original_free = free;
        #else
        // On Linux, you might need:
        // original_malloc = dlsym(RTLD_NEXT, "malloc");
        // original_free = dlsym(RTLD_NEXT, "free");
        original_malloc = malloc;
        original_free = free;
        #endif
        tracking_initialized = true;
    }
    
    // Reset counters
    atomic_store(&total_allocations, 0);
    atomic_store(&total_deallocations, 0);
    atomic_store(&current_bytes, 0);
    atomic_store(&peak_bytes, 0);
    atomic_store(&total_allocated_bytes, 0);
    atomic_store(&total_deallocated_bytes, 0);
}

// Enable/disable allocation tracking
void enable_allocation_tracking() {
    atomic_store(&tracking_enabled, true);
}

void disable_allocation_tracking() {
    atomic_store(&tracking_enabled, false);
}

// Custom malloc implementation with tracking
static void* tracked_malloc(size_t size) {
    void* ptr = original_malloc(size + HEADER_SIZE);
    
    if (ptr && atomic_load(&tracking_enabled)) {
        alloc_header_t* header = (alloc_header_t*)ptr;
        header->size = size;
        header->magic = ALLOC_MAGIC;
        
        // Update statistics atomically
        atomic_fetch_add(&total_allocations, 1);
        atomic_fetch_add(&total_allocated_bytes, size);
        
        size_t new_current = atomic_fetch_add(&current_bytes, size) + size;
        
        // Update peak if necessary
        size_t current_peak = atomic_load(&peak_bytes);
        while (new_current > current_peak) {
            if (atomic_compare_exchange_weak(&peak_bytes, &current_peak, new_current)) {
                break;
            }
        }
        
        return (char*)ptr + HEADER_SIZE;
    }
    
    return ptr;
}

// Custom free implementation with tracking
static void tracked_free(void* ptr) {
    if (!ptr) return;
    
    if (atomic_load(&tracking_enabled)) {
        alloc_header_t* header = (alloc_header_t*)((char*)ptr - HEADER_SIZE);
        
        // Verify magic number
        if (header->magic == ALLOC_MAGIC) {
            size_t size = header->size;
            header->magic = 0; // Clear magic to detect double-free
            
            // Update statistics
            atomic_fetch_add(&total_deallocations, 1);
            atomic_fetch_add(&total_deallocated_bytes, size);
            atomic_fetch_sub(&current_bytes, size);
            
            original_free(header);
        } else {
            // Not our allocation or corrupted - use original free
            original_free(ptr);
        }
    } else {
        original_free(ptr);
    }
}

// ============================================================================
// Public API Implementation
// ============================================================================

// ============================================================================
// Dual-Mode Allocator Support
// ============================================================================

// Required by exp-rs custom allocator (when custom_cbindgen_alloc is enabled)
void* exp_rs_malloc(size_t size) {
    // Mark that custom allocator is being used
    atomic_store(&custom_allocator_used, true);
    
    if (!tracking_initialized) {
        init_memory_tracking();
    }
    
    if (atomic_load(&tracking_enabled)) {
        return tracked_malloc(size);
    } else {
        // Simple pass-through when tracking is disabled
        return malloc(size);
    }
}

void exp_rs_free(void* ptr) {
    // Mark that custom allocator is being used
    atomic_store(&custom_allocator_used, true);
    
    if (!tracking_initialized) {
        init_memory_tracking();
    }
    
    if (atomic_load(&tracking_enabled)) {
        tracked_free(ptr);
    } else {
        // Simple pass-through when tracking is disabled
        free(ptr);
    }
}

// ============================================================================
// System Allocator Tracking (for when custom_cbindgen_alloc is disabled)
// ============================================================================

// When custom allocator is disabled, we get allocation stats from Rust's built-in tracking
// No need to intercept malloc/free - we query Rust directly

// Stubs for API compatibility (not used when system allocator is active)
void mark_rust_allocation_start() {
    // No-op when using system allocator - Rust tracks its own allocations
}

void mark_rust_allocation_end() {
    // No-op when using system allocator
}

// Get memory statistics - works with both custom and system allocator
memory_stats_t get_memory_stats() {
    memory_stats_t stats;
    
    if (using_custom_allocator()) {
        // Use our C-side tracking when custom allocator is active
        stats.total_allocs = atomic_load(&total_allocations);
        stats.total_deallocs = atomic_load(&total_deallocations);
        stats.current_bytes = atomic_load(&current_bytes);
        stats.peak_bytes = atomic_load(&peak_bytes);
        stats.total_allocated_bytes = atomic_load(&total_allocated_bytes);
        stats.total_deallocated_bytes = atomic_load(&total_deallocated_bytes);
        stats.leaked_allocs = stats.total_allocs - stats.total_deallocs;
    } else {
        // Use Rust-side tracking when system allocator is active
        stats.total_allocs = exp_rs_get_allocation_count();
        stats.total_deallocs = exp_rs_get_free_count();
        stats.current_bytes = exp_rs_get_current_allocated();
        stats.peak_bytes = 0; // Rust doesn't track peak - would need to be added
        stats.total_allocated_bytes = exp_rs_get_total_allocated();
        stats.total_deallocated_bytes = exp_rs_get_total_freed();
        stats.leaked_allocs = stats.total_allocs - stats.total_deallocs;
    }
    
    return stats;
}

void print_memory_stats(const char* phase) {
    memory_stats_t stats = get_memory_stats();
    printf("Memory Stats [%s]:\n", phase);
    printf("  Allocations: %zu\n", stats.total_allocs);
    printf("  Deallocations: %zu\n", stats.total_deallocs);
    printf("  Current bytes: %zu\n", stats.current_bytes);
    printf("  Peak bytes: %zu (%.1f KB)\n", stats.peak_bytes, stats.peak_bytes / 1024.0);
    printf("  Total allocated: %zu (%.1f KB)\n", stats.total_allocated_bytes, stats.total_allocated_bytes / 1024.0);
    printf("  Leaked allocations: %zu\n", stats.leaked_allocs);
}

// Reset memory statistics
void reset_memory_stats() {
    atomic_store(&total_allocations, 0);
    atomic_store(&total_deallocations, 0);
    atomic_store(&current_bytes, 0);
    atomic_store(&peak_bytes, 0);
    atomic_store(&total_allocated_bytes, 0);
    atomic_store(&total_deallocated_bytes, 0);
}