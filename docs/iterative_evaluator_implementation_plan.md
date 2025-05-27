# Iterative Evaluator Implementation Plan

## Overview
Replace the recursive AST evaluation with an iterative approach using an explicit stack to eliminate stack overflow issues and improve performance, especially for embedded systems.

## Goals
1. **Eliminate stack overflow** from deep recursion
2. **Improve performance** by 2-3x for nested expressions
3. **Reduce memory usage** by 10-100x for stack space
4. **Maintain compatibility** with existing API
5. **Enable predictable memory usage** for embedded systems

## Implementation Strategy

### Phase 1: Design the Stack-Based Evaluation System

#### 1.1 Define Operation Types
```rust
// src/eval/stack_ops.rs
enum EvalOp {
    // Push an expression to evaluate
    Eval { expr: AstExpr, ctx_id: usize },
    
    // Apply a unary operation
    ApplyUnary { op: UnaryOp },
    
    // Apply a binary operation (left operand already computed)
    ApplyBinary { op: BinaryOp, left: Real },
    
    // Complete a binary operation
    CompleteBinary { op: BinaryOp },
    
    // Apply a function with N arguments
    ApplyFunction { 
        name: FunctionName,
        args_needed: usize,
        args_collected: Vec<Real>,
        ctx_id: usize,
    },
    
    // Handle ternary operator
    ApplyTernary { 
        condition: Real,
        false_branch: AstExpr,
        ctx_id: usize,
    },
    
    // Variable lookup
    LookupVariable { name: HString, ctx_id: usize },
    
    // Array access
    AccessArray { array_name: HString, ctx_id: usize },
    
    // Attribute access
    AccessAttribute { 
        object_name: HString,
        attr_name: HString,
        ctx_id: usize,
    },
}
```

#### 1.2 Context Management
```rust
// Track contexts without recursion
struct ContextStack {
    contexts: Vec<EvalContext>,
    // Map context IDs to parent IDs for variable lookup chains
    parent_map: FnvIndexMap<usize, Option<usize>, 32>,
}
```

### Phase 2: Implement Core Evaluator

#### 2.1 Main Evaluation Loop
```rust
// src/eval/iterative.rs
pub fn eval_iterative(
    ast: &AstExpr,
    ctx: Option<Rc<EvalContext>>,
) -> Result<Real, ExprError> {
    const MAX_STACK_DEPTH: usize = 100;
    const INITIAL_CAPACITY: usize = 32;
    
    let mut op_stack: Vec<EvalOp> = Vec::with_capacity(INITIAL_CAPACITY);
    let mut value_stack: Vec<Real> = Vec::with_capacity(INITIAL_CAPACITY);
    let mut ctx_stack = ContextStack::new();
    
    // Initialize with root context
    let root_ctx_id = ctx_stack.push_context(ctx);
    op_stack.push(EvalOp::Eval { 
        expr: ast.clone(), 
        ctx_id: root_ctx_id 
    });
    
    while let Some(op) = op_stack.pop() {
        // Check depth limit
        if op_stack.len() > MAX_STACK_DEPTH {
            return Err(ExprError::RecursionLimit(
                format!("Maximum evaluation depth {} exceeded", MAX_STACK_DEPTH)
            ));
        }
        
        match op {
            EvalOp::Eval { expr, ctx_id } => {
                process_eval_op(expr, ctx_id, &mut op_stack, &mut value_stack)?;
            }
            EvalOp::ApplyUnary { op } => {
                let operand = value_stack.pop()
                    .ok_or(ExprError::InternalError("Stack underflow"))?;
                value_stack.push(apply_unary_op(op, operand)?);
            }
            // ... handle other operations
        }
    }
    
    value_stack.pop()
        .ok_or(ExprError::InternalError("No result on stack"))
}
```

#### 2.2 Expression Processing
```rust
fn process_eval_op(
    expr: AstExpr,
    ctx_id: usize,
    op_stack: &mut Vec<EvalOp>,
    value_stack: &mut Vec<Real>,
) -> Result<(), ExprError> {
    match expr {
        AstExpr::Constant(val) => {
            value_stack.push(val);
        }
        
        AstExpr::Variable(name) => {
            op_stack.push(EvalOp::LookupVariable { name, ctx_id });
        }
        
        AstExpr::UnaryOp { op, operand } => {
            // Push operations in reverse order
            op_stack.push(EvalOp::ApplyUnary { op });
            op_stack.push(EvalOp::Eval { 
                expr: *operand, 
                ctx_id 
            });
        }
        
        AstExpr::BinaryOp { op, left, right } => {
            // Evaluate left, then right, then apply
            op_stack.push(EvalOp::CompleteBinary { op });
            op_stack.push(EvalOp::Eval { expr: *right, ctx_id });
            op_stack.push(EvalOp::Eval { expr: *left, ctx_id });
        }
        
        AstExpr::Function { name, args } => {
            if args.is_empty() {
                op_stack.push(EvalOp::ApplyFunction {
                    name,
                    args_needed: 0,
                    args_collected: vec![],
                    ctx_id,
                });
            } else {
                // Push function application
                op_stack.push(EvalOp::ApplyFunction {
                    name,
                    args_needed: args.len(),
                    args_collected: vec![],
                    ctx_id,
                });
                // Push argument evaluations in reverse order
                for arg in args.into_iter().rev() {
                    op_stack.push(EvalOp::Eval { expr: arg, ctx_id });
                }
            }
        }
        
        // ... handle other expression types
    }
    Ok(())
}
```

