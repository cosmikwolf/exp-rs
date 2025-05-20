# Logical Operators Implementation Plan

This document outlines a comprehensive plan for implementing short-circuit logical operators (`&&` and `||`) in the expression evaluator. The implementation will be done in a single shot, covering all necessary components.

## Overview

Currently, the expression evaluator supports comparison operators but lacks logical operators with short-circuit evaluation. This implementation will add `&&` (logical AND) and `||` (logical OR) operators with proper short-circuit semantics.

Short-circuit evaluation means:
- For `a && b`: If `a` is false, `b` is not evaluated at all.
- For `a || b`: If `a` is true, `b` is not evaluated at all.

## Implementation Components

### 1. AST Structure Changes

Add a new AST node type for logical operations:

```rust
enum AstExpr {
    // Existing variants
    Constant(Real),
    Variable(String),
    Function { name: String, args: Vec<AstExpr> },
    Array { name: String, index: Box<AstExpr> },
    Attribute { base: String, attr: String },
    
    // New logical operation node
    LogicalOp { 
        op: LogicalOperator, 
        left: Box<AstExpr>, 
        right: Box<AstExpr> 
    },
}

enum LogicalOperator {
    And,  // Represents &&
    Or,   // Represents ||
}
```

Implement `Debug` and `Clone` traits for the new types:

```rust
impl fmt::Debug for LogicalOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogicalOperator::And => write!(f, "&&"),
            LogicalOperator::Or => write!(f, "||"),
        }
    }
}

// Clone should be derivable
```

### 2. Parser Modifications

Modify the `parse_infix_operators` method in `PrattParser` to handle logical operators specially:

```rust
fn parse_infix_operators(
    &mut self,
    mut lhs: AstExpr,
    min_bp: u8,
    allow_comma: bool,
) -> Result<AstExpr, ExprError> {
    loop {
        // Get the next operator
        let op_text = if let Some(tok) = self.peek() {
            // ... existing code to get operator text
        } else {
            break;
        };

        // Special case for logical operators
        if op_text == "&&" || op_text == "||" {
            // Get binding power - these are already defined in get_binding_power
            let Some(bp) = Self::get_binding_power(op_text) else {
                break;
            };

            // Check minimum binding power
            if bp.left < min_bp {
                break;
            }

            // Consume the operator
            self.next();

            // Parse the right side with appropriate binding power
            let rhs = self.parse_expr_unified(bp.right, allow_comma)?;

            // Create a LogicalOp node instead of a Function node
            lhs = AstExpr::LogicalOp { 
                op: if op_text == "&&" { LogicalOperator::And } else { LogicalOperator::Or },
                left: Box::new(lhs),
                right: Box::new(rhs)
            };
            continue;
        }

        // Normal operator handling for non-logical operators
        // ... existing code
    }
    Ok(lhs)
}
```

Ensure the binding powers in `get_binding_power` are appropriate:
```rust
// These should already exist but verify precedence is correct
"||" => Some(BindingPower::left_assoc(2)),  // Lower precedence
"&&" => Some(BindingPower::left_assoc(3)),  // Higher precedence
```

### 3. Evaluation Logic

Add handling for `LogicalOp` nodes in `eval_ast_inner`:

```rust
match ast {
    // ... existing cases
    
    AstExpr::LogicalOp { op, left, right } => {
        // Track recursion depth as needed
        let should_track = true; // Logical ops should track recursion
        
        if should_track {
            check_and_increment_recursion_depth()?;
        }
        
        // Store result to ensure we always decrement the counter if needed
        let result = match op {
            LogicalOperator::And => {
                // Evaluate left side first
                let left_val = eval_ast_inner(left, ctx.clone(), func_cache, var_cache)?;
                
                // Short-circuit if left is false (0.0)
                if left_val == 0.0 {
                    Ok(0.0)
                } else {
                    // Only evaluate right side if left is true (non-zero)
                    let right_val = eval_ast_inner(right, ctx, func_cache, var_cache)?;
                    // Result is true (1.0) only if both are true (non-zero)
                    Ok(if right_val != 0.0 { 1.0 } else { 0.0 })
                }
            },
            LogicalOperator::Or => {
                // Evaluate left side first
                let left_val = eval_ast_inner(left, ctx.clone(), func_cache, var_cache)?;
                
                // Short-circuit if left is true (non-zero)
                if left_val != 0.0 {
                    Ok(1.0)
                } else {
                    // Only evaluate right side if left is false (zero)
                    let right_val = eval_ast_inner(right, ctx, func_cache, var_cache)?;
                    // Result is true (1.0) if either is true (non-zero)
                    Ok(if right_val != 0.0 { 1.0 } else { 0.0 })
                }
            }
        };
        
        // Only decrement if we incremented
        if should_track {
            decrement_recursion_depth();
        }
        
        result
    },
    
    // ... other cases
}
```

