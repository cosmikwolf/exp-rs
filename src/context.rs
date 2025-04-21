extern crate alloc;

#[cfg(test)]
use crate::Real;
#[cfg(not(test))]
use crate::{Real, String, ToString, Vec};
#[cfg(not(test))]
use alloc::rc::Rc;
#[cfg(not(test))]
use hashbrown::HashMap;
#[cfg(test)]
use std::collections::HashMap;
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
pub struct FunctionRegistry<'a> {
    /// Native functions implemented in Rust code
    pub native_functions: HashMap<String, crate::types::NativeFunction<'a>>,
    /// Functions defined using expression strings 
    pub expression_functions: HashMap<String, crate::types::ExpressionFunction>,
    /// User-defined functions with custom behavior
    pub user_functions: HashMap<String, UserFunction>,
}

use core::cell::RefCell;

/// Evaluation context for expressions.
///
/// This is the main configuration object that holds variables, constants, arrays, functions,
/// and other settings for evaluating expressions. You typically create an `EvalContext`,
/// add your variables and functions, and then pass it to the `interp` function.
///
/// # Examples
///
/// ```text
/// // Create a new evaluation context
/// let mut ctx = EvalContext::new();
/// 
/// // Add variables
/// ctx.set_parameter("x", 5.0);
/// ctx.set_parameter("y", 10.0);
/// 
/// // Add a constant
/// ctx.constants.insert("PI_SQUARED".to_string(), 9.8696);
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
/// ```text
/// // Create a parent context
/// let mut parent = EvalContext::new();
/// parent.set_parameter("x", 1.0);
/// 
/// // Create a child context
/// let mut child = EvalContext::new();
/// child.set_parameter("y", 2.0); 
/// child.parent = Some(Rc::new(parent));
/// 
/// // The child context can access both its own variables and the parent's
/// ```
#[derive(Default)]
pub struct EvalContext<'a> {
    /// Variables that can be modified during evaluation
    pub variables: HashMap<String, Real>,
    /// Constants that cannot be modified during evaluation
    pub constants: HashMap<String, Real>,
    /// Arrays of values that can be accessed using array[index] syntax
    pub arrays: HashMap<String, Vec<Real>>,
    /// Object attributes that can be accessed using object.attribute syntax
    pub attributes: HashMap<String, HashMap<String, Real>>,
    /// Multi-dimensional arrays (not yet fully supported)
    pub nested_arrays: HashMap<String, HashMap<usize, Vec<Real>>>,
    /// Registry of functions available in this context
    pub function_registry: Rc<FunctionRegistry<'a>>,
    /// Optional parent context for variable/function inheritance
    pub parent: Option<Rc<EvalContext<'a>>>,
    /// Optional cache for parsed ASTs to speed up repeated evaluations
    pub ast_cache: Option<RefCell<HashMap<String, Rc<crate::types::AstExpr>>>>,
}

