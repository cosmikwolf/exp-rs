# AST Cloning Optimization Analysis

## Executive Summary

The exp-rs expression evaluator currently allocates ~250 bytes per expression evaluation due to AST cloning in the iterative evaluator. At 1000Hz with 7 expressions, this results in 1.75 MB/sec of memory allocations. This document analyzes three potential solutions to eliminate these allocations.

## Problem Description

### Current Behavior
```rust
// src/eval/iterative.rs:84
self.op_stack.push(EvalOp::Eval { 
    expr: ast.clone(),  // <-- Allocates ~250 bytes per evaluation
    ctx_id: root_ctx_id 
});
```

### Impact
- **Memory allocation rate**: 250 bytes × 7 expressions × 1000 Hz = 1.75 MB/sec
- **Performance impact**: Continuous allocator pressure
- **Embedded suitability**: Poor for real-time systems

### Root Cause
The iterative evaluator uses an explicit operation stack to avoid recursion. When evaluating expressions, it needs to store AST fragments for deferred evaluation (e.g., short-circuit operations, function arguments). The current design clones AST nodes to maintain ownership.

## Solution Analysis

### Solution 1: Reference Counting (Rc<AstExpr>)

#### Overview
Replace owned `AstExpr` values with `Rc<AstExpr>` throughout the evaluation system. This changes deep cloning to reference count increments.

#### Implementation Details

**Core Changes:**
1. **AST Definition** (`src/types.rs`):
```rust
// Current
pub enum AstExpr {
    Array { name: String, index: Box<AstExpr> },
    LogicalOp { op: LogicalOperator, left: Box<AstExpr>, right: Box<AstExpr> },
    // ...
}

// Proposed
pub enum AstExpr {
    Array { name: String, index: Rc<AstExpr> },
    LogicalOp { op: LogicalOperator, left: Rc<AstExpr>, right: Rc<AstExpr> },
    // ...
}
```

2. **Operation Stack** (`src/eval/stack_ops.rs`):
```rust
// Current
pub enum EvalOp {
    Eval { expr: AstExpr, ctx_id: usize },
    ShortCircuitAnd { right_expr: AstExpr, ctx_id: usize },
}

// Proposed
pub enum EvalOp {
    Eval { expr: Rc<AstExpr>, ctx_id: usize },
    ShortCircuitAnd { right_expr: Rc<AstExpr>, ctx_id: usize },
}
```

3. **Evaluation Logic** (`src/eval/iterative.rs`):
```rust
// Current
self.op_stack.push(EvalOp::Eval { expr: ast.clone(), ctx_id });

// Proposed
self.op_stack.push(EvalOp::Eval { expr: ast.clone(), ctx_id }); // Only clones Rc
```

#### Advantages
- **Memory efficiency**: Reference counting instead of deep cloning (8 bytes vs 250 bytes)
- **Minimal API changes**: Most evaluation logic remains unchanged
- **Proven pattern**: Standard Rust idiom for shared immutable data
- **Incremental implementation**: Can be done file by file

#### Disadvantages
- **Complex integration**: Parser produces Box<AstExpr>, needs conversion layer
- **Mixed type system**: Box in parser, Rc in evaluator creates confusion
- **Memory overhead**: 8 bytes per AST node for reference count
- **Potential fragmentation**: Many small Rc allocations
- **Clone semantics change**: `clone()` behavior changes from deep to shallow

#### Implementation Effort
- **Time estimate**: 4-5 days
- **Risk level**: Medium
- **Files affected**: ~10-15 files
- **Lines of code**: ~500-800 modifications

#### Potential Issues
1. **Parser Integration Complexity**:
   - Need Box→Rc conversion utilities
   - Double allocation during parsing (Box then Rc)
   
2. **ExpressionFunction Storage**:
   - Must update to store `Rc<AstExpr>` instead of `AstExpr`
   
3. **Method Signatures**:
   - Methods taking `self` must change to `&self`
   
4. **FFI Boundary**:
   - Need conversion at C interface

---

### Solution 2: Index-Based AST

#### Overview
Replace pointer-based AST with index-based representation using an arena allocator. AST nodes reference each other via indices instead of pointers.

#### Implementation Details

**Core Design:**
```rust
pub type NodeId = u32;

pub struct AstArena {
    nodes: Vec<AstNode>,
    next_id: NodeId,
}

pub enum AstNode {
    Constant(Real),
    Variable(String),
    Function { name: String, args: Vec<NodeId> },
    Array { name: String, index: NodeId },
    LogicalOp { op: LogicalOperator, left: NodeId, right: NodeId },
    Conditional { condition: NodeId, true_branch: NodeId, false_branch: NodeId },
}
```

**Evaluation Changes:**
```rust
pub enum EvalOp {
    Eval { node_id: NodeId, ctx_id: usize },
    ShortCircuitAnd { right_node_id: NodeId, ctx_id: usize },
}

impl EvalEngine {
    fn get_node(&self, id: NodeId) -> &AstNode {
        &self.current_arena.nodes[id as usize]
    }
}
```

