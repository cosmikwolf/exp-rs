# Batch Evaluation API Implementation Plan

## Overview
This document outlines the plan for implementing batch evaluation functionality in exp-rs to improve performance when evaluating multiple expressions with multiple parameter sets.

## Design Goals
- **Single comprehensive API** instead of multiple functions for different cases
- **Flexible memory management** - users can choose who allocates
- **Granular error handling** - know exactly what failed where
- **Performance-focused** - minimize overhead, maximize throughput

## Data Structures

### BatchStatus (Error Tracking)
```c
typedef struct BatchStatus {
    int32_t code;           // 0 = success, non-zero = error
    size_t expr_index;      // Which expression failed
    size_t batch_index;     // Which batch item failed
} BatchStatus;
```

### BatchEvalRequest (Main Request)
```c
typedef struct BatchEvalRequest {
    // Expressions
    const char** expressions;
    size_t expression_count;
    
    // Parameters
    const char** param_names;
    size_t param_count;
    const Real** param_values;  // [param_idx][batch_idx]
    size_t batch_size;
    
    // Results (can be NULL for auto-allocation)
    Real** results;             // [expr_idx][batch_idx]
    
    // Options
    bool allocate_results;      // If true, library allocates results
    bool stop_on_error;         // If true, stop on first error
    
    // Error tracking (optional, can be NULL)
    BatchStatus* statuses;      // Array of batch_size * expression_count
} BatchEvalRequest;
```

### BatchEvalResult (For Library Allocation)
```c
typedef struct BatchEvalResult {
    Real** results;             // Allocated result arrays
    size_t expression_count;
    size_t batch_size;
    int32_t status;            // Overall status
} BatchEvalResult;
```

## API Functions

### Main Batch Evaluation Function
```c
// Primary batch evaluation function
int32_t exp_rs_batch_eval(
    const BatchEvalRequest* request,
    EvalContextOpaque* ctx
);
```

### Allocation Helper
```c
// Alternative that returns allocated results
int32_t exp_rs_batch_eval_alloc(
    const BatchEvalRequest* request,
    EvalContextOpaque* ctx,
    BatchEvalResult* result
);

// Free results allocated by library
void exp_rs_batch_free_results(
    BatchEvalResult* result
);
```

### Pre-compiled Expressions (Phase 2)
```c
// Pre-compile expressions for repeated use
BatchPreparedExprs* exp_rs_batch_prepare(
    const char** expressions,
    size_t expression_count,
    EvalContextOpaque* ctx
);

// Evaluate pre-compiled expressions
int32_t exp_rs_batch_eval_prepared(
    BatchPreparedExprs* prepared,
    const BatchEvalRequest* request,
    EvalContextOpaque* ctx
);

// Free pre-compiled expressions
void exp_rs_batch_free_prepared(
    BatchPreparedExprs* prepared
);
```

## Usage Examples

### Simple Case - Library Allocates
```c
BatchEvalRequest request = {
    .expressions = exprs,
    .expression_count = 3,
    .param_names = params,
    .param_count = 2,
    .param_values = param_vals,
    .batch_size = 1000,
    .results = NULL,
    .allocate_results = true,
    .stop_on_error = false,
    .statuses = NULL
};

BatchEvalResult result;
int status = exp_rs_batch_eval_alloc(&request, ctx, &result);
if (status == 0) {
    // Use result.results[expr_idx][batch_idx]
    exp_rs_batch_free_results(&result);
}
```

### Advanced Case - With Error Tracking
```c
BatchStatus* statuses = calloc(3 * 1000, sizeof(BatchStatus));
Real* results[3];
for (int i = 0; i < 3; i++) {
    results[i] = malloc(1000 * sizeof(Real));
}

BatchEvalRequest request = {
    .expressions = exprs,
    .expression_count = 3,
    .param_names = params,
    .param_count = 2,
    .param_values = param_vals,
    .batch_size = 1000,
    .results = results,
    .allocate_results = false,
    .stop_on_error = false,
    .statuses = statuses
};

int status = exp_rs_batch_eval(&request, ctx);
// Check individual statuses for partial failures
for (size_t i = 0; i < 3 * 1000; i++) {
    if (statuses[i].code != 0) {
        printf("Failed at expr %zu, batch %zu\n", 
               statuses[i].expr_index, 
               statuses[i].batch_index);
    }
}
```

### High-Performance Case - Pre-compiled
```c
// One-time setup
BatchPreparedExprs* prepared = exp_rs_batch_prepare(exprs, 3, ctx);

// Repeated evaluation with different data
for (int dataset = 0; dataset < 100; dataset++) {
    load_next_dataset(param_vals);
    
    BatchEvalRequest request = {
        .expressions = NULL,  // Not needed with prepared
        .expression_count = 3,
        .param_names = params,
        .param_count = 2,
        .param_values = param_vals,
        .batch_size = 1000,
        .results = results,
        .allocate_results = false,
        .stop_on_error = true,
        .statuses = NULL
    };
    
    exp_rs_batch_eval_prepared(prepared, &request, ctx);
}

exp_rs_batch_free_prepared(prepared);
```

## Implementation Strategy

### Phase 1: Core Functionality
1. Parse all expressions once (leverage AST cache)
2. Clone context to avoid modifying original
3. For each batch index:
   - Update all parameters in context
   - Evaluate all expressions
   - Store results/errors

### Phase 2: Optimizations
1. Reuse evaluation context across iterations
2. Minimize allocations
3. Pre-compiled expression support
4. Consider parallelization for large batches

## Error Handling

Two modes controlled by `stop_on_error`:
- **false**: Continue evaluation, record all errors in statuses array
- **true**: Stop at first error, return immediately

Error codes:
- 0: Success
- 1-99: Parsing errors
- 100-199: Evaluation errors
- 200+: System errors (allocation, etc.)

## Performance Expectations

Based on current analysis:
- **Current overhead**: ~50x slower than native (after AST caching)
- **Expected with batching**: ~15-30x slower than native
- **Improvement**: 40-70% performance gain

The improvement comes from:
1. Amortized FFI overhead across multiple evaluations
2. Single parse operation for all evaluations
3. Reduced context setup overhead
4. Better memory locality

## Testing Strategy

### Unit Tests (Rust)
- Edge cases (null pointers, empty arrays)
- Error propagation
- Memory management
- Partial evaluation with errors

### Integration Tests (C)
- Real expression evaluation
- Performance benchmarks against loop of individual calls
- Memory leak detection
- Stress testing with large batches

### Benchmark Tests
- Compare against current approach
- Measure improvement for different expression complexities
- Test scaling with batch size
- Profile memory usage

## Future Enhancements

1. **Parallel Evaluation**: Add threading support for large batches
2. **SIMD Optimization**: Vectorize simple expressions
3. **Streaming API**: For datasets that don't fit in memory
4. **Expression Templates**: Reusable patterns with placeholders
5. **Progress Callbacks**: For long-running batch operations
6. **Mixed Precision**: Support different precision per expression

## Open Questions

1. **Array Layout**: Is `[param_idx][batch_idx]` the most intuitive?
2. **NaN Handling**: Special status code for NaN results?
3. **Expression Validation**: Validate all before starting evaluation?
4. **Result Layout**: Row-major vs column-major for cache efficiency?
5. **API Naming**: Are the function names clear and consistent?