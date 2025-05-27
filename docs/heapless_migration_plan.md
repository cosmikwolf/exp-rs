# Migration Plan: hashbrown → heapless

## Overview

This document outlines the complete migration strategy from hashbrown to heapless to resolve ARM Cortex-M unaligned memory access issues while maintaining functionality and performance.

## Problem Statement

The current hashbrown implementation causes `UsageFault` on ARM Cortex-M processors due to unaligned memory access in the hash table implementation. This makes the library unusable on embedded targets that require strict memory alignment.

## Migration Strategy

### Phase 1: Dependencies and Type Definitions

#### 1.1 Update Cargo.toml
```toml
[dependencies]
# Remove hashbrown
# hashbrown = { version = "0.15.3", default-features = false, features = ["default-hasher"] }

# Add heapless
heapless = { version = "0.8", default-features = false }

# Keep existing dependencies
bitflags = "2.9.0"
libm = { version = "0.2", optional = true }
serde = { version = "1.0", features = ["derive"], default-features = false }
```

#### 1.2 Create Type Aliases (src/types.rs)
```rust
// Type aliases for migration - makes future changes easier
use heapless::{FnvIndexMap, String};

// Configuration constants - can be adjusted based on target constraints
pub const MAX_VARIABLES: usize = 64;
pub const MAX_CONSTANTS: usize = 32; 
pub const MAX_ARRAYS: usize = 16;
pub const MAX_ATTRIBUTES: usize = 16;
pub const MAX_NESTED_ARRAYS: usize = 8;
pub const MAX_AST_CACHE: usize = 128;
pub const MAX_NATIVE_FUNCTIONS: usize = 64;
pub const MAX_EXPRESSION_FUNCTIONS: usize = 32;
pub const MAX_USER_FUNCTIONS: usize = 16;
pub const MAX_ATTR_KEYS: usize = 8;

// String length limits for embedded efficiency
pub const MAX_KEY_LENGTH: usize = 32;
pub const MAX_FUNCTION_NAME_LENGTH: usize = 24;

// Primary type aliases
pub type HeaplessString = String<MAX_KEY_LENGTH>;
pub type FunctionName = String<MAX_FUNCTION_NAME_LENGTH>;
pub type VariableMap = FnvIndexMap<HeaplessString, crate::Real, MAX_VARIABLES>;
pub type ConstantMap = FnvIndexMap<HeaplessString, crate::Real, MAX_CONSTANTS>;
pub type ArrayMap = FnvIndexMap<HeaplessString, Vec<crate::Real>, MAX_ARRAYS>;
pub type AttributeMap = FnvIndexMap<HeaplessString, FnvIndexMap<HeaplessString, crate::Real, MAX_ATTR_KEYS>, MAX_ATTRIBUTES>;
pub type NestedArrayMap = FnvIndexMap<HeaplessString, FnvIndexMap<usize, Vec<crate::Real>, MAX_NESTED_ARRAYS>, MAX_NESTED_ARRAYS>;
pub type AstCacheMap = FnvIndexMap<HeaplessString, Rc<crate::types::AstExpr>, MAX_AST_CACHE>;
pub type NativeFunctionMap<'a> = FnvIndexMap<FunctionName, crate::types::NativeFunction<'a>, MAX_NATIVE_FUNCTIONS>;
pub type ExpressionFunctionMap = FnvIndexMap<FunctionName, crate::types::ExpressionFunction, MAX_EXPRESSION_FUNCTIONS>;
pub type UserFunctionMap = FnvIndexMap<FunctionName, crate::context::UserFunction, MAX_USER_FUNCTIONS>;
```

### Phase 2: Core Structure Updates

#### 2.1 Update EvalContext (src/context.rs)
```rust
pub struct EvalContext<'a> {
    /// Variables that can be modified during evaluation
    pub variables: VariableMap,
    /// Constants that cannot be modified during evaluation  
    pub constants: ConstantMap,
    /// Arrays of values that can be accessed using array[index] syntax
    pub arrays: ArrayMap,
    /// Object attributes that can be accessed using object.attribute syntax
    pub attributes: AttributeMap,
    /// Multi-dimensional arrays (not yet fully supported)
    pub nested_arrays: NestedArrayMap,
    /// Registry of functions available in this context
    pub function_registry: Rc<FunctionRegistry<'a>>,
    /// Optional parent context for variable/function inheritance
    pub parent: Option<Rc<EvalContext<'a>>>,
    /// Optional cache for parsed ASTs to speed up repeated evaluations
    pub ast_cache: Option<RefCell<AstCacheMap>>,
}
```

#### 2.2 Update FunctionRegistry (src/context.rs)
```rust
#[derive(Default, Clone)]
pub struct FunctionRegistry<'a> {
    /// Native functions implemented in Rust code
    pub native_functions: NativeFunctionMap<'a>,
    /// Functions defined using expression strings
    pub expression_functions: ExpressionFunctionMap,
    /// User-defined functions with custom behavior
    pub user_functions: UserFunctionMap,
}
```