## Testing Strategy

### Test Categories

1. **Basic Logical Operations**:
   - Test simple AND/OR expressions with constant values
   - Verify truth tables for both operators

2. **Short-Circuit Evaluation**:
   - Test that right operands aren't evaluated when not needed
   - Use functions with side effects to verify

3. **Operator Precedence**:
   - Test that AND has higher precedence than OR
   - Test combinations with comparison operators

4. **Complex Expressions**:
   - Test nested logical expressions
   - Test expressions with variables and functions

5. **Performance Tests**:
   - Compare parsing and evaluation times before and after

### Test Cases

Create a new file `tests/logical_operators_test.rs` with these tests:

```rust
#[test]
fn test_basic_logical_operations() {
    // Test AND operator
    assert_eq!(interp("1 && 1", None).unwrap(), 1.0);
    assert_eq!(interp("1 && 0", None).unwrap(), 0.0);
    assert_eq!(interp("0 && 1", None).unwrap(), 0.0);
    assert_eq!(interp("0 && 0", None).unwrap(), 0.0);
    
    // Test OR operator
    assert_eq!(interp("1 || 1", None).unwrap(), 1.0);
    assert_eq!(interp("1 || 0", None).unwrap(), 1.0);
    assert_eq!(interp("0 || 1", None).unwrap(), 1.0);
    assert_eq!(interp("0 || 0", None).unwrap(), 0.0);
    
    // Test with actual boolean results from comparisons
    assert_eq!(interp("(5 > 3) && (2 < 4)", None).unwrap(), 1.0);
    assert_eq!(interp("(5 < 3) || (2 > 4)", None).unwrap(), 0.0);
    assert_eq!(interp("(5 > 3) || (2 > 4)", None).unwrap(), 1.0);
}

#[test]
fn test_short_circuit_evaluation() {
    use exp_rs::context::EvalContext;
    use std::rc::Rc;
    use std::cell::RefCell;
    
    // Use RefCell to track function evaluation
    let eval_count = Rc::new(RefCell::new(0));
    
    // Create context with functions that have side effects
    let mut ctx = EvalContext::new();
    
    // Clone Rc for use in closures
    let count1 = Rc::clone(&eval_count);
    ctx.register_native_function("increment_and_return_true", 0, move |_| {
        *count1.borrow_mut() += 1;
        1.0
    });
    
    let count2 = Rc::clone(&eval_count);
    ctx.register_native_function("increment_and_return_false", 0, move |_| {
        *count2.borrow_mut() += 1;
        0.0
    });
    
    let ctx_rc = Rc::new(ctx);
    
    // Test AND short-circuit
    *eval_count.borrow_mut() = 0;
    let result = interp("0 && increment_and_return_true()", Some(ctx_rc.clone())).unwrap();
    assert_eq!(result, 0.0);
    assert_eq!(*eval_count.borrow(), 0, "Right side of AND should not be evaluated when left is false");
    
    // Test OR short-circuit
    *eval_count.borrow_mut() = 0;
    let result = interp("1 || increment_and_return_false()", Some(ctx_rc.clone())).unwrap();
    assert_eq!(result, 1.0);
    assert_eq!(*eval_count.borrow(), 0, "Right side of OR should not be evaluated when left is true");
    
    // Verify non-short-circuit cases
    *eval_count.borrow_mut() = 0;
    let result = interp("1 && increment_and_return_true()", Some(ctx_rc.clone())).unwrap();
    assert_eq!(result, 1.0);
    assert_eq!(*eval_count.borrow(), 1, "Right side of AND should be evaluated when left is true");
    
    *eval_count.borrow_mut() = 0;
    let result = interp("0 || increment_and_return_false()", Some(ctx_rc.clone())).unwrap();
    assert_eq!(result, 0.0);
    assert_eq!(*eval_count.borrow(), 1, "Right side of OR should be evaluated when left is false");
}

#[test]
fn test_operator_precedence() {
    // Verify AND has higher precedence than OR
    assert_eq!(interp("0 && 0 || 1", None).unwrap(), 1.0); // (0 && 0) || 1
    assert_eq!(interp("1 || 0 && 0", None).unwrap(), 1.0); // 1 || (0 && 0)
    
    // Verify comparison operators have higher precedence than logical operators
    assert_eq!(interp("5 > 3 && 2 < 4", None).unwrap(), 1.0); // (5 > 3) && (2 < 4)
    assert_eq!(interp("5 > 3 || 2 > 4", None).unwrap(), 1.0); // (5 > 3) || (2 > 4)
    
    // Test with parentheses to force evaluation order
    assert_eq!(interp("0 && (0 || 1)", None).unwrap(), 0.0);
    assert_eq!(interp("(1 || 0) && 0", None).unwrap(), 0.0);
}

#[test]
fn test_complex_logical_expressions() {
    // Test nested expressions
    assert_eq!(interp("(1 && 1) && (1 && 1)", None).unwrap(), 1.0);
    assert_eq!(interp("(1 || 0) && (0 || 1)", None).unwrap(), 1.0);
    assert_eq!(interp("(0 || 0) || (0 || 0)", None).unwrap(), 0.0);
    
    // Test with variables
    use exp_rs::context::EvalContext;
    use std::rc::Rc;
    
    let mut ctx = EvalContext::new();
    ctx.set_parameter("x", 5.0);
    ctx.set_parameter("y", -3.0);
    
    let ctx_rc = Rc::new(ctx);
    
    // Test complex expressions with variables
    assert_eq!(interp("x > 0 && y < 0", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("x < 0 || y < 0", Some(ctx_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("(x > 0 && y > 0) || (x < 0 && y < 0)", Some(ctx_rc.clone())).unwrap(), 0.0);
    
    // Test in custom functions
    let mut ctx2 = EvalContext::new();
    ctx2.register_expression_function("is_between", &["x", "min", "max"], "x >= min && x <= max").unwrap();
    
    let ctx2_rc = Rc::new(ctx2);
    
    assert_eq!(interp("is_between(5, 1, 10)", Some(ctx2_rc.clone())).unwrap(), 1.0);
    assert_eq!(interp("is_between(0, 1, 10)", Some(ctx2_rc.clone())).unwrap(), 0.0);
    assert_eq!(interp("is_between(5, 10, 20)", Some(ctx2_rc.clone())).unwrap(), 0.0);
}

#[test]
fn test_recursion_with_logical_operators() {
    // Test that factorial with a logical guard correctly terminates
    use exp_rs::context::EvalContext;
    use std::rc::Rc;
    
    let mut ctx = EvalContext::new();
    
    // Create a properly guarded recursive factorial function
    ctx.register_expression_function(
        "factorial",
        &["n"],
        "n <= 1 ? 1 : n * factorial(n-1)",  // Ternary isn't implemented, so next line instead
    ).unwrap_err();  // This should fail because we don't support ternary

    // Register using logical operators instead
    ctx.register_expression_function(
        "factorial",
        &["n"],
        "(n <= 1 && 1) || (n * factorial(n-1))",
    ).unwrap();
    
    let ctx_rc = Rc::new(ctx);
    
    // Test factorial with short-circuit evaluation
    let result = interp("factorial(5)", Some(ctx_rc.clone())).unwrap();
    assert_eq!(result, 120.0, "factorial(5) should be 120");
    
    // Try with factorial(0) to ensure base case works
    let result = interp("factorial(0)", Some(ctx_rc.clone())).unwrap();
    assert_eq!(result, 1.0, "factorial(0) should be 1");
}
```

