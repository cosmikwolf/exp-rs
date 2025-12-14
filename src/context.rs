extern crate alloc;

#[cfg(test)]
use crate::Real;
#[cfg(not(test))]
use crate::{Real, String, ToString, Vec};
#[cfg(not(test))]
use alloc::rc::Rc;
// Import heapless types and helper traits
use crate::types::{TryIntoFunctionName, TryIntoHeaplessString};
#[cfg(test)]
use std::rc::Rc;
#[cfg(test)]
use std::string::{String, ToString};
#[cfg(test)]
use std::vec::Vec;

/// Registry for different types of functions available in an evaluation context.
///
/// This struct holds three types of functions:
/// 1. Native functions: Rust functions that can be called from expressions
/// 2. Expression functions: Functions defined using expression strings
/// 3. User functions: Functions defined by the user with custom behavior
///
/// This is an internal implementation detail and users typically don't interact with it directly.
#[allow(dead_code)]
#[derive(Default, Clone)]
pub struct FunctionRegistry {
    /// Native functions implemented in Rust code
    pub native_functions: crate::types::NativeFunctionMap,
    /// Functions defined using expression strings
    pub expression_functions: crate::types::ExpressionFunctionMap,
}

/// Evaluation context for expressions.
///
/// This is the main configuration object that holds variables, constants, arrays, functions,
/// and other settings for evaluating expressions. You typically create an `EvalContext`,
/// add your variables and functions, and then pass it to the `interp` function.
///
/// # Examples
///
/// ```
/// use exp_rs::context::EvalContext;
/// use exp_rs::engine::interp;
/// use exp_rs::Real;
/// use std::rc::Rc;
///
/// let mut ctx = EvalContext::new();
///
/// // Add variables
/// ctx.set_parameter("x", 5.0 as Real);
/// ctx.set_parameter("y", 10.0 as Real);
///
/// // Add a constant
/// ctx.constants.insert("PI_SQUARED".try_into().unwrap(), 9.8696 as Real).unwrap();
///
/// // Register a custom function
/// ctx.register_native_function("multiply", 2, |args| args[0] * args[1]);
///
/// // Evaluate expressions using this context
/// let result = interp("x + y * PI_SQUARED", Some(Rc::new(ctx.clone()))).unwrap();
/// let result2 = interp("multiply(x, y)", Some(Rc::new(ctx))).unwrap();
/// ```
///
/// Contexts can be nested to create scopes:
///
/// ```
/// use exp_rs::context::EvalContext;
/// use exp_rs::Real;
/// use std::rc::Rc;
///
/// let mut parent = EvalContext::new();
/// parent.set_parameter("x", 1.0 as Real);
///
/// let mut child = EvalContext::new();
/// child.set_parameter("y", 2.0 as Real);
/// child.parent = Some(Rc::new(parent));
///
/// // The child context can access both its own variables and the parent's
/// ```
pub struct EvalContext {
    /// Variables that can be modified during evaluation
    pub variables: crate::types::VariableMap,
    /// Constants that cannot be modified during evaluation
    pub constants: crate::types::ConstantMap,
    /// Arrays of values that can be accessed using array[index] syntax
    pub arrays: crate::types::ArrayMap,
    /// Object attributes that can be accessed using object.attribute syntax
    pub attributes: crate::types::AttributeMap,
    /// Multi-dimensional arrays (not yet fully supported)
    pub nested_arrays: crate::types::NestedArrayMap,
    /// Registry of functions available in this context
    pub native_functions: Rc<crate::types::NativeFunctionMap>,
    /// Optional parent context for variable/function inheritance
    pub parent: Option<Rc<EvalContext>>,
}

impl EvalContext {
    /// Creates a new empty evaluation context.
    ///
    /// The context starts with no variables, constants, arrays, or functions.
    /// You can add these elements using the appropriate methods and fields.
    ///
    /// # Examples
    ///
    /// ```
    /// use exp_rs::context::EvalContext;
    ///
    /// let ctx = EvalContext::new();
    /// // Now add variables, constants, functions, etc.
    /// ```
    pub fn new() -> Self {
        let mut ctx = Self {
            variables: crate::types::VariableMap::new(),
            constants: crate::types::ConstantMap::new(),
            arrays: crate::types::ArrayMap::new(),
            attributes: crate::types::AttributeMap::new(),
            nested_arrays: crate::types::NestedArrayMap::new(),
            native_functions: Rc::new(crate::types::NativeFunctionMap::new()),
            parent: None,
        };

        // Always register default math functions
        // The actual functions registered depend on feature flags
        ctx.register_default_math_functions();

        ctx
    }