impl<'a> EvalContext<'a> {
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
        Self {
            variables: HashMap::new(),
            constants: HashMap::new(),
            arrays: HashMap::new(),
            attributes: HashMap::new(),
            nested_arrays: HashMap::new(),
            function_registry: Rc::new(FunctionRegistry::default()),
            parent: None,
            ast_cache: None,
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
    /// use alloc::rc::Rc;
    ///
    /// let mut ctx = EvalContext::new();
    /// ctx.set_parameter("x", 42.0);
    ///
    /// let result = interp("x * 2", Some(Rc::new(ctx))).unwrap();
    /// assert_eq!(result, 84.0);
    /// ```
    pub fn set_parameter(&mut self, name: &str, value: Real) -> Option<Real> {
        self.variables.insert(name.to_string(), value)
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
    /// ```text
    /// // Create a context
    /// let mut ctx = EvalContext::new();
    ///
    /// // Register a function that adds all its arguments
    /// ctx.register_native_function("sum", 3, |args| {
    ///     args.iter().sum()
    /// });
    ///
    /// // Use the function in expressions
    /// let result = interp("sum(10, 20, 30)", Some(Rc::new(ctx))).unwrap();
    /// // Result: 60.0
    /// ```
    ///
    /// Functions with variable argument counts:
    ///
    /// ```text
    /// // Create a context
    /// let mut ctx = EvalContext::new();
    ///
    /// // Register a function that accepts a fixed number of arguments
    /// ctx.register_native_function("mean", 5, |args| {
    ///     args.iter().sum::<f64>() / args.len() as f64
    /// });
    ///
    /// // Use the function in expressions
    /// let result = interp("mean(1, 2, 3, 4, 5)", Some(Rc::new(ctx))).unwrap();
    /// // Result: 3.0
    /// ```
    pub fn register_native_function<F>(&mut self, name: &str, arity: usize, implementation: F)
    where
        F: Fn(&[Real]) -> Real + 'static,
    {
        Rc::make_mut(&mut self.function_registry)
            .native_functions
            .insert(
                name.to_string(),
                crate::types::NativeFunction {
                    arity,
                    implementation: Rc::new(implementation),
                    name: name.to_string().into(),
                    description: None,
                },
            );
    }

    /// Registers a function defined by an expression.
    ///
    /// Expression functions are defined by a string expression and a list of parameter names.
    /// They can use other functions and variables available in the context.
    ///
    /// # Parameters
    ///
    /// * `name`: The name of the function as it will be used in expressions
    /// * `params`: The names of the parameters the function accepts
    /// * `expression`: The expression that defines the function's body
    ///
    /// # Returns
    ///
    /// `Ok(())` if the function was registered successfully, or an error if the expression
    /// could not be parsed.
    ///
    /// # Examples
    ///
    /// ```text
    /// // Create a context
    /// let mut ctx = EvalContext::new();
    ///
    /// // Register a function to calculate the hypotenuse
    /// ctx.register_expression_function(
    ///     "hypotenuse",
    ///     &["a", "b"],
    ///     "sqrt(a^2 + b^2)"
    /// ).unwrap();
    ///
    /// // Use the function in expressions
    /// let result = interp("hypotenuse(3, 4)", Some(Rc::new(ctx))).unwrap();
    /// // Result: 5.0
    /// ```
    ///
    /// Expression functions can call other functions:
    ///
    /// ```text
    /// // Create a context
    /// let mut ctx = EvalContext::new();
    ///
    /// // Register a polynomial function
    /// ctx.register_expression_function(
    ///     "polynomial",
    ///     &["x"],
    ///     "x^3 + 2*x^2 + 3*x + 4"
    /// ).unwrap();
    ///
    /// // Use the function in expressions
    /// let result = interp("polynomial(2)", Some(Rc::new(ctx))).unwrap();
    /// // Result: 26.0 (2^3 + 2*2^2 + 3*2 + 4 = 8 + 8 + 6 + 4 = 26)
    /// ```
    pub fn register_expression_function(
        &mut self,
        name: &str,
        params: &[&str],
        expression: &str,
    ) -> Result<(), crate::error::ExprError> {
        // Parse the expression, passing parameter names as reserved variables
        let param_names: Vec<String> = params.iter().map(|&s| s.to_string()).collect();
        let ast = crate::engine::parse_expression_with_reserved(expression, Some(&param_names))?;

        // Store the expression function
        Rc::make_mut(&mut self.function_registry)
            .expression_functions
            .insert(
                name.to_string(),
                crate::types::ExpressionFunction {
                    name: name.to_string(),
                    params: param_names,
                    expression: expression.to_string(),
                    compiled_ast: ast,
                    description: None,
                },
            );

        Ok(())
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
    /// # Examples
    ///
    /// ```
    /// use exp_rs::context::EvalContext;
    /// use exp_rs::engine::interp;
    /// use alloc::rc::Rc;
    ///
    /// let mut ctx = EvalContext::new();
    /// ctx.enable_ast_cache();
    ///
    /// // First evaluation will parse and cache the AST
    /// ctx.set_parameter("x", 1.0);
    /// let result1 = interp("x^2 + 2*x + 1", Some(Rc::new(ctx.clone()))).unwrap();
    ///
    /// // Subsequent evaluations will reuse the cached AST
    /// ctx.set_parameter("x", 2.0);
    /// let result2 = interp("x^2 + 2*x + 1", Some(Rc::new(ctx))).unwrap();
    /// ```
    pub fn enable_ast_cache(&self) {
        if self.ast_cache.is_none() {
            // Use interior mutability to set ast_cache
            let cache = RefCell::new(HashMap::new());
            // SAFETY: We use unsafe to mutate a field in an immutable reference.
            // This is safe because ast_cache is an Option<RefCell<_>> and we only set it once.
            unsafe {
                let self_mut = self as *const _ as *mut Self;
                (*self_mut).ast_cache = Some(cache);
            }
        }
    }

    /// Disables AST caching and clears the cache.
    ///
    /// This is useful if you want to free up memory or if you want to force
    /// re-parsing of expressions.
    ///
    /// # Examples
    ///
    /// ```
    /// use exp_rs::context::EvalContext;
    ///
    /// let ctx = EvalContext::new();
    /// ctx.enable_ast_cache();
    /// // ... use the context with AST caching ...
    /// ctx.disable_ast_cache();
    /// ```
    pub fn disable_ast_cache(&self) {
        // SAFETY: same as above
        unsafe {
            let self_mut = self as *const _ as *mut Self;
            (*self_mut).ast_cache = None;
        }
    }

    /// Clear the AST cache if enabled.
    pub fn clear_ast_cache(&self) {
        if let Some(cache) = self.ast_cache.as_ref() {
            cache.borrow_mut().clear();
        }
    }

    /// Registers all built-in math functions as native functions in the context.
    ///
    /// This is only available if the `no-builtin-math` feature is **not** enabled.
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
    /// # Feature: `no-builtin-math`
    ///
    /// If the `no-builtin-math` feature is enabled, this method is not available and you must
    /// register all required math functions yourself.
    #[cfg(not(feature = "no-builtin-math"))]
    pub fn register_default_math_functions(&mut self) {
        self.register_native_function("abs", 1, |args| crate::functions::abs(args[0], 0.0));
        self.register_native_function("acos", 1, |args| crate::functions::acos(args[0], 0.0));
        self.register_native_function("asin", 1, |args| crate::functions::asin(args[0], 0.0));
        self.register_native_function("atan", 1, |args| crate::functions::atan(args[0], 0.0));
        self.register_native_function("atan2", 2, |args| crate::functions::atan2(args[0], args[1]));
        self.register_native_function("ceil", 1, |args| crate::functions::ceil(args[0], 0.0));
        self.register_native_function("cos", 1, |args| crate::functions::cos(args[0], 0.0));
        self.register_native_function("cosh", 1, |args| crate::functions::cosh(args[0], 0.0));
        self.register_native_function("e", 0, |_args| crate::functions::e(0.0, 0.0));
        self.register_native_function("exp", 1, |args| crate::functions::exp(args[0], 0.0));
        self.register_native_function("floor", 1, |args| crate::functions::floor(args[0], 0.0));
        self.register_native_function("ln", 1, |args| crate::functions::ln(args[0], 0.0));
        self.register_native_function("log", 1, |args| crate::functions::log(args[0], 0.0));
        self.register_native_function("log10", 1, |args| crate::functions::log10(args[0], 0.0));
        self.register_native_function("max", 2, |args| crate::functions::max(args[0], args[1]));
        self.register_native_function("min", 2, |args| crate::functions::min(args[0], args[1]));
        self.register_native_function("pi", 0, |_args| crate::functions::pi(0.0, 0.0));
        self.register_native_function("pow", 2, |args| crate::functions::pow(args[0], args[1]));
        self.register_native_function("^", 2, |args| crate::functions::pow(args[0], args[1]));
        self.register_native_function("sin", 1, |args| crate::functions::sin(args[0], 0.0));
        self.register_native_function("sinh", 1, |args| crate::functions::sinh(args[0], 0.0));
        self.register_native_function("sqrt", 1, |args| crate::functions::sqrt(args[0], 0.0));
        self.register_native_function("tan", 1, |args| crate::functions::tan(args[0], 0.0));
        self.register_native_function("tanh", 1, |args| crate::functions::tanh(args[0], 0.0));
        self.register_native_function("sign", 1, |args| crate::functions::sign(args[0], 0.0));
        self.register_native_function("add", 2, |args| crate::functions::add(args[0], args[1]));
        self.register_native_function("sub", 2, |args| crate::functions::sub(args[0], args[1]));
        self.register_native_function("mul", 2, |args| crate::functions::mul(args[0], args[1]));
        self.register_native_function("div", 2, |args| crate::functions::div(args[0], args[1]));
        self.register_native_function("fmod", 2, |args| crate::functions::fmod(args[0], args[1]));
        self.register_native_function("neg", 1, |args| crate::functions::neg(args[0], 0.0));
        self.register_native_function("comma", 2, |args| crate::functions::comma(args[0], args[1]));
        // Add more as needed
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
    /// If the `no-builtin-math` feature is enabled, built-in math functions are not available,
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
        if let Some(val) = self.variables.get(name) {
            Some(*val)
        } else if let Some(parent) = &self.parent {
            parent.get_variable(name)
        } else {
            None
        }
    }

    pub fn get_constant(&self, name: &str) -> Option<Real> {
        if let Some(val) = self.constants.get(name) {
            Some(*val)
        } else if let Some(parent) = &self.parent {
            parent.get_constant(name)
        } else {
            None
        }
    }

    pub fn get_array(&self, name: &str) -> Option<&Vec<Real>> {
        if let Some(arr) = self.arrays.get(name) {
            Some(arr)
        } else if let Some(parent) = &self.parent {
            parent.get_array(name)
        } else {
            None
        }
    }

    pub fn get_attribute_map(&self, base: &str) -> Option<&HashMap<String, Real>> {
        if let Some(attr_map) = self.attributes.get(base) {
            Some(attr_map)
        } else if let Some(parent) = &self.parent {
            parent.get_attribute_map(base)
        } else {
            None
        }
    }

    pub fn get_native_function(&self, name: &str) -> Option<&crate::types::NativeFunction> {
        if let Some(f) = self.function_registry.native_functions.get(name) {
            Some(f)
        } else if let Some(parent) = &self.parent {
            parent.get_native_function(name)
        } else {
            None
        }
    }

    pub fn get_user_function(&self, name: &str) -> Option<&crate::context::UserFunction> {
        if let Some(f) = self.function_registry.user_functions.get(name) {
            Some(f)
        } else if let Some(parent) = &self.parent {
            parent.get_user_function(name)
        } else {
            None
        }
    }

    pub fn get_expression_function(&self, name: &str) -> Option<&crate::types::ExpressionFunction> {
        if let Some(f) = self.function_registry.expression_functions.get(name) {
            Some(f)
        } else if let Some(parent) = &self.parent {
            parent.get_expression_function(name)
        } else {
            None
        }
    }
}

impl<'a> Clone for EvalContext<'a> {
    fn clone(&self) -> Self {
        Self {
            variables: self.variables.clone(),
            constants: self.constants.clone(),
            arrays: self.arrays.clone(),
            attributes: self.attributes.clone(),
            nested_arrays: self.nested_arrays.clone(),
            function_registry: self.function_registry.clone(),
            parent: self.parent.clone(),
            ast_cache: self.ast_cache.clone(),
        }
    }
}

/// Helper trait to allow shallow cloning of HashMap<String, NativeFunction>
pub trait CloneShallowNativeFunctions<'a> {
    fn clone_shallow(&self) -> HashMap<Cow<'a, str>, crate::types::NativeFunction<'a>>;
}

// For test and non-test, implement shallow clone as just copying the references (not the closures)
impl<'a> CloneShallowNativeFunctions<'a>
    for HashMap<Cow<'a, str>, crate::types::NativeFunction<'a>>
{
    fn clone_shallow(&self) -> HashMap<Cow<'a, str>, crate::types::NativeFunction<'a>> {
        // This is a shallow clone: just copy the map, but do not clone the NativeFunction (which would panic)
        // Instead, just copy the references to the same NativeFunction objects.
        // This is safe as long as the closures are not mutated.
        self.iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    crate::types::NativeFunction {
                        arity: v.arity,
                        implementation: v.implementation.clone(), // just clone the Rc pointer (shallow)
                        name: v.name.clone(),
                        description: v.description.clone(),
                    },
                )
            })
            .collect()
    }
}

