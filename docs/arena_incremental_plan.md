# Incremental Arena Implementation Plan

## Current Situation

The arena implementation requires updating:
- 59 calls to `parse_expression` 
- 42 calls to `eval_ast`
- All test files
- The entire evaluation engine

This is too large to do in one step without breaking everything.

## Revised Approach: Focused Implementation

### Step 1: Create Minimal Arena Path (Current Focus)

Instead of updating everything at once, create a minimal arena-based evaluation path:

1. **New Arena Module** (`src/arena.rs`)
   - Arena-aware parser wrapper
   - Arena-aware evaluator
   - Helper functions for tests

2. **Update Core Parser Functions**
   - Focus on getting `parse_primary()` working with arena
   - Add arena string allocation for variables/functions
   - Convert Vecs to arena slices

3. **Create Arena Batch Builder**
   - Complete the `ArenaBatchBuilder` implementation
   - Use only arena-based parsing

4. **FFI Integration**
   - Wire up `exp_rs_batch_builder_new_with_arena` to use `ArenaBatchBuilder`
   - This gives us a working arena path from C

### Step 2: Minimal Testing

1. Create integration test that uses arena from C
2. Verify zero allocations during evaluation
3. Benchmark against old implementation

### Step 3: Gradual Migration

Once we have a working arena path:
1. Update remaining parser functions incrementally
2. Convert tests one module at a time
3. Eventually remove non-arena code

## Benefits of This Approach

1. **Working Code Faster** - Get arena benefits without breaking everything
2. **Easier Testing** - Can compare arena vs non-arena side by side
3. **Lower Risk** - If something goes wrong, haven't broken entire codebase
4. **Clear Progress** - Can measure allocation reduction immediately

## Implementation Priority

1. Get `parse_primary()` to allocate strings in arena
2. Get `parse_function_call()` to use arena slices for args
3. Update `ArenaBatchBuilder::add_expression()` to actually parse into arena
4. Create simple C test to verify it works

This focused approach will deliver the performance benefits much sooner than trying to update the entire codebase at once.