    /// Creates a new context with default math functions registered.
    ///
    /// This is a convenience method for creating a context with all standard
    /// math functions already registered. It's equivalent to calling `new()`
    /// since default functions are now always registered.
    ///
    /// Kept for backward compatibility.
    pub fn with_default_functions() -> Self {
        // Simply call new() as it now always registers default functions
        Self::new()
    }

    /// Creates an evaluation context without any pre-registered functions.
    ///
    /// This creates a context with no built-in functions or constants.
    /// Note that basic operators (+, -, *, /, %, <, >, <=, >=, ==, !=) are still
    /// available as they are handled by the parser, not the function registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use exp_rs::context::EvalContext;
    ///
    /// let mut ctx = EvalContext::empty();
    /// // Basic operators still work
    /// // But functions like sin, cos, abs, etc. must be registered manually
    /// ctx.register_native_function("abs", 1, |args| args[0].abs());
    /// ctx.register_native_function("sin", 1, |args| args[0].sin());
    /// ```
    pub fn empty() -> Self {
        Self {
            variables: crate::types::VariableMap::new(),
            constants: crate::types::ConstantMap::new(),
            arrays: crate::types::ArrayMap::new(),
            attributes: crate::types::AttributeMap::new(),
            nested_arrays: crate::types::NestedArrayMap::new(),
            native_functions: Rc::new(crate::types::NativeFunctionMap::new()),
            parent: None,
        }
    }

    /// Sets a parameter (variable) in the context.
    ///
    /// This method adds or updates a variable in the context. Variables can be used
    /// in expressions and their values can be changed between evaluations.
    ///
    /// # Parameters
    ///
    /// * `name`: The name of the variable
    /// * `value`: The value to assign to the variable
    ///
    /// # Returns
    ///
    /// The previous value of the variable, if it existed
    ///
    /// # Examples
    ///
    /// ```
    /// use exp_rs::context::EvalContext;
    /// use exp_rs::engine::interp;
    /// use exp_rs::Real;
    /// use std::rc::Rc;
    ///
    /// let mut ctx = EvalContext::new();
    /// ctx.set_parameter("x", 42.0 as Real);
    ///
    /// let result = interp("x * 2", Some(Rc::new(ctx))).unwrap();
    /// assert_eq!(result, 84.0);
    /// ```
    pub fn set_parameter(
        &mut self,
        name: &str,
        value: Real,
    ) -> Result<Option<Real>, crate::error::ExprError> {
        let key = name.try_into_heapless()?;
        match self.variables.insert(key, value) {
            Ok(old_value) => Ok(old_value),
            Err(_) => Err(crate::error::ExprError::CapacityExceeded("variables")),
        }
    }

    /// Registers a native function in the context.
    ///
    /// Native functions are implemented in Rust and can be called from expressions.
    /// They take a slice of Real values as arguments and return a Real value.
    ///
    /// # Parameters
    ///
    /// * `name`: The name of the function as it will be used in expressions
    /// * `arity`: The number of arguments the function expects
    /// * `implementation`: A closure or function that implements the function logic
    ///
    /// # Examples
    ///
    /// ```
    /// use exp_rs::context::EvalContext;
    /// use exp_rs::engine::interp;
    /// use exp_rs::Real;
    /// use std::rc::Rc;
    ///
    /// let mut ctx = EvalContext::new();
    ///
    /// // Register a function that adds all its arguments
    /// ctx.register_native_function("sum", 3, |args| {
    ///     args.iter().sum::<Real>()
    /// });
    ///
    /// let result = interp("sum(10, 20, 30)", Some(Rc::new(ctx))).unwrap();
    /// assert_eq!(result, 60.0);
    /// ```
    ///
    /// Functions with variable argument counts:
    ///
    /// ```
    /// use exp_rs::context::EvalContext;
    /// use exp_rs::engine::interp;
    /// use exp_rs::Real;
    /// use std::rc::Rc;
    ///
    /// let mut ctx = EvalContext::new();
    ///
    /// // Register a function that calculates the mean of its arguments
    /// ctx.register_native_function("mean", 5, |args| {
    ///     args.iter().sum::<Real>() / args.len() as Real
    /// });
    ///
    /// let result = interp("mean(1, 2, 3, 4, 5)", Some(Rc::new(ctx))).unwrap();
    /// assert_eq!(result, 3.0);
    /// ```
    pub fn register_native_function<F>(
        &mut self,
        name: &str,
        arity: usize,
        implementation: F,
    ) -> Result<(), crate::error::ExprError>
    where
        F: Fn(&[Real]) -> Real + 'static,
    {
        let key = name.try_into_function_name()?;
        let function = crate::types::NativeFunction {
            arity,
            implementation: Rc::new(implementation),
            name: key.clone(),
            description: None,
        };

        match Rc::make_mut(&mut self.native_functions).insert(key, function) {
            Ok(_) => Ok(()),
            Err(_) => Err(crate::error::ExprError::CapacityExceeded(
                "native_functions",
            )),
        }
    }