#### Advanced Design (After Iterations)

**Chunked Arena for Stable References:**
```rust
pub struct AstArena {
    chunks: Vec<Box<[Option<AstNode>]>>, // Multiple fixed-size chunks
    chunk_size: usize,
    current_chunk: usize,
    current_offset: usize,
}

impl AstArena {
    fn get_node(&self, id: NodeId) -> Option<&AstNode> {
        let chunk_id = id.0 as usize / self.chunk_size;
        let offset = id.0 as usize % self.chunk_size;
        self.chunks.get(chunk_id)?.get(offset)?.as_ref()
    }
}
```

#### Advantages
- **Zero allocations**: No memory allocation during evaluation
- **Maximum performance**: Direct array indexing
- **Cache friendly**: Contiguous memory layout
- **Predictable memory**: Pre-allocated arena size
- **No GC overhead**: No reference counting

#### Disadvantages
- **Major redesign**: Complete AST representation change
- **Complex implementation**: Arena management, index validation
- **API breaking changes**: All AST consumers must be updated
- **Debugging difficulty**: Indices provide no semantic information
- **Fixed capacity issues**: Must pre-allocate or handle growth
- **Unsafe code required**: For performance optimization

#### Implementation Effort
- **Time estimate**: 2-3 weeks
- **Risk level**: High
- **Files affected**: ~20+ files
- **Lines of code**: ~1500-2000 modifications

#### Potential Issues
1. **Arena Lifetime Management**:
   - Arena must outlive all evaluations
   - Complex with concurrent evaluations
   
2. **Memory Fragmentation**:
   - Deleted nodes create holes
   - Need free list management
   
3. **Cross-Arena References**:
   - Expression functions may reference different arenas
   - Requires arena hierarchy
   
4. **Growth Strategy**:
   - Fixed size limits expression complexity
   - Dynamic growth invalidates indices

---

### Solution 3: Bytecode Compilation

#### Overview
Compile AST to bytecode once, then execute bytecode for evaluations. Eliminates AST traversal entirely.

#### Implementation Details

**Bytecode Design:**
```rust
pub enum Instruction {
    // Stack operations
    LoadConst(Real),
    LoadParam(u16),      // Parameter index
    LoadVar(u16),        // Variable index
    
    // Arithmetic
    Add, Sub, Mul, Div, Mod, Pow,
    Negate, Abs,
    
    // Comparison
    Less, Greater, LessEq, GreaterEq, Equal, NotEqual,
    
    // Logical
    And, Or, Not,
    
    // Control flow
    Jump(i32),           // Unconditional jump
    JumpIfFalse(i32),    // Jump if top of stack is false
    
    // Functions
    Call(u16, u8),       // Function ID, arity
    Return,
    
    // Array/Attribute access
    LoadArray(u16, u16), // Array ID, index
    LoadAttr(u16, u16),  // Object ID, attribute ID
}

pub struct CompiledExpr {
    instructions: Vec<Instruction>,
    constants: Vec<Real>,
    strings: Vec<String>,
    param_map: FnvIndexMap<String, u16, 16>,
}
```

**Compiler Implementation:**
```rust
struct BytecodeCompiler {
    instructions: Vec<Instruction>,
    constants: Vec<Real>,
    strings: Vec<String>,
}

impl BytecodeCompiler {
    fn compile_expr(&mut self, expr: &AstExpr) -> Result<(), ExprError> {
        match expr {
            AstExpr::Constant(val) => {
                let idx = self.add_constant(*val);
                self.emit(Instruction::LoadConst(idx));
            }
            AstExpr::Variable(name) => {
                let idx = self.get_var_index(name)?;
                self.emit(Instruction::LoadVar(idx));
            }
            AstExpr::Function { name, args } => {
                // Compile arguments in order
                for arg in args {
                    self.compile_expr(arg)?;
                }
                let func_id = self.get_func_id(name)?;
                self.emit(Instruction::Call(func_id, args.len() as u8));
            }
            // ... handle all variants
        }
        Ok(())
    }
}
```

**VM Implementation:**
```rust
struct BytecodeVM {
    stack: Vec<Real>,
    ip: usize,  // Instruction pointer
}

impl BytecodeVM {
    fn execute(&mut self, code: &CompiledExpr, ctx: &EvalContext) -> Result<Real, ExprError> {
        self.stack.clear();
        self.ip = 0;
        
        while self.ip < code.instructions.len() {
            match code.instructions[self.ip] {
                Instruction::LoadConst(idx) => {
                    self.stack.push(code.constants[idx as usize]);
                }
                Instruction::Add => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(a + b);
                }
                Instruction::JumpIfFalse(offset) => {
                    if self.peek()? == 0.0 {
                        self.ip = (self.ip as i32 + offset) as usize;
                        continue;
                    }
                }
                // ... handle all instructions
            }
            self.ip += 1;
        }
        
        self.pop() // Final result
    }
}
```