### Phase 3: API Changes and Error Handling

#### 3.1 Insert Method Changes
The most significant API change is that `insert()` now returns `Result<Option<V>, (K, V)>` instead of `Option<V>`.

##### Before (hashbrown):
```rust
self.variables.insert(name.to_string(), value);
```

##### After (heapless):
```rust
match self.variables.insert(name.try_into()?, value) {
    Ok(_) => Ok(()),
    Err((key, value)) => Err(ExprError::CapacityExceeded("variables"))
}
```

#### 3.2 String Conversion Helper
```rust
// Helper trait for string conversion
pub trait TryIntoHeaplessString {
    fn try_into_heapless(self) -> Result<HeaplessString, ExprError>;
}

impl TryIntoHeaplessString for &str {
    fn try_into_heapless(self) -> Result<HeaplessString, ExprError> {
        HeaplessString::try_from(self)
            .map_err(|_| ExprError::StringTooLong)
    }
}

impl TryIntoHeaplessString for String {
    fn try_into_heapless(self) -> Result<HeaplessString, ExprError> {
        HeaplessString::try_from(self.as_str())
            .map_err(|_| ExprError::StringTooLong)
    }
}
```

#### 3.3 Error Type Updates (src/error.rs)
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ExprError {
    // ... existing errors ...
    
    /// Capacity exceeded for a heapless container
    CapacityExceeded(&'static str),
    
    /// String too long for heapless string buffer
    StringTooLong,
}
```

### Phase 4: Implementation Updates

#### 4.1 Variable Management
```rust
impl<'a> EvalContext<'a> {
    pub fn set_parameter(&mut self, name: &str, value: Real) -> Result<(), ExprError> {
        let key = name.try_into_heapless()?;
        match self.variables.insert(key, value) {
            Ok(_) => Ok(()),
            Err(_) => Err(ExprError::CapacityExceeded("variables"))
        }
    }
    
    pub fn get_variable(&self, name: &str) -> Option<Real> {
        if let Ok(key) = name.try_into_heapless() {
            if let Some(val) = self.variables.get(&key) {
                return Some(*val);
            }
        }
        
        // Check parent context
        if let Some(parent) = &self.parent {
            parent.get_variable(name)
        } else {
            None
        }
    }
}
```

#### 4.2 Function Registration
```rust
impl<'a> EvalContext<'a> {
    pub fn register_native_function<F>(&mut self, name: &str, arity: usize, implementation: F) -> Result<(), ExprError>
    where
        F: Fn(&[Real]) -> Real + 'static,
    {
        let key = name.try_into_heapless()?;
        let function = crate::types::NativeFunction {
            arity,
            implementation: Rc::new(implementation),
            name: key.clone(),
            description: None,
        };
        
        match Rc::make_mut(&mut self.function_registry).native_functions.insert(key, function) {
            Ok(_) => Ok(()),
            Err(_) => Err(ExprError::CapacityExceeded("native_functions"))
        }
    }
}
```

### Phase 5: Default Function Registration

#### 5.1 Math Functions Registration
```rust
pub fn register_default_math_functions(&mut self) -> Result<(), ExprError> {
    // Basic operators - use macro to reduce boilerplate
    macro_rules! register_fn {
        ($name:expr, $arity:expr, $impl:expr) => {
            self.register_native_function($name, $arity, $impl)?;
        };
    }
    
    // Basic arithmetic
    register_fn!("+", 2, |args| args[0] + args[1]);
    register_fn!("-", 2, |args| args[0] - args[1]);
    register_fn!("*", 2, |args| args[0] * args[1]);
    register_fn!("/", 2, |args| args[0] / args[1]);
    register_fn!("%", 2, |args| args[0] % args[1]);
    
    // Comparison operators
    register_fn!("<", 2, |args| if args[0] < args[1] { 1.0 } else { 0.0 });
    register_fn!(">", 2, |args| if args[0] > args[1] { 1.0 } else { 0.0 });
    register_fn!("<=", 2, |args| if args[0] <= args[1] { 1.0 } else { 0.0 });
    register_fn!(">=", 2, |args| if args[0] >= args[1] { 1.0 } else { 0.0 });
    register_fn!("==", 2, |args| if args[0] == args[1] { 1.0 } else { 0.0 });
    register_fn!("!=", 2, |args| if args[0] != args[1] { 1.0 } else { 0.0 });
    
    // Math functions (if libm feature enabled)
    #[cfg(feature = "libm")]
    {
        register_fn!("sin", 1, |args| crate::functions::sin(args[0], 0.0));
        register_fn!("cos", 1, |args| crate::functions::cos(args[0], 0.0));
        register_fn!("tan", 1, |args| crate::functions::tan(args[0], 0.0));
        register_fn!("sqrt", 1, |args| crate::functions::sqrt(args[0], 0.0));
        register_fn!("ln", 1, |args| crate::functions::ln(args[0], 0.0));
        register_fn!("log10", 1, |args| crate::functions::log10(args[0], 0.0));
        register_fn!("exp", 1, |args| crate::functions::exp(args[0], 0.0));
        register_fn!("abs", 1, |args| args[0].abs());
        register_fn!("max", 2, |args| args[0].max(args[1]));
        register_fn!("min", 2, |args| args[0].min(args[1]));
        register_fn!("floor", 1, |args| crate::functions::floor(args[0], 0.0));
        register_fn!("ceil", 1, |args| crate::functions::ceil(args[0], 0.0));
        register_fn!("round", 1, |args| args[0].round());
        // ... more functions
    }
    
    Ok(())
}
```

### Phase 6: Testing Strategy

#### 6.1 Capacity Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_variable_capacity_limit() {
        let mut ctx = EvalContext::new();
        
        // Fill up to capacity
        for i in 0..MAX_VARIABLES {
            let name = format!("var{}", i);
            assert!(ctx.set_parameter(&name, i as Real).is_ok());
        }
        
        // One more should fail
        assert!(matches!(
            ctx.set_parameter("overflow", 1.0),
            Err(ExprError::CapacityExceeded("variables"))
        ));
    }
    
    #[test]
    fn test_function_capacity_limit() {
        let mut ctx = EvalContext::new();
        
        // Try to overflow function registry
        for i in 0..MAX_NATIVE_FUNCTIONS + 1 {
            let name = format!("fn{}", i);
            let result = ctx.register_native_function(&name, 1, |args| args[0]);
            
            if i < MAX_NATIVE_FUNCTIONS {
                assert!(result.is_ok(), "Should succeed for function {}", i);
            } else {
                assert!(matches!(result, Err(ExprError::CapacityExceeded("native_functions"))));
            }
        }
    }
    
    #[test]  
    fn test_string_length_limit() {
        let mut ctx = EvalContext::new();
        
        // String within limit should work
        let short_name = "x".repeat(MAX_KEY_LENGTH);
        assert!(ctx.set_parameter(&short_name, 1.0).is_ok());
        
        // String exceeding limit should fail
        let long_name = "x".repeat(MAX_KEY_LENGTH + 1);
        assert!(matches!(
            ctx.set_parameter(&long_name, 1.0),
            Err(ExprError::StringTooLong)
        ));
    }
}
```

#### 6.2 Compatibility Testing
```rust
#[test]
fn test_existing_functionality() {
    let mut ctx = EvalContext::new();
    
    // Test basic variable operations
    assert!(ctx.set_parameter("x", 5.0).is_ok());
    assert_eq!(ctx.get_variable("x"), Some(5.0));
    
    // Test expression evaluation
    let result = crate::engine::interp("x + 1", Some(Rc::new(ctx))).unwrap();
    assert_eq!(result, 6.0);
}
```

### Phase 7: Migration Steps

1. **Backup current code** - Create branch before migration
2. **Update dependencies** - Add heapless, remove hashbrown
3. **Add type aliases** - Create new type definitions
4. **Update core structures** - Modify EvalContext and FunctionRegistry
5. **Update API calls** - Handle new insert() return types
6. **Add error handling** - Handle capacity and string length errors
7. **Update tests** - Add capacity limit tests
8. **Performance testing** - Verify acceptable performance
9. **Documentation** - Update docs with new limitations

### Phase 8: Configuration Options

#### 8.1 Feature Flags
```toml
[features]
default = ["libm"]
f32 = []
libm = ["dep:libm"]
large_capacity = []  # Doubles all capacity limits
```

#### 8.2 Conditional Compilation
```rust
#[cfg(feature = "large_capacity")]
pub const MAX_VARIABLES: usize = 128;
#[cfg(not(feature = "large_capacity"))]
pub const MAX_VARIABLES: usize = 64;
```

### Phase 9: Rollback Plan

If migration causes issues:

1. **Revert Cargo.toml** - Switch back to hashbrown
2. **Conditional compilation** - Support both backends
3. **Feature flag** - `heapless` feature for embedded targets

```rust
#[cfg(feature = "heapless")]
use heapless::FnvIndexMap as HashMap;
#[cfg(not(feature = "heapless"))] 
use hashbrown::HashMap;
```

## Performance Considerations

- **Memory**: Fixed allocation vs dynamic
- **Speed**: Linear probing (heapless) vs Robin Hood hashing (hashbrown)
- **Capacity**: Fixed limits require careful sizing
- **String operations**: Additional copying for HeaplessString

## Embedded Target Benefits

- **No unaligned access** - Works on ARM Cortex-M
- **Predictable memory** - No heap fragmentation
- **Deterministic performance** - No dynamic allocation
- **Stack allocation** - Better for real-time systems

## Breaking Changes

1. **API methods return Result** instead of direct success
2. **String length limits** enforced
3. **Capacity limits** enforced
4. **Some iterator types change**

## Timeline

- **Week 1**: Dependencies and type aliases
- **Week 2**: Core structure updates and basic API
- **Week 3**: Function registration and error handling  
- **Week 4**: Testing and documentation
- **Week 5**: Performance optimization and final testing