    /// Enables AST caching for this context to improve performance.
    ///
    /// When enabled, repeated calls to `interp` with the same expression string
    /// will reuse the parsed AST, greatly improving performance for repeated evaluations
    /// with different variable values.
    ///
    /// This is particularly useful in loops or when evaluating the same expression
    /// multiple times with different parameter values.
    ///
    /// # Note
    ///
    /// AST caching has been removed in the arena-based implementation.
    /// The arena architecture provides better performance characteristics
    /// without the need for explicit caching.
    ///
    /// Disables AST caching and clears the cache.
    ///
    /// This is useful if you want to free up memory or if you want to force
    /// re-parsing of expressions.
    ///
    /// # Note
    ///
    /// AST caching has been removed in the arena-based implementation.
    /// This functionality is no longer available.
    ///
    /// Clear the AST cache if enabled.
    ///
    /// Registers all built-in math functions as native functions in the context.
    ///
    /// # Usage
    ///
    /// ```
    /// # use exp_rs::EvalContext;
    /// let mut ctx = EvalContext::new();
    /// ctx.register_default_math_functions();
    /// ```
    ///
    /// After calling this, you can override any built-in by registering your own native function
    /// with the same name using [`register_native_function`](Self::register_native_function).
    ///
    /// # Feature: `libm`
    ///
    /// If the `libm` feature is enabled, this will use the libm implementations.
    /// Otherwise, it will use the standard library implementation which is not available
    /// in `no_std` environments.
    ///
    /// Enables default math functions for this context.
    ///
    /// Alias for `register_default_math_functions()`.
    pub fn enable_default_functions(&mut self) {
        self.register_default_math_functions();
    }

