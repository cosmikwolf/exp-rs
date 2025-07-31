# Arena Implementation Summary

## What Has Been Accomplished

### 1. Core Type Changes ✅
- Added lifetime parameter `'arena` to `AstExpr` enum in `types.rs`
- Changed all owned types to arena references:
  - `String` → `&'arena str`
  - `Box<AstExpr>` → `&'arena AstExpr<'arena>`
  - `Vec<AstExpr>` → `&'arena [AstExpr<'arena>]`
- Removed `Clone`, `Debug`, `PartialEq` derives that don't work with references

### 2. Context Cleanup ✅
- Removed AST cache from `EvalContext` (incompatible with arena lifetimes)
- Removed `compiled_ast` field from `ExpressionFunction`
- Simplified the context to prepare for arena usage

### 3. FFI Arena Infrastructure ✅
Created complete C API for arena management:
```c
// Arena lifecycle
Arena* exp_rs_arena_new(size_t size_hint);
void exp_rs_arena_free(Arena* arena);
void exp_rs_arena_reset(Arena* arena);

// Helper functions
size_t exp_rs_estimate_arena_size(expressions, count, iterations);
BatchBuilder* exp_rs_batch_builder_new_with_arena(Arena* arena);
```

### 4. Parser Preparation ✅
- Added arena field to `PrattParser`
- Updated parser struct with lifetime parameters
- Created arena-aware parse function signatures

### 5. BatchBuilder Skeleton ✅
- Created `ArenaBatchBuilder<'arena>` structure
- Defined the interface for arena-based batch evaluation

## What Remains To Be Done

### 1. Parser Implementation 🔧
The parser needs to be updated to actually use the arena:
- Update all AST node creation to allocate strings with `arena.alloc_str()`
- Convert `Vec` to `bumpalo::collections::Vec` and then to slices
- Update all return types to include lifetime

### 2. Evaluation Engine Updates 🔧
- Update `EvalOp` enum to use references instead of owned values
- Remove the critical `ast.clone()` in `iterative.rs:84`
- Update all evaluation functions to work with borrowed ASTs

### 3. Widespread Refactoring 🔧
- 59 calls to `parse_expression` need updating
- 42 calls to `eval_ast` need lifetime parameters
- All tests need arena setup

## The Core Challenge

The main issue is that adding lifetimes to `AstExpr` creates a cascade of required changes throughout the codebase. Every function that touches an AST now needs lifetime parameters, which affects:
- Parser (all parsing functions)
- Evaluator (all eval functions)
- Tests (all test setup)
- FFI (lifetime management across language boundary)

## Recommended Path Forward

### Option 1: Complete the Full Refactoring
Continue updating all ~100+ affected functions to support arena lifetimes. This is the "correct" approach but requires significant effort.

### Option 2: Hybrid Approach
1. Keep the current non-arena code working
2. Add a parallel arena-based API (like we started with `parse_expression_arena`)
3. Use arena only in the critical path (batch evaluation from C)
4. Gradually migrate other uses

### Option 3: Alternative Memory Strategy
Instead of arena allocation, consider:
- Reference counting with `Rc` (but this isn't no_std compatible)
- Custom allocator that pools AST nodes
- Pre-allocated AST node pool with indices

## Performance Impact

The goal is to eliminate ~1,364 byte allocations per expression evaluation. With arena allocation:
- Parse once into arena: ~1,364 bytes allocated
- Evaluate 1000 times: 0 additional bytes
- Total savings: ~1.36 MB for 1000 evaluations

## Conclusion

The arena implementation is architecturally sound and the infrastructure is in place. The main challenge is the extensive refactoring required due to lifetime propagation. The FFI functions are ready, but need the Rust implementation to be completed to be functional.

For immediate performance benefits, focusing on just the batch evaluation path (Option 2) might be the most practical approach.