use alloc::borrow::Cow;

/// User-defined function.
#[derive(Clone)]
#[allow(dead_code)]
pub struct UserFunction {
    pub params: Vec<String>,
    pub body: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine;
    use crate::types::AstExpr;
    use std::rc::Rc;

    #[test]
    fn test_get_variable_parent_chain() {
        // Create parent context with some variables
        let mut parent_ctx = EvalContext::new();
        parent_ctx.set_parameter("parent_only", 1.0);
        parent_ctx.set_parameter("shadowed", 2.0);
        
        // Create child context with its own variables
        let mut child_ctx = EvalContext::new();
        child_ctx.set_parameter("child_only", 3.0);
        child_ctx.set_parameter("shadowed", 4.0); // Shadows parent's value
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
        grandparent_ctx.set_parameter("grandparent_var", 1.0);
        grandparent_ctx.set_parameter("shadowed", 2.0);

        // Create parent context
        let mut parent_ctx = EvalContext::new();
        parent_ctx.set_parameter("parent_var", 3.0);
        parent_ctx.set_parameter("shadowed", 4.0);
        parent_ctx.parent = Some(Rc::new(grandparent_ctx));

        // Create child context
        let mut child_ctx = EvalContext::new();
        child_ctx.set_parameter("child_var", 5.0);
        child_ctx.set_parameter("shadowed", 6.0);
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
        ctx.set_parameter("x", 1.0);
        ctx.parent = None;

        assert_eq!(ctx.get_variable("x"), Some(1.0));
        assert_eq!(ctx.get_variable("nonexistent"), None);
    }