    /// Registers all built-in math functions as native functions in the context.
    pub fn register_default_math_functions(&mut self) {
        // Basic operators as functions (always available)
        let _ = self.register_native_function("+", 2, |args| args[0] + args[1]);
        let _ = self.register_native_function("-", 2, |args| args[0] - args[1]);
        let _ = self.register_native_function("*", 2, |args| args[0] * args[1]);
        let _ = self.register_native_function("/", 2, |args| args[0] / args[1]);
        let _ = self.register_native_function("%", 2, |args| args[0] % args[1]);

        // Comparison operators (always available)
        let _ =
            self.register_native_function("<", 2, |args| if args[0] < args[1] { 1.0 } else { 0.0 });
        let _ =
            self.register_native_function(">", 2, |args| if args[0] > args[1] { 1.0 } else { 0.0 });
        let _ = self.register_native_function(
            "<=",
            2,
            |args| if args[0] <= args[1] { 1.0 } else { 0.0 },
        );
        let _ = self.register_native_function(
            ">=",
            2,
            |args| if args[0] >= args[1] { 1.0 } else { 0.0 },
        );
        let _ = self.register_native_function(
            "==",
            2,
            |args| if args[0] == args[1] { 1.0 } else { 0.0 },
        );
        let _ = self.register_native_function(
            "!=",
            2,
            |args| if args[0] != args[1] { 1.0 } else { 0.0 },
        );

        // Logical operators (always available)
        let _ = self.register_native_function("&&", 2, |args| {
            if args[0] != 0.0 && args[1] != 0.0 {
                1.0
            } else {
                0.0
            }
        });
        let _ = self.register_native_function("||", 2, |args| {
            if args[0] != 0.0 || args[1] != 0.0 {
                1.0
            } else {
                0.0
            }
        });

        // Function aliases for the operators (always available)
        let _ = self.register_native_function("add", 2, |args| args[0] + args[1]);
        let _ = self.register_native_function("sub", 2, |args| args[0] - args[1]);
        let _ = self.register_native_function("mul", 2, |args| args[0] * args[1]);
        let _ = self.register_native_function("div", 2, |args| args[0] / args[1]);
        let _ = self.register_native_function("fmod", 2, |args| args[0] % args[1]);
        let _ = self.register_native_function("neg", 1, |args| -args[0]);

        // Sequence operators (always available)
        let _ = self.register_native_function(",", 2, |args| args[1]); // The actual comma operator
        let _ = self.register_native_function("comma", 2, |args| args[1]); // Function alias for the comma operator

        // Core math functions that don't require libm (always available)
        let _ = self.register_native_function("abs", 1, |args| args[0].abs());
        let _ = self.register_native_function("max", 2, |args| args[0].max(args[1]));
        let _ = self.register_native_function("min", 2, |args| args[0].min(args[1]));
        let _ = self.register_native_function("sign", 1, |args| {
            if args[0] > 0.0 {
                1.0
            } else if args[0] < 0.0 {
                -1.0
            } else {
                0.0
            }
        });

        // Math constants (always available)
        #[cfg(feature = "f32")]
        let _ = self.register_native_function("e", 0, |_| core::f32::consts::E);
        #[cfg(not(feature = "f32"))]
        let _ = self.register_native_function("e", 0, |_| core::f64::consts::E);

        #[cfg(feature = "f32")]
        let _ = self.register_native_function("pi", 0, |_| core::f32::consts::PI);
        #[cfg(not(feature = "f32"))]
        let _ = self.register_native_function("pi", 0, |_| core::f64::consts::PI);

        // Advanced math functions with libm
        #[cfg(feature = "libm")]
        {
            let _ = self
                .register_native_function("acos", 1, |args| crate::functions::acos(args[0], 0.0));
            let _ = self
                .register_native_function("asin", 1, |args| crate::functions::asin(args[0], 0.0));
            let _ = self
                .register_native_function("atan", 1, |args| crate::functions::atan(args[0], 0.0));
            let _ = self.register_native_function("atan2", 2, |args| {
                crate::functions::atan2(args[0], args[1])
            });
            let _ = self
                .register_native_function("ceil", 1, |args| crate::functions::ceil(args[0], 0.0));
            let _ =
                self.register_native_function("cos", 1, |args| crate::functions::cos(args[0], 0.0));
            let _ = self
                .register_native_function("cosh", 1, |args| crate::functions::cosh(args[0], 0.0));
            let _ =
                self.register_native_function("exp", 1, |args| crate::functions::exp(args[0], 0.0));
            let _ = self
                .register_native_function("floor", 1, |args| crate::functions::floor(args[0], 0.0));
            let _ = self
                .register_native_function("round", 1, |args| crate::functions::round(args[0], 0.0));
            let _ =
                self.register_native_function("ln", 1, |args| crate::functions::ln(args[0], 0.0));
            let _ =
                self.register_native_function("log", 1, |args| crate::functions::log(args[0], 0.0));
            let _ = self
                .register_native_function("log10", 1, |args| crate::functions::log10(args[0], 0.0));
            let _ = self
                .register_native_function("pow", 2, |args| crate::functions::pow(args[0], args[1]));
            let _ = self
                .register_native_function("^", 2, |args| crate::functions::pow(args[0], args[1]));
            let _ =
                self.register_native_function("sin", 1, |args| crate::functions::sin(args[0], 0.0));
            let _ = self
                .register_native_function("sinh", 1, |args| crate::functions::sinh(args[0], 0.0));
            let _ = self
                .register_native_function("sqrt", 1, |args| crate::functions::sqrt(args[0], 0.0));
            let _ =
                self.register_native_function("tan", 1, |args| crate::functions::tan(args[0], 0.0));
            let _ = self
                .register_native_function("tanh", 1, |args| crate::functions::tanh(args[0], 0.0));
        }

        // In test mode without libm, provide std library implementations
        #[cfg(all(not(feature = "libm"), test))]
        {
            let _ = self.register_native_function("acos", 1, |args| args[0].acos());
            let _ = self.register_native_function("asin", 1, |args| args[0].asin());
            let _ = self.register_native_function("atan", 1, |args| args[0].atan());
            let _ = self.register_native_function("atan2", 2, |args| args[0].atan2(args[1]));
            let _ = self.register_native_function("ceil", 1, |args| args[0].ceil());
            let _ = self.register_native_function("cos", 1, |args| args[0].cos());
            let _ = self.register_native_function("cosh", 1, |args| args[0].cosh());
            let _ = self.register_native_function("exp", 1, |args| args[0].exp());
            let _ = self.register_native_function("floor", 1, |args| args[0].floor());
            let _ = self.register_native_function("round", 1, |args| args[0].round());
            let _ = self.register_native_function("ln", 1, |args| args[0].ln());
            let _ = self.register_native_function("log", 1, |args| args[0].log10());
            let _ = self.register_native_function("log10", 1, |args| args[0].log10());
            let _ = self.register_native_function("pow", 2, |args| args[0].powf(args[1]));
            let _ = self.register_native_function("^", 2, |args| args[0].powf(args[1]));
            let _ = self.register_native_function("sin", 1, |args| args[0].sin());
            let _ = self.register_native_function("sinh", 1, |args| args[0].sinh());
            let _ = self.register_native_function("sqrt", 1, |args| args[0].sqrt());
            let _ = self.register_native_function("tan", 1, |args| args[0].tan());
            let _ = self.register_native_function("tanh", 1, |args| args[0].tanh());
        }

        // In non-test no_std mode without libm, we don't register advanced math functions
        // Users must register their own implementations if needed
    }

