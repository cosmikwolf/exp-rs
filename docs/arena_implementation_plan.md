# Arena-Based AST Implementation Plan for exp-rs

## Executive Summary

This document outlines a comprehensive plan to eliminate all dynamic memory allocations during expression evaluation in exp-rs by implementing arena-based memory management. The goal is to reduce memory bandwidth from 10 MB/s to ~1 MB/s when evaluating expressions at 1000Hz on embedded systems (STM32H7).

## Problem Statement

Currently, exp-rs allocates ~1,364 bytes per expression evaluation due to AST cloning in `src/eval/iterative.rs:84`. For a system running at 1000Hz with 7 expressions, this results in ~10 MB/s of memory traffic, which is excessive for embedded systems with limited memory bandwidth.

## Solution Overview

Implement an arena-first approach where the AST type (`AstExpr`) is modified to use arena allocation throughout. All strings and recursive structures will be allocated in a bump allocator arena, eliminating all dynamic allocations during evaluation.

## Key Design Decisions

1. **Arena-First**: Modify `AstExpr` directly rather than creating a separate arena type
2. **Single-Threaded**: Leverage single-threaded execution for simpler arena management  
3. **FFI Compatible**: Arena allocated via `exp_rs_malloc` for memory section control
4. **Breaking Changes Allowed**: Library is for a single project, allowing optimal design

## Implementation Plan

### Phase 1: Add Arena Lifetime to AST

#### 1.1 Modify AstExpr Type
```rust
// Before
pub enum AstExpr {
    Constant(Real),
    Variable(String),
    Function {
        name: String,
        args: Vec<AstExpr>,
    },
    LogicalOp {
        op: LogicalOperator,
        left: Box<AstExpr>,
        right: Box<AstExpr>,
    },
    // ... etc
}

// After
pub enum AstExpr<'arena> {
    Constant(Real),
    Variable(&'arena str),
    Function {
        name: &'arena str,
        args: &'arena [AstExpr<'arena>],
    },
    LogicalOp {
        op: LogicalOperator,
        left: &'arena AstExpr<'arena>,
        right: &'arena AstExpr<'arena>,
    },
    // ... etc
}
```

#### 1.2 Remove Derive Traits
Remove `Clone`, `Debug`, `PartialEq` derives from `AstExpr` as they won't work with arena references. Implement manually where needed.

#### 1.3 Remove AST Cache
Delete the AST cache entirely from `EvalContext` - it's barely used and incompatible with arena allocation.

### Phase 2: Create Arena Infrastructure

#### 2.1 FFI Arena Functions
```rust
// Opaque arena type for C
#[repr(C)]
pub struct ArenaOpaque {
    _private: [u8; 0],
}

#[no_mangle]
pub extern "C" fn exp_rs_arena_new(size_hint: usize) -> *mut ArenaOpaque {
    // Uses exp_rs_malloc internally via global allocator
    let arena = Box::new(Bump::with_capacity(size_hint));
    Box::into_raw(arena) as *mut ArenaOpaque
}

#[no_mangle]
pub extern "C" fn exp_rs_arena_free(arena: *mut ArenaOpaque) {
    if !arena.is_null() {
        unsafe {
            let _ = Box::from_raw(arena as *mut Bump);
        }
    }
}

#[no_mangle]
pub extern "C" fn exp_rs_arena_reset(arena: *mut ArenaOpaque) {
    if !arena.is_null() {
        unsafe {
            let arena = &mut *(arena as *mut Bump);
            arena.reset();
        }
    }
}
```

#### 2.2 Update BatchBuilder
```rust
pub struct BatchBuilder<'arena> {
    arena: &'arena Bump,
    expressions: Vec<(&'arena str, &'arena AstExpr<'arena>)>,
    compiled_functions: HashMap<&'arena str, CompiledFunction<'arena>>,
    engine: ArenaEvalEngine<'arena>,
    // ...
}

#[no_mangle]
pub extern "C" fn exp_rs_batch_builder_new_with_arena(
    arena: *mut ArenaOpaque
) -> *mut BatchBuilderOpaque {
    let arena = unsafe { &*(arena as *const Bump) };
    let builder = BatchBuilder::new(arena);
    Box::into_raw(Box::new(builder)) as *mut BatchBuilderOpaque
}
```

### Phase 3: Update Parser

#### 3.1 Add Arena to Parser Struct
```rust
struct PrattParser<'input, 'arena> {
    lexer: Lexer<'input>,
    arena: &'arena Bump,  // Just add this field
    current: Option<Token>,
    errors: Vec<ExprError>,
    // ... existing fields unchanged
}
```

#### 3.2 Update Parse Functions
Only the top-level parse function signatures change:
```rust
pub fn parse_expression<'arena>(
    input: &str,
    arena: &'arena Bump,
) -> Result<AstExpr<'arena>, ExprError> {
    let mut parser = PrattParser::new(input, arena);
    parser.parse()
}
```

#### 3.3 Update String Allocations
Throughout the parser, change:
- `name.clone()` → `self.arena.alloc_str(name)`
- `Vec::new()` → `bumpalo::collections::Vec::new_in(self.arena)`
- `.collect::<Vec<_>>()` → `.collect::<Vec<_>>().into_bump_slice()`

### Phase 4: Expression Function Changes

#### 4.1 Remove Compiled AST Storage
```rust
// Before
pub struct ExpressionFunction {
    pub params: Vec<String>,
    pub expression: String,
    pub compiled_ast: AstExpr,  // Remove this
}

// After
pub struct ExpressionFunction {
    pub params: Vec<String>,
    pub expression: String,  // Keep source only
}
```

