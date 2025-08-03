# Archived Tests

This directory contains tests that have been archived because their functionality is already covered by other, more comprehensive tests.

## Archived Tests

### test_direct_batch_alloc.c
**Archived Date**: 2025-08-03
**Reason**: 
- Uses deprecated `exp_rs_batch_eval_with_context` API that returns -99 (not implemented)
- Attempts memory tracking via dlsym malloc interception which is unreliable
- Zero-allocation behavior is better tested in `test_embedded_pool.c` and `test_arena_integration.c`

### test_arena_ffi.c  
**Archived Date**: 2025-08-03
**Reason**:
- Contains broken allocation tracking code (custom_malloc is never set as the allocator)
- All valid test scenarios are covered by `test_arena_integration.c`:
  - Arena creation/destruction → `test_arena_lifecycle()`
  - Zero-allocation evaluation → `test_zero_allocations()` 
  - Arena reset functionality → `test_arena_reset_reuse()`
  - Arena size estimation → `test_arena_size_estimation()`

## Coverage Analysis

The remaining test suite provides complete coverage:
- `test_arena_integration.c` - Comprehensive arena and batch evaluation testing
- `test_embedded_pool.c` - Real-world embedded system patterns
- `test_simple_batch.c` - Basic "hello world" example (needs assertion fixes)
- Various timing tests - Performance characterization at different granularities

No functionality has been lost by archiving these tests.