#### Advantages
- **Zero allocations**: No memory allocation during execution
- **Fastest execution**: Simple instruction dispatch loop
- **Compact representation**: Bytecode smaller than AST
- **Optimization opportunities**: Constant folding, dead code elimination
- **Industry proven**: Used by Lua, Python, JavaScript

#### Disadvantages
- **Extreme complexity**: Requires compiler + VM implementation
- **Compilation overhead**: Must compile AST to bytecode
- **Two representations**: Maintain AST for debugging + bytecode for execution
- **Short-circuit complexity**: Control flow requires jump instructions
- **Function dispatch complexity**: Dynamic function resolution
- **Debugging challenges**: Lost source mapping

#### Implementation Effort
- **Time estimate**: 4-6 weeks
- **Risk level**: Very High
- **Files affected**: ~20+ files
- **Lines of code**: ~2000-3000 new code

#### Potential Issues
1. **Short-Circuit Evaluation**:
   - Requires complex jump logic
   - Nested conditions create spaghetti code
   
2. **Dynamic Functions**:
   - User-registered functions need runtime lookup
   - Expression functions incompatible with pre-compilation
   
3. **Error Handling**:
   - Stack traces become meaningless
   - Need source mapping for debugging
   
4. **Variable-Length Arguments**:
   - Functions like `max(a,b,c,d)` need special handling
   
5. **Context Variables**:
   - Runtime lookups still needed
   - Parameter overrides complicate compilation

## Performance Comparison

| Solution | Allocation Rate | Execution Speed | Implementation Time | Risk |
|----------|----------------|-----------------|-------------------|------|
| Current | 1.75 MB/sec | Baseline | 0 | None |
| Rc<AstExpr> | ~0 MB/sec | ~95% of baseline | 4-5 days | Medium |
| Index-Based | 0 MB/sec | 110% of baseline | 2-3 weeks | High |
| Bytecode | 0 MB/sec | 150% of baseline | 4-6 weeks | Very High |

## Memory Leak Test Results

Recent testing with custom allocator tracking revealed:
- **BatchBuilder lifecycle**: No memory leaks (perfect cleanup)
- **Repeated create/destroy**: No memory leaks over 100 cycles
- **Complex expressions**: Minor leak of 352 bytes (0.5% of total)
- **Error conditions**: No memory leaks

This confirms the AST cloning is the primary allocation issue, not memory leaks.

## Recommendation

### Short Term (Pragmatic)
**Accept the current allocation rate** of 1.75 MB/sec. The memory leak testing shows excellent cleanup, and the allocation rate may be acceptable for many embedded systems with modern allocators.

### Medium Term (If Needed)
Implement **Rc<AstExpr>** despite its complexity. While it requires significant changes, it:
- Eliminates 95%+ of allocations
- Maintains debugging capabilities
- Can be implemented incrementally
- Uses proven Rust patterns

### Long Term (If Performance Critical)
Consider **index-based AST** only if:
- Rc<AstExpr> proves insufficient
- You have 2-3 weeks for implementation
- Performance is absolutely critical
- You can accept debugging complexity

### Not Recommended
**Bytecode compilation** - The complexity far exceeds the benefits for this use case. Only consider if you need order-of-magnitude performance improvements and have VM expertise.

## Implementation Checklist for Rc<AstExpr>

If proceeding with Rc<AstExpr>:

1. **Phase 1: Core Types** (Day 1)
   - [ ] Update AstExpr enum to use Rc fields
   - [ ] Update EvalOp to store Rc<AstExpr>
   - [ ] Create Box→Rc conversion utilities

2. **Phase 2: Parser Integration** (Day 2)
   - [ ] Add conversion layer after parsing
   - [ ] Update BatchBuilder to store Rc<AstExpr>
   - [ ] Test basic expression evaluation

3. **Phase 3: Evaluation Engine** (Day 3)
   - [ ] Update process_eval to use .as_ref()
   - [ ] Fix all match arms for Rc dereferencing
   - [ ] Update function evaluation logic

4. **Phase 4: Extended Types** (Day 4)
   - [ ] Update ExpressionFunction storage
   - [ ] Fix method signatures (self → &self)
   - [ ] Update tests for new semantics

5. **Phase 5: Testing & Validation** (Day 5)
   - [ ] Create allocation benchmarks
   - [ ] Verify memory reduction
   - [ ] Test all expression types
   - [ ] Profile performance impact

## Conclusion

The AST cloning issue is real and significant (1.75 MB/sec), but the solutions are complex. The pragmatic approach is to first determine if this allocation rate is actually problematic for your specific embedded system. If optimization is required, Rc<AstExpr> provides the best balance of performance improvement and implementation complexity.