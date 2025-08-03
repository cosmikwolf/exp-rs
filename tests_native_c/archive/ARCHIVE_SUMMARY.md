# Archived Tests Summary

This directory contains tests that have been archived due to redundancy, use of deprecated APIs, or being superseded by better implementations.

## Archived Tests

### Batch Evaluation Tests (Deprecated API)
1. **test_batch_with_sin.c** - Basic test covered by other tests, uses deprecated API
2. **test_batch_allocations.c** - Simple allocation tracking superseded by test_batch_builder_memory.c
3. **test_batch_eval.c** - Comprehensive but uses deprecated API (functionality should be migrated)
4. **test_batch_builder_memory.c** - Excellent memory leak detection but uses deprecated API (should be migrated)

### Timing Tests (Redundant)
5. **test_bulk_timing.c** - Superseded by test_realistic_timing.c (less precise)
6. **test_setup_timing.c** - Superseded by test_proper_timing.c
7. **test_setup_timing_precise.c** - Superseded by test_proper_timing.c
8. **test_timing_detailed.c** - Superseded by test_microbenchmark.c
9. **test_nanotime_setup.c** - Superseded by test_proper_timing.c

### Other Tests (Deprecated/Redundant)
10. **test_context_memory.c** - General memory analysis not arena-specific
11. **test_embedded_pool_.c** - Duplicate of test_embedded_pool.c with memory tracking
12. **test_simple_eval.c** - Uses deprecated API, functionality covered elsewhere
13. **test_simple_expr_alloc.c** - Allocation testing better covered in arena tests

## Tests to Migrate

The following tests contain valuable functionality that should be migrated to use the new arena-based API before permanent archival:

1. **test_batch_eval.c** - Comprehensive error handling and performance comparison tests
2. **test_batch_builder_memory.c** - Memory leak detection functionality

## Kept Tests

The following tests remain in the main test directory:

### Batch/Arena Tests
- `test_simple_batch.c` - Uses new arena-based API
- `test_direct_batch_alloc.c` - Tests direct batch evaluation API
- `test_arena_ffi.c` - Arena zero-allocation verification
- `test_arena_integration.c` - Comprehensive arena integration test

### Timing Tests
- `test_proper_timing.c` - Primary comprehensive timing test
- `test_realistic_timing.c` - Bulk/realistic usage patterns
- `test_detailed_timing.c` - Arena-specific performance testing
- `test_microbenchmark.c` - Individual operation microbenchmarks

### Embedded Tests
- `test_embedded_pool.c` - Embedded system use case demonstration

### Other
- `test_simple_eval.c` - Basic evaluation test (if updated to use new API)