### Phase 3: Handle Special Cases

#### 3.1 Expression Functions
Expression functions need special handling to create new contexts:

```rust
fn apply_expression_function(
    func: &ExpressionFunction,
    args: Vec<Real>,
    ctx_stack: &mut ContextStack,
    op_stack: &mut Vec<EvalOp>,
    parent_ctx_id: usize,
) -> Result<(), ExprError> {
    // Create new context for function
    let mut func_ctx = EvalContext::new();
    
    // Set parameters
    for (param, value) in func.params.iter().zip(args.iter()) {
        func_ctx.set_parameter(param, *value)?;
    }
    
    // Copy function registry from parent
    if let Some(parent_ctx) = ctx_stack.get_context(parent_ctx_id) {
        func_ctx.function_registry = parent_ctx.function_registry.clone();
    }
    
    // Push new context and evaluate body
    let func_ctx_id = ctx_stack.push_context_with_parent(func_ctx, parent_ctx_id);
    op_stack.push(EvalOp::Eval {
        expr: func.compiled_ast.clone(),
        ctx_id: func_ctx_id,
    });
    
    Ok(())
}
```

#### 3.2 Short-Circuit Evaluation
Handle && and || operators with proper short-circuiting:

```rust
EvalOp::ShortCircuitAnd { right_expr, ctx_id } => {
    let left_val = value_stack.pop().unwrap();
    if left_val == 0.0 {
        // Short circuit - don't evaluate right
        value_stack.push(0.0);
    } else {
        // Need to evaluate right
        op_stack.push(EvalOp::CompleteAnd);
        op_stack.push(EvalOp::Eval { 
            expr: right_expr, 
            ctx_id 
        });
    }
}
```

### Phase 4: Integration

#### 4.1 Update Public API
```rust
// src/eval/mod.rs
pub fn eval_ast(
    ast: &AstExpr,
    ctx: Option<Rc<EvalContext>>,
) -> Result<Real, ExprError> {
    // Use iterative evaluator
    iterative::eval_iterative(ast, ctx)
}
```

#### 4.2 Migration Path
1. Implement iterative evaluator alongside recursive
2. Add feature flag to switch between them
3. Run comprehensive benchmarks
4. Gradually migrate tests
5. Remove recursive implementation

### Phase 5: Optimization

#### 5.1 Memory Pool for Stack Operations
```rust
// Reuse allocations
struct EvalEngine {
    op_stack: Vec<EvalOp>,
    value_stack: Vec<Real>,
    ctx_stack: ContextStack,
}

impl EvalEngine {
    fn eval(&mut self, ast: &AstExpr, ctx: Option<Rc<EvalContext>>) -> Result<Real, ExprError> {
        // Clear stacks but keep capacity
        self.op_stack.clear();
        self.value_stack.clear();
        self.ctx_stack.clear();
        
        // ... evaluation logic
    }
}
```

#### 5.2 Inline Hot Functions
- Mark frequently-called functions with `#[inline]`
- Use profile-guided optimization

#### 5.3 Cache Optimization
- Ensure stack structures fit in L1/L2 cache
- Use capacity hints based on typical expression depth

### Phase 6: Testing

#### 6.1 Test Cases
1. **Correctness Tests**
   - All existing tests must pass
   - Deep nesting (factorial, fibonacci)
   - Complex expressions with all operators
   - Function calls with multiple arguments
   - Variable scoping

2. **Performance Tests**
   - Benchmark vs recursive implementation
   - Memory usage comparison
   - Cache miss analysis

3. **Edge Cases**
   - Empty expressions
   - Single constants
   - Maximum depth handling
   - Stack underflow/overflow

#### 6.2 Benchmarking
```rust
#[bench]
fn bench_deep_nesting_iterative(b: &mut Bencher) {
    let expr = create_deeply_nested_expr(50);
    let ctx = create_test_context();
    b.iter(|| {
        eval_iterative(&expr, Some(Rc::new(ctx.clone())))
    });
}
```

### Phase 7: Documentation

#### 7.1 Update Documentation
- Explain the iterative approach
- Document performance characteristics
- Update examples

#### 7.2 Migration Guide
- How to update custom evaluation code
- Performance tuning guide
- Debugging tips

## Timeline Estimate

1. **Week 1**: Design and implement core evaluator (Phase 1-2)
2. **Week 2**: Handle special cases and integration (Phase 3-4)
3. **Week 3**: Optimization and testing (Phase 5-6)
4. **Week 4**: Documentation and final testing (Phase 7)

## Risk Mitigation

1. **Compatibility**: Keep recursive implementation during migration
2. **Complexity**: Start with simple expressions, add features incrementally
3. **Performance**: Profile early and often
4. **Memory**: Monitor stack usage in embedded environment

## Success Metrics

1. **No stack overflows** for expressions up to depth 1000
2. **2-3x performance improvement** for nested expressions
3. **50% less memory usage** overall
4. **All tests passing** with identical results
5. **Predictable max memory usage** (important for embedded)

## Alternative Approaches Considered

1. **Trampolining**: Rejected due to allocation overhead
2. **Continuation Passing**: Too complex for marginal benefit
3. **Hybrid approach**: Possible future optimization

## Conclusion

The iterative evaluator will provide significant performance and reliability improvements, especially for embedded systems. The implementation is straightforward and maintains API compatibility while solving the stack overflow problem permanently.