## Risks and Mitigations

### Technical Risks

1. **Parser Complexity**
   - Risk: Modifying the parser could introduce bugs in handling expressions
   - Mitigation: Comprehensive tests with various expression patterns

2. **AST Compatibility**
   - Risk: New AST node type might cause issues with existing code
   - Mitigation: Test all parser and evaluation combinations

3. **Short-circuit Behavior**
   - Risk: Short-circuit logic might not work as expected in complex expressions
   - Mitigation: Specific tests with side effects to verify evaluation order

4. **Recursion Handling**
   - Risk: Short-circuit could create new issues with recursion management
   - Mitigation: Tests for recursive functions using logical operators

### Backward Compatibility

The changes should maintain backward compatibility since:
1. We're not changing behavior of existing operators
2. The new operators have distinct syntax that wasn't previously valid

However, any code that directly inspects AST structures will need to account for the new node type.

## Implementation Checklist

1. [ ] Update AST structures with new LogicalOp node
2. [ ] Modify parser to handle && and || specially
3. [ ] Implement evaluation logic with short-circuit behavior
4. [ ] Add comprehensive tests
5. [ ] Update documentation

## Future Enhancements

After this implementation is complete, consider:

1. Adding ternary conditional operator (? :)
2. Implementing additional logical operators (XOR, etc.)
3. Optimizing the AST for logical expressions (e.g., constant folding)

## Conclusion

This single-shot implementation plan addresses all aspects of adding short-circuit logical operators to the expression evaluator. By following this plan, we can ensure that the implementation is complete, correct, and maintains backward compatibility.