# Arena Implementation Progress

## Completed Tasks

### Phase 1: AST Type Changes ✓
- Added lifetime parameter `'arena` to `AstExpr` enum
- Changed all string fields from `String` to `&'arena str`
- Changed all recursive fields from `Box<AstExpr>` to `&'arena AstExpr<'arena>`
- Changed `Vec` fields to `&'arena [AstExpr<'arena>]`
- Removed `Clone`, `Debug`, `PartialEq` derives from `AstExpr`
- Removed `compiled_ast` field from `ExpressionFunction`
- Removed AST cache from `EvalContext`

### Phase 2: Arena Infrastructure ✓
- Added bumpalo dependency to Cargo.toml
- Created FFI arena management functions:
  - `exp_rs_arena_new(size_hint)` - Creates new arena
  - `exp_rs_arena_free(arena)` - Frees arena
  - `exp_rs_arena_reset(arena)` - Resets arena for reuse
  - `exp_rs_estimate_arena_size(expressions, count, iterations)` - Estimates needed size
  - `exp_rs_batch_builder_new_with_arena(arena)` - Creates arena-aware batch builder

## In Progress

### Phase 3: Parser Updates (Started)
- Added `arena: Option<&'arena Bump>` field to `PrattParser`
- Added lifetime parameters `'input` and `'arena` to parser
- Created `new_with_arena()` constructor
- Need to update all AST node creation to use arena when available

### Phase 2.2: BatchBuilder Updates (Started)
- Created `ArenaBatchBuilder<'arena>` struct
- Added basic structure and methods
- Blocked on parser arena support

## Pending Tasks

### Parser Completion
- Update all `AstExpr::` node creation to check for arena
- Implement string allocation in arena (`self.arena.alloc_str()`)
- Convert `Vec` to arena slices
- Handle both arena and non-arena modes

### Expression Functions
- Convert to template pattern (store source only)
- Compile on-demand into arena

### Evaluation Engine
- Update `EvalOp` to use references
- Remove AST cloning in iterative evaluator
- Update stack operations

### Test Infrastructure
- Create test helpers for arena-based tests
- Update existing tests to work with lifetimes

## Current Blockers

1. **Lifetime Errors**: Many functions throughout the codebase need lifetime parameters added
2. **Parser Complexity**: Need to support both arena and non-arena modes during transition
3. **Test Compatibility**: Tests need significant updates to work with arena lifetimes

## Next Steps

1. Focus on getting a minimal arena-based parser working
2. Create a simple arena-based evaluation test
3. Gradually convert remaining components
4. Update tests incrementally

## Notes

- The implementation is following the plan from `/docs/arena_implementation_plan.md`
- Breaking changes are acceptable as confirmed by the user
- Focus is on eliminating the ~1,364 byte allocation per evaluation