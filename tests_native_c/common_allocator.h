#ifndef COMMON_ALLOCATOR_H
#define COMMON_ALLOCATOR_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// Custom allocator functions required by exp-rs when built with custom_cbindgen_alloc feature
void* exp_rs_malloc(size_t size);
void exp_rs_free(void* ptr);

// Memory tracking functionality (optional - only used by memory management tests)
void init_memory_tracking(void);
void enable_allocation_tracking(void);
void disable_allocation_tracking(void);
void reset_memory_stats(void);

typedef struct {
    size_t total_allocs;
    size_t total_deallocs;
    size_t current_bytes;
    size_t peak_bytes;
    size_t total_allocated_bytes;
    size_t leaked_allocs;
} memory_stats_t;

memory_stats_t get_memory_stats(void);
void print_memory_stats(const char* phase);

#ifdef __cplusplus
}
#endif

#endif // COMMON_ALLOCATOR_H