#### 4.2 Compile Functions On-Demand
```rust
impl BatchBuilder<'arena> {
    fn compile_expression_function(
        &self,
        func: &ExpressionFunction
    ) -> Result<CompiledFunction<'arena>, ExprError> {
        let ast = parse_expression_arena(&func.expression, self.arena)?;
        Ok(CompiledFunction {
            params: /* allocate in arena */,
            ast,
        })
    }
}
```

### Phase 5: Update Evaluation Engine

#### 5.1 Modify EvalOp
```rust
// Before
pub enum EvalOp {
    Eval { 
        expr: AstExpr,  // Owned
        ctx_id: usize 
    },
    // ...
}

// After
pub enum EvalOp<'arena> {
    Eval { 
        expr: &'arena AstExpr<'arena>,  // Reference
        ctx_id: usize 
    },
    // ...
}
```

#### 5.2 Remove All Cloning
The critical line that clones ASTs (`src/eval/iterative.rs:84`) becomes:
```rust
// Before
self.op_stack.push(EvalOp::Eval { 
    expr: ast.clone(),  // PROBLEM: 1,364 byte allocation
    ctx_id: root_ctx_id 
});

// After  
self.op_stack.push(EvalOp::Eval { 
    expr: ast,  // Just a reference, no allocation!
    ctx_id: root_ctx_id 
});
```

### Phase 6: Test Infrastructure

#### 6.1 Test Helper Functions
```rust
#[cfg(test)]
thread_local! {
    static TEST_ARENA: Bump = Bump::new();
}

#[cfg(test)]
fn with_test_arena<F, R>(f: F) -> R 
where F: FnOnce(&Bump) -> R 
{
    TEST_ARENA.with(|arena| {
        arena.reset();
        f(arena)
    })
}

#[cfg(test)]
fn test_parse(expr: &str) -> AstExpr<'static> {
    TEST_ARENA.with(|arena| {
        // Unsafe: we know TEST_ARENA is 'static
        unsafe {
            let arena_ref = &*(arena as *const Bump);
            parse_expression_arena(expr, arena_ref).unwrap()
        }
    })
}
```

## Usage Pattern from C

```c
// One-time setup
Arena* arena = exp_rs_arena_new(256 * 1024);  // 256KB

// For each batch of evaluations
exp_rs_arena_reset(arena);

// Create batch builder with arena
BatchBuilder* builder = exp_rs_batch_builder_new_with_arena(arena);

// Add expressions (parsed into arena)
exp_rs_batch_builder_add_expression(builder, "x + y");
exp_rs_batch_builder_add_expression(builder, "sin(x) * cos(y)");

// Add parameters
int x_idx = exp_rs_batch_builder_add_parameter(builder, "x", 0.0);
int y_idx = exp_rs_batch_builder_add_parameter(builder, "y", 0.0);

// Evaluate many times with different parameters
for (int i = 0; i < 1000; i++) {
    exp_rs_batch_builder_set_param(builder, x_idx, sensor_data[i].x);
    exp_rs_batch_builder_set_param(builder, y_idx, sensor_data[i].y);
    exp_rs_batch_builder_eval(builder, context);
    
    // Get results
    float result1 = exp_rs_batch_builder_get_result(builder, 0);
    float result2 = exp_rs_batch_builder_get_result(builder, 1);
}

// Cleanup
exp_rs_batch_builder_free(builder);
exp_rs_arena_free(arena);
```

## Memory Layout

With arena allocation, memory layout becomes cache-friendly:

```
Arena Memory Layout:
[AstExpr nodes...][Strings...][Function arrays...][Temp buffers...]
^                                                                  ^
|                    Sequential allocation                         |
```

All related data is allocated sequentially, improving cache locality.

## Performance Expectations

1. **Memory Allocation**: 0 bytes per evaluation (down from 1,364)
2. **Memory Bandwidth**: ~1 MB/s (down from 10 MB/s)  
3. **Cache Performance**: Improved due to sequential layout
4. **Evaluation Speed**: Faster due to no allocation overhead

## Known Limitations

1. All expressions in a batch must use the same arena
2. Cannot serialize arena-allocated ASTs
3. Arena size must be pre-determined
4. Test code needs updating to use arena

## Risk Mitigation

1. **Test Updates**: Create helper functions to minimize changes
2. **Gradual Migration**: Can be done in phases
3. **Compatibility**: Old API can coexist during transition
4. **Memory Sizing**: Provide estimation helpers

## Implementation Order

1. **Phase 1**: AST type changes (mechanical refactoring)
2. **Phase 2**: Arena infrastructure (new code)
3. **Phase 3**: Parser updates (add arena field)
4. **Phase 4**: Expression function changes (remove storage)
5. **Phase 5**: Evaluation engine (remove cloning)
6. **Phase 6**: Test updates (use helpers)

## Success Criteria

1. Zero allocations during expression evaluation
2. Memory bandwidth under 1 MB/s at 1000Hz
3. All tests passing with arena allocation
4. Clean FFI interface for arena management
5. No performance regressions

## Conclusion

This plan provides a clear path to eliminate all dynamic allocations during expression evaluation. The arena-first approach is simpler than alternatives and provides optimal performance for embedded systems. With 94% confidence in the implementation approach, this represents a straightforward but impactful optimization for exp-rs.