    #[test]
    fn test_get_variable_cyclic_reference_safety() {
        // Create two contexts that will reference each other
        let mut ctx1 = EvalContext::new();
        let mut ctx2 = EvalContext::new();
        
        ctx1.set_parameter("var1", 1.0);
        ctx2.set_parameter("var2", 2.0);

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
        ctx.set_parameter("x", 100.0);
        
        // Create a function context that uses 'x' as parameter
        let mut func_ctx = EvalContext::new();
        func_ctx.set_parameter("x", 5.0); // Parameter value
        func_ctx.parent = Some(Rc::new(ctx.clone()));
        
        // Test variable lookup in function scope
        assert_eq!(func_ctx.get_variable("x"), Some(5.0), 
            "Function parameter should shadow parent variable");
            
        // Print debug info
        println!("Parent context x = {:?}", ctx.get_variable("x"));
        println!("Function context x = {:?}", func_ctx.get_variable("x"));
        println!("Function context variables: {:?}", func_ctx.variables);
        println!("Function context parent variables: {:?}", 
            func_ctx.parent.as_ref().map(|p| &p.variables));
    }

    #[test]
    fn test_get_variable_nested_scopes() {
        let mut root_ctx = EvalContext::new();
        root_ctx.set_parameter("x", 1.0);
        root_ctx.set_parameter("y", 1.0);
        
        let mut mid_ctx = EvalContext::new();
        mid_ctx.set_parameter("x", 2.0);
        mid_ctx.parent = Some(Rc::new(root_ctx));
        
        let mut leaf_ctx = EvalContext::new();
        leaf_ctx.set_parameter("x", 3.0);
        leaf_ctx.parent = Some(Rc::new(mid_ctx));
        
        // Test variable lookup at each level
        assert_eq!(leaf_ctx.get_variable("x"), Some(3.0),
            "Should get leaf context value");
        assert_eq!(leaf_ctx.get_variable("y"), Some(1.0),
            "Should get root context value when not shadowed");
            
        println!("Variable lookup in nested scopes:");
        println!("leaf x = {:?}", leaf_ctx.get_variable("x"));
        println!("leaf y = {:?}", leaf_ctx.get_variable("y"));
        println!("leaf variables: {:?}", leaf_ctx.variables);
        println!("mid variables: {:?}", 
            leaf_ctx.parent.as_ref().map(|p| &p.variables));
        println!("root variables: {:?}", 
            leaf_ctx.parent.as_ref().and_then(|p| p.parent.as_ref()).map(|p| &p.variables));
    }

