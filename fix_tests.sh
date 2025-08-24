#!/bin/bash

# Fix test_pattern_isolation.c
sed -i '' 's/ExprArena\* arena = expr_arena_new(16384);/\/\/ Arena is now managed internally by batch/' tests_native_c/test_pattern_isolation.c
sed -i '' 's/ExprBatch\* audio_pattern = expr_batch_new(arena);/ExprBatch* audio_pattern = expr_batch_new(16384);/' tests_native_c/test_pattern_isolation.c
sed -i '' 's/ExprBatch\* video_pattern = expr_batch_new(arena);/ExprBatch* video_pattern = expr_batch_new(16384);/' tests_native_c/test_pattern_isolation.c
sed -i '' 's/ExprBatch\* sensor_pattern = expr_batch_new(arena);/ExprBatch* sensor_pattern = expr_batch_new(16384);/' tests_native_c/test_pattern_isolation.c
sed -i '' 's/ExprBatch\* dynamic_pattern = expr_batch_new(arena);/ExprBatch* dynamic_pattern = expr_batch_new(16384);/' tests_native_c/test_pattern_isolation.c
sed -i '' '/expr_arena_free(arena);/d' tests_native_c/test_pattern_isolation.c

# Fix test_memory_analysis.c
sed -i '' 's/ExprArena\* arena = expr_arena_new(4 \* 1024);/\/\/ Arena is now managed internally by batch/' tests_native_c/test_memory_analysis.c
sed -i '' 's/ExprBatch\* builder = expr_batch_new(arena);/ExprBatch* builder = expr_batch_new(4 * 1024);/' tests_native_c/test_memory_analysis.c
sed -i '' '/expr_arena_free(arena);/d' tests_native_c/test_memory_analysis.c

# Fix test_panic_handler.c
sed -i '' 's/ExprArena\* arena = expr_arena_new(8192);/\/\/ Arena is now managed internally by batch/' tests_native_c/test_panic_handler.c
sed -i '' 's/ExprBatch\* batch = expr_batch_new(arena);/ExprBatch* batch = expr_batch_new(8192);/' tests_native_c/test_panic_handler.c
sed -i '' '/expr_arena_free(arena);/d' tests_native_c/test_panic_handler.c

# Fix test_batch_memory_tracking.c - this one needs more complex fixes
# We'll handle it separately

# Fix test_memory_management.c
sed -i '' 's/ExprArena\* arena = expr_arena_new(/\/\/ Arena managed internally: /' tests_native_c/test_memory_management.c
sed -i '' 's/ExprArena\* test_arena = expr_arena_new(/\/\/ Arena managed internally: /' tests_native_c/test_memory_management.c
sed -i '' 's/ExprArena\* parse_arena = expr_arena_new(/\/\/ Arena managed internally: /' tests_native_c/test_memory_management.c
sed -i '' '/expr_arena_free(/d' tests_native_c/test_memory_management.c
sed -i '' '/expr_arena_reset(/d' tests_native_c/test_memory_management.c
sed -i '' 's/assert(arena != NULL);/\/\/ Arena check removed/' tests_native_c/test_memory_management.c
sed -i '' 's/ExprBatch\* builder = expr_batch_new(arena);/ExprBatch* builder = expr_batch_new(256 * 1024);/' tests_native_c/test_memory_management.c

# Fix test_error_handling.c
sed -i '' 's/ExprArena\* arena = expr_arena_new(/\/\/ Arena managed internally: /' tests_native_c/test_error_handling.c
sed -i '' 's/ExprBatch\* batch = expr_batch_new(arena);/ExprBatch* batch = expr_batch_new(8192);/' tests_native_c/test_error_handling.c
sed -i '' 's/batch = expr_batch_new(arena);/batch = expr_batch_new(1024);/' tests_native_c/test_error_handling.c
sed -i '' '/expr_arena_free(/d' tests_native_c/test_error_handling.c
sed -i '' 's/expr_batch_new(NULL)/expr_batch_new(0)/' tests_native_c/test_error_handling.c

# Fix test_performance.c
sed -i '' 's/ExprArena\* arena = expr_arena_new(/\/\/ Arena managed internally: /' tests_native_c/test_performance.c
sed -i '' 's/ExprArena\* test_arena = expr_arena_new(/\/\/ Arena managed internally: /' tests_native_c/test_performance.c
sed -i '' 's/ExprArena\* parse_arena = expr_arena_new(/\/\/ Arena managed internally: /' tests_native_c/test_performance.c
sed -i '' 's/ExprBatch\* builder = expr_batch_new(test_arena);/ExprBatch* builder = expr_batch_new(32768);/' tests_native_c/test_performance.c
sed -i '' 's/ExprBatch\* builder = expr_batch_new(parse_arena);/ExprBatch* builder = expr_batch_new(32768);/' tests_native_c/test_performance.c
sed -i '' '/expr_arena_free(/d' tests_native_c/test_performance.c
sed -i '' '/expr_arena_reset(/d' tests_native_c/test_performance.c

echo "Test files updated to use new single-object API"