    /// Register a native function with the context.
    ///
    /// # Overriding Built-ins
    ///
    /// If a function with the same name as a built-in is registered, the user-defined function
    /// will take precedence over the built-in. This allows users to override any built-in math
    /// function at runtime.
    ///
    /// # Disabling Built-ins
    ///
    /// If the `libm` feature is not enabled, built-in math functions are not available,
    /// and users must register their own implementations for all required functions.
    ///
    /// # Example
    ///
    /// ```
    /// # use exp_rs::EvalContext;
    /// let mut ctx = EvalContext::new();
    /// // Override the "sin" function
    /// ctx.register_native_function("sin", 1, |args| 42.0);
    /// ```

    pub fn get_variable(&self, name: &str) -> Option<Real> {
        if let Ok(key) = name.try_into_heapless() {
            if let Some(val) = self.variables.get(&key) {
                return Some(*val);
            }
        }

        if let Some(parent) = &self.parent {
            parent.get_variable(name)
        } else {
            None
        }
    }

    pub fn get_constant(&self, name: &str) -> Option<Real> {
        if let Ok(key) = name.try_into_heapless() {
            if let Some(val) = self.constants.get(&key) {
                return Some(*val);
            }
        }

        if let Some(parent) = &self.parent {
            parent.get_constant(name)
        } else {
            None
        }
    }

    pub fn get_array(&self, name: &str) -> Option<&alloc::vec::Vec<crate::Real>> {
        if let Ok(key) = name.try_into_heapless() {
            if let Some(arr) = self.arrays.get(&key) {
                return Some(arr);
            }
        }

        if let Some(parent) = &self.parent {
            parent.get_array(name)
        } else {
            None
        }
    }

    /// Helper method to set an attribute value on an object
    pub fn set_attribute(
        &mut self,
        object_name: &str,
        attr_name: &str,
        value: Real,
    ) -> Result<Option<Real>, crate::error::ExprError> {
        let obj_key = object_name.try_into_heapless()?;
        let attr_key = attr_name.try_into_heapless()?;

        // Get or create the object's attribute map
        if !self.attributes.contains_key(&obj_key) {
            let attr_map = heapless::FnvIndexMap::<
                crate::types::HString,
                Real,
                { crate::types::EXP_RS_MAX_ATTR_KEYS },
            >::new();
            self.attributes
                .insert(obj_key.clone(), attr_map)
                .map_err(|_| crate::error::ExprError::CapacityExceeded("attributes"))?;
        }

        // Get mutable reference to the attribute map and insert the value
        if let Some(attr_map) = self.attributes.get_mut(&obj_key) {
            attr_map
                .insert(attr_key, value)
                .map_err(|_| crate::error::ExprError::CapacityExceeded("object attributes"))
        } else {
            unreachable!("Just inserted the object")
        }
    }

    pub fn get_attribute_map(
        &self,
        base: &str,
    ) -> Option<
        &heapless::FnvIndexMap<crate::types::HString, Real, { crate::types::EXP_RS_MAX_ATTR_KEYS }>,
    > {
        if let Ok(key) = base.try_into_heapless() {
            if let Some(attr_map) = self.attributes.get(&key) {
                return Some(attr_map);
            }
        }

        if let Some(parent) = &self.parent {
            parent.get_attribute_map(base)
        } else {
            None
        }
    }

    pub fn get_native_function(&self, name: &str) -> Option<&crate::types::NativeFunction> {
        if let Ok(key) = name.try_into_function_name() {
            if let Some(f) = self.native_functions.get(&key) {
                return Some(f);
            }
        }

        if let Some(parent) = &self.parent {
            parent.get_native_function(name)
        } else {
            None
        }
    }

    /// Get a list of all native function names in this context (including parent contexts)
    pub fn list_native_functions(&self) -> Vec<String> {
        let mut functions = Vec::new();
        let mut seen = alloc::collections::BTreeSet::new();

        // Collect from current context
        for (name, _) in self.native_functions.iter() {
            let name_str = name.to_string();
            if seen.insert(name_str.clone()) {
                functions.push(name_str);
            }
        }

        // Collect from parent contexts
        if let Some(parent) = &self.parent {
            for name in parent.list_native_functions() {
                if seen.insert(name.clone()) {
                    functions.push(name);
                }
            }
        }

        functions.sort();
        functions
    }