    #[test]
    fn test_get_variable_function_parameter_precedence() {
        let mut ctx = EvalContext::new();
        
        // Register a function that uses parameter 'x'
        ctx.register_expression_function("f", &["x"], "x * 2").unwrap();
        
        // Set a global 'x'
        ctx.set_parameter("x", 100.0);
        
        // Create evaluation context for function
        let mut func_ctx = EvalContext::new();
        func_ctx.set_parameter("x", 5.0); // Parameter value
        func_ctx.parent = Some(Rc::new(ctx));
        
        println!("Function parameter context:");
        println!("func_ctx x = {:?}", func_ctx.get_variable("x"));
        println!("func_ctx variables: {:?}", func_ctx.variables);
        println!("parent variables: {:?}", 
            func_ctx.parent.as_ref().map(|p| &p.variables));
        
        assert_eq!(func_ctx.get_variable("x"), Some(5.0),
            "Function parameter should take precedence over global x");
    }

    #[test]
    fn test_get_variable_temporary_scope() {
        let mut ctx = EvalContext::new();
        ctx.set_parameter("x", 1.0);
        
        // Create temporary scope
        let mut temp_ctx = EvalContext::new();
        temp_ctx.parent = Some(Rc::new(ctx));
        
        // Variable lookup should find parent value
        assert_eq!(temp_ctx.get_variable("x"), Some(1.0),
            "Should find variable in parent scope");
        
        // Add variable to temporary scope
        temp_ctx.set_parameter("x", 2.0);
        
        // Should now find local value
        assert_eq!(temp_ctx.get_variable("x"), Some(2.0),
            "Should find shadowed variable in local scope");
            
        println!("Temporary scope variable lookup:");
        println!("temp x = {:?}", temp_ctx.get_variable("x")); 
        println!("temp variables: {:?}", temp_ctx.variables);
        println!("parent variables: {:?}",
            temp_ctx.parent.as_ref().map(|p| &p.variables));
    }