    /// Get a list of all expression function names in this context (including parent contexts)
    pub fn list_expression_functions(&self) -> Vec<String> {
        let mut functions = Vec::new();
        let mut seen = alloc::collections::BTreeSet::new();

        // Collect from current context
        // Expression functions no longer exist in context

        // Collect from parent contexts
        if let Some(parent) = &self.parent {
            for name in parent.list_expression_functions() {
                if seen.insert(name.clone()) {
                    functions.push(name);
                }
            }
        }

        functions.sort();
        functions
    }
}

impl Clone for EvalContext {
    fn clone(&self) -> Self {
        Self {
            variables: self.variables.clone(),
            constants: self.constants.clone(),
            arrays: self.arrays.clone(),
            attributes: self.attributes.clone(),
            nested_arrays: self.nested_arrays.clone(),
            native_functions: self.native_functions.clone(),
            parent: self.parent.clone(),
        }
    }
}

impl Default for EvalContext {
    /// Creates a new EvalContext with default values and math functions registered.
    /// This ensures that EvalContext::default() behaves the same as
    fn default() -> Self {
        EvalContext::new()
        // let mut ctx = Self {
        //     variables: HashMap::new(),
        //     constants: HashMap::new(),
        //     arrays: HashMap::new(),
        //     attributes: HashMap::new(),
        //     nested_arrays: HashMap::new(),
        //     function_registry: Rc::new(FunctionRegistry::default()),
        //     parent: None,
        //     ast_cache: None,
        // };
        //
        // // Register default math functions, same as in new()
        // ctx.register_default_math_functions();
        //
        // ctx
    }
}