    #[test]
    fn test_native_function() {
        let mut ctx = EvalContext::new();

        ctx.register_native_function("add_all", 3, |args| args.iter().sum());

        let val = engine::interp("add_all(1, 2, 3)", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val, 6.0);
    }

    #[test]
    fn test_expression_function() {
        let mut ctx = EvalContext::new();

        ctx.register_expression_function("double", &["x"], "x * 2")
            .unwrap();

        ctx.variables.insert("value".to_string().into(), 5.0);

        let val = engine::interp("double(value)", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val, 10.0);

        let val2 = engine::interp("double(7)", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val2, 14.0);
    }

    #[test]
    fn test_array_access() {
        let mut ctx = EvalContext::new();
        ctx.arrays.insert(
            "climb_wave_wait_time".to_string().into(),
            vec![10.0, 20.0, 30.0],
        );
        let val = engine::interp("climb_wave_wait_time[1]", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val, 20.0);
    }

    #[test]
    fn test_array_access_ast_structure() {
        let mut ctx = EvalContext::new();
        ctx.arrays.insert(
            "climb_wave_wait_time".to_string().into(),
            vec![10.0, 20.0, 30.0],
        );
        let ast = engine::parse_expression("climb_wave_wait_time[1]").unwrap();
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
        let mut foo_map = HashMap::new();
        foo_map.insert("bar".to_string().into(), 42.0);
        ctx.attributes.insert("foo".to_string().into(), foo_map);

        let ast = engine::parse_expression("foo.bar").unwrap();
        println!("AST for foo.bar: {:?}", ast);

        let ctx_copy = ctx.clone();
        let eval_result = crate::eval::eval_ast(&ast, Some(Rc::new(ctx_copy)));
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
        assert_eq!(prev, None);

        let val = engine::interp("x", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val, 10.0);

        let prev = ctx.set_parameter("x", 20.0);
        assert_eq!(prev, Some(10.0));

        let val = engine::interp("x", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val, 20.0);

        let val = engine::interp("x * 2", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(val, 40.0);
    }
}