// Helper trait removed - heapless containers support Clone directly

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine;
    use crate::types::AstExpr;
    use crate::types::TryIntoHeaplessString;
    use std::rc::Rc;

    #[test]
    fn test_get_variable_parent_chain() {
        // Create parent context with some variables
        let mut parent_ctx = EvalContext::new();
        let _ = parent_ctx.set_parameter("parent_only", 1.0);
        let _ = parent_ctx.set_parameter("shadowed", 2.0);

        // Create child context with its own variables
        let mut child_ctx = EvalContext::new();
        let _ = child_ctx.set_parameter("child_only", 3.0);
        let _ = child_ctx.set_parameter("shadowed", 4.0); // Shadows parent's value
        child_ctx.parent = Some(Rc::new(parent_ctx));

        // Test variable only in parent
        assert_eq!(child_ctx.get_variable("parent_only"), Some(1.0));

        // Test variable only in child
        assert_eq!(child_ctx.get_variable("child_only"), Some(3.0));

        // Test shadowed variable (child's value should be returned)
        assert_eq!(child_ctx.get_variable("shadowed"), Some(4.0));

        // Test non-existent variable
        assert_eq!(child_ctx.get_variable("nonexistent"), None);
    }

    #[test]
    fn test_get_variable_deep_chain() {
        // Create grandparent context
        let mut grandparent_ctx = EvalContext::new();
        let _ = grandparent_ctx.set_parameter("grandparent_var", 1.0);
        let _ = grandparent_ctx.set_parameter("shadowed", 2.0);

        // Create parent context
        let mut parent_ctx = EvalContext::new();
        let _ = parent_ctx.set_parameter("parent_var", 3.0);
        let _ = parent_ctx.set_parameter("shadowed", 4.0);
        parent_ctx.parent = Some(Rc::new(grandparent_ctx));

        // Create child context
        let mut child_ctx = EvalContext::new();
        let _ = child_ctx.set_parameter("child_var", 5.0);
        let _ = child_ctx.set_parameter("shadowed", 6.0);
        child_ctx.parent = Some(Rc::new(parent_ctx));

        // Test lookup at each level
        assert_eq!(child_ctx.get_variable("child_var"), Some(5.0));
        assert_eq!(child_ctx.get_variable("parent_var"), Some(3.0));
        assert_eq!(child_ctx.get_variable("grandparent_var"), Some(1.0));

        // Test shadowing - should get closest value
        assert_eq!(child_ctx.get_variable("shadowed"), Some(6.0));
    }

    #[test]
    fn test_get_variable_null_parent() {
        let mut ctx = EvalContext::new();
        let _ = ctx.set_parameter("x", 1.0);
        ctx.parent = None;

        assert_eq!(ctx.get_variable("x"), Some(1.0));
        assert_eq!(ctx.get_variable("nonexistent"), None);
    }

    #[test]
    fn test_get_variable_cyclic_reference_safety() {
        // Create two contexts that will reference each other
        let mut ctx1 = EvalContext::new();
        let mut ctx2 = EvalContext::new();

        let _ = ctx1.set_parameter("var1", 1.0);
        let _ = ctx2.set_parameter("var2", 2.0);

        // Create a cyclic reference (this would be unsafe in practice)
        // We'll test that variable lookup still works without infinite recursion
        let ctx1_rc = Rc::new(ctx1);
        ctx2.parent = Some(Rc::clone(&ctx1_rc));

        // Test lookup still works in potential cycle
        assert_eq!(ctx2.get_variable("var2"), Some(2.0));
        assert_eq!(ctx2.get_variable("var1"), Some(1.0));
    }

    #[test]
    fn test_get_variable_in_function_scope() {
        let mut ctx = EvalContext::new();

        // Set up parent context with a variable
        let _ = ctx.set_parameter("x", 100.0);

        // Create a function context that uses 'x' as parameter
        let mut func_ctx = EvalContext::new();
        let _ = func_ctx.set_parameter("x", 5.0); // Parameter value
        func_ctx.parent = Some(Rc::new(ctx.clone()));

        // Test variable lookup in function scope
        assert_eq!(
            func_ctx.get_variable("x"),
            Some(5.0),
            "Function parameter should shadow parent variable"
        );

        // Print debug info
        println!("Parent context x = {:?}", ctx.get_variable("x"));
        println!("Function context x = {:?}", func_ctx.get_variable("x"));
        println!("Function context variables: {:?}", func_ctx.variables);
        println!(
            "Function context parent variables: {:?}",
            func_ctx.parent.as_ref().map(|p| &p.variables)
        );
    }

    #[test]
    fn test_get_variable_nested_scopes() {
        let mut root_ctx = EvalContext::new();
        let _ = root_ctx.set_parameter("x", 1.0);
        let _ = root_ctx.set_parameter("y", 1.0);

        let mut mid_ctx = EvalContext::new();
        let _ = mid_ctx.set_parameter("x", 2.0);
        mid_ctx.parent = Some(Rc::new(root_ctx));

        let mut leaf_ctx = EvalContext::new();
        let _ = leaf_ctx.set_parameter("x", 3.0);
        leaf_ctx.parent = Some(Rc::new(mid_ctx));

        // Test variable lookup at each level
        assert_eq!(
            leaf_ctx.get_variable("x"),
            Some(3.0),
            "Should get leaf context value"
        );
        assert_eq!(
            leaf_ctx.get_variable("y"),
            Some(1.0),
            "Should get root context value when not shadowed"
        );

        println!("Variable lookup in nested scopes:");
        println!("leaf x = {:?}", leaf_ctx.get_variable("x"));
        println!("leaf y = {:?}", leaf_ctx.get_variable("y"));
        println!("leaf variables: {:?}", leaf_ctx.variables);
        println!(
            "mid variables: {:?}",
            leaf_ctx.parent.as_ref().map(|p| &p.variables)
        );
        println!(
            "root variables: {:?}",
            leaf_ctx
                .parent
                .as_ref()
                .and_then(|p| p.parent.as_ref())
                .map(|p| &p.variables)
        );
    }

    #[test]
    fn test_get_variable_function_parameter_precedence() {
        let mut ctx = EvalContext::new();
        let arena = bumpalo::Bump::new();
        let mut batch = crate::expression::Expression::new(&arena);

        // Register a function that uses parameter 'x' in arena batch
        batch
            .register_expression_function("f", &["x"], "x * 2")
            .unwrap();

        // Set a global 'x'
        let _ = ctx.set_parameter("x", 100.0);

        // Create evaluation context for function
        let mut func_ctx = EvalContext::new();
        let _ = func_ctx.set_parameter("x", 5.0); // Parameter value
        func_ctx.parent = Some(Rc::new(ctx));

        println!("Function parameter context:");
        println!("func_ctx x = {:?}", func_ctx.get_variable("x"));
        println!("func_ctx variables: {:?}", func_ctx.variables);
        println!(
            "parent variables: {:?}",
            func_ctx.parent.as_ref().map(|p| &p.variables)
        );

        assert_eq!(
            func_ctx.get_variable("x"),
            Some(5.0),
            "Function parameter should take precedence over global x"
        );
    }

    #[test]
    fn test_get_variable_temporary_scope() {
        let mut ctx = EvalContext::new();
        let _ = ctx.set_parameter("x", 1.0);

        // Create temporary scope
        let mut temp_ctx = EvalContext::new();
        temp_ctx.parent = Some(Rc::new(ctx));

        // Variable lookup should find parent value
        assert_eq!(
            temp_ctx.get_variable("x"),
            Some(1.0),
            "Should find variable in parent scope"
        );

        // Add variable to temporary scope
        let _ = temp_ctx.set_parameter("x", 2.0);

        // Should now find local value
        assert_eq!(
            temp_ctx.get_variable("x"),
            Some(2.0),
            "Should find shadowed variable in local scope"
        );

        println!("Temporary scope variable lookup:");
        println!("temp x = {:?}", temp_ctx.get_variable("x"));
        println!("temp variables: {:?}", temp_ctx.variables);
        println!(
            "parent variables: {:?}",
            temp_ctx.parent.as_ref().map(|p| &p.variables)
        );
    }

    #[test]
    fn test_native_function() {
        let mut ctx = EvalContext::new();

        ctx.register_native_function("add_all", 3, |args| args.iter().sum())
            .unwrap();

        let val = engine::interp("add_all(1, 2, 3)", Some(Rc::new(ctx))).unwrap();
        assert_eq!(val, 6.0);
    }

    #[test]
    fn test_array_access() {
        let mut ctx = EvalContext::new();
        ctx.arrays
            .insert(
                "climb_wave_wait_time".try_into_heapless().unwrap(),
                vec![10.0, 20.0, 30.0],
            )
            .expect("Failed to insert array");
        let val = engine::interp("climb_wave_wait_time[1]", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val, 20.0);
    }

    #[test]
    fn test_array_access_ast_structure() {
        let mut ctx = EvalContext::new();
        ctx.arrays
            .insert(
                "climb_wave_wait_time".try_into_heapless().unwrap(),
                vec![10.0, 20.0, 30.0],
            )
            .expect("Failed to insert array");
        use bumpalo::Bump;
        let arena = Bump::new();
        let ast = engine::parse_expression("climb_wave_wait_time[1]", &arena).unwrap();
        match ast {
            AstExpr::Array { name, index } => {
                assert_eq!(name, "climb_wave_wait_time");
                match *index {
                    AstExpr::Constant(val) => assert_eq!(val, 1.0),
                    _ => panic!("Expected constant index"),
                }
            }
            _ => panic!("Expected array AST node"),
        }
    }

    #[test]
    fn test_attribute_access() {
        let mut ctx = EvalContext::new();
        let mut foo_map = heapless::FnvIndexMap::<
            crate::types::HString,
            crate::Real,
            { crate::types::EXP_RS_MAX_ATTR_KEYS },
        >::new();
        foo_map
            .insert("bar".try_into_heapless().unwrap(), 42.0)
            .unwrap();
        ctx.attributes
            .insert("foo".try_into_heapless().unwrap(), foo_map)
            .unwrap();

        use bumpalo::Bump;
        let arena = Bump::new();
        let ast = engine::parse_expression("foo.bar", &arena).unwrap();
        println!("AST for foo.bar: {:?}", ast);

        let ctx_copy = ctx.clone();
        let eval_result = crate::eval::eval_ast(&ast, Some(Rc::new(ctx_copy)), &arena);
        println!("Direct eval_ast result: {:?}", eval_result);

        let ctx_copy2 = ctx.clone();
        let val = engine::interp("foo.bar", Some(Rc::new(ctx_copy2))).unwrap();
        assert_eq!(val, 42.0);

        let ctx_copy3 = ctx.clone();
        let err = engine::interp("foo.baz", Some(Rc::new(ctx_copy3))).unwrap_err();
        println!("Error for foo.baz: {:?}", err);

        let ctx_copy4 = ctx.clone();
        let err2 = engine::interp("nope.bar", Some(Rc::new(ctx_copy4))).unwrap_err();
        println!("Error for nope.bar: {:?}", err2);

        let err3 = engine::interp("foo.bar", None).unwrap_err();
        println!("Error for foo.bar with None context: {:?}", err3);
    }

    #[test]
    fn test_set_parameter() {
        let mut ctx = EvalContext::new();

        let prev = ctx.set_parameter("x", 10.0);
        assert_eq!(prev.unwrap(), None);

        let val = engine::interp("x", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val, 10.0);

        let prev = ctx.set_parameter("x", 20.0);
        assert_eq!(prev.unwrap(), Some(10.0));

        let val = engine::interp("x", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val, 20.0);

        let val = engine::interp("x * 2", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val, 40.0);
    }
}
