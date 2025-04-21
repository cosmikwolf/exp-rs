//! Type definitions for the expression parser and evaluator.
//!
//! This module contains the core data structures used throughout the expression parser
//! and evaluator, including the Abstract Syntax Tree (AST) representation, token types,
//! function definitions, and other auxiliary types.

extern crate alloc;

#[cfg(test)]
use crate::Real;
#[cfg(not(test))]
use crate::{Box, Real, String, Vec};
#[cfg(not(test))]
use alloc::rc::Rc;
#[cfg(test)]
use std::rc::Rc;
#[cfg(test)]
use std::boxed::Box;
#[cfg(test)]
use std::string::String;
#[cfg(test)]
use std::vec::Vec;

/// Abstract Syntax Tree (AST) node representing an expression.
/// 
/// The AST is the core data structure used for representing parsed expressions.
/// Each variant of this enum represents a different type of expression node,
/// forming a tree structure that can be evaluated to produce a result.
#[derive(Clone, Debug, PartialEq)]
pub enum AstExpr {
    /// A literal numerical value.
    /// 
    /// Examples: `3.14`, `42`, `-1.5`
    Constant(Real),
    
    /// A named variable reference.
    /// 
    /// Examples: `x`, `temperature`, `result`
    Variable(String),
    
    /// A function call with a name and list of argument expressions.
    /// 
    /// Examples: `sin(x)`, `max(a, b)`, `sqrt(x*x + y*y)`
    Function { 
        /// The name of the function being called
        name: String, 
        /// The arguments passed to the function
        args: Vec<AstExpr> 
    },
    
    /// An array element access.
    /// 
    /// Examples: `array[0]`, `values[i+1]`
    Array { 
        /// The name of the array
        name: String, 
        /// The expression for the index
        index: Box<AstExpr> 
    },
    
    /// An attribute access on an object.
    /// 
    /// Examples: `point.x`, `settings.value`
    Attribute { 
        /// The base object name
        base: String, 
        /// The attribute name
        attr: String 
    },
}

impl AstExpr {
    /// Helper method that raises a constant expression to a power.
    /// 
    /// This is primarily used in testing to evaluate power operations on constants.
    /// For non-constant expressions, it returns 0.0 as a default value.
    /// 
    /// # Parameters
    /// 
    /// * `exp` - The exponent to raise the constant to
    /// 
    /// # Returns
    /// 
    /// The constant raised to the given power, or 0.0 for non-constant expressions
    pub fn pow(self, exp: Real) -> Real {
        match self {
            #[cfg(feature = "f32")]
            AstExpr::Constant(val) => libm::powf(val, exp),
            #[cfg(not(feature = "f32"))]
            AstExpr::Constant(val) => libm::pow(val, exp),
            _ => 0.0, // Default for non-constant expressions
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ExprError;
    use crate::eval::eval_ast;

    #[test]
    fn test_eval_ast_array_and_attribute_errors() {
        // Array not found
        let ast = AstExpr::Array {
            name: "arr".to_string(),
            index: Box::new(AstExpr::Constant(0.0)),
        };
        let err = eval_ast(&ast, None).unwrap_err();
        match err {
            ExprError::UnknownVariable { name } => assert_eq!(name, "arr"),
            _ => panic!("Expected UnknownVariable error"),
        }
        // Attribute not found
        let ast2 = AstExpr::Attribute {
            base: "foo".to_string(),
            attr: "bar".to_string(),
        };
        let err2 = eval_ast(&ast2, None).unwrap_err();
        match err2 {
            ExprError::AttributeNotFound { base, attr } => {
                assert_eq!(base, "foo");
                assert_eq!(attr, "bar");
            }
            _ => panic!("Expected AttributeNotFound error"),
        }
    }

    #[test]
    fn test_eval_ast_function_wrong_arity() {
        // sin with 2 args (should be 1)
        let ast = AstExpr::Function {
            name: "sin".to_string(),
            args: vec![AstExpr::Constant(1.0), AstExpr::Constant(2.0)],
        };
        let err = eval_ast(&ast, None).unwrap_err();
        match err {
            ExprError::InvalidFunctionCall {
                name,
                expected,
                found,
            } => {
                assert_eq!(name, "sin");
                assert_eq!(expected, 1);
                assert_eq!(found, 2);
            }
            _ => panic!("Expected InvalidFunctionCall error"),
        }
    }

    #[test]
    fn test_eval_ast_unknown_function_and_variable() {
        // Unknown function
        let ast = AstExpr::Function {
            name: "notafunc".to_string(),
            args: vec![AstExpr::Constant(1.0)],
        };
        let err = eval_ast(&ast, None).unwrap_err();
        match err {
            ExprError::UnknownFunction { name } => assert_eq!(name, "notafunc"),
            _ => panic!("Expected UnknownFunction error"),
        }
        // Unknown variable
        let ast2 = AstExpr::Variable("notavar".to_string());
        let err2 = eval_ast(&ast2, None).unwrap_err();
        match err2 {
            ExprError::UnknownVariable { name } => assert_eq!(name, "notavar"),
            _ => panic!("Expected UnknownVariable error"),
        }
    }
}

/// Classifies the kind of expression node in the AST.
/// 
/// This enum is used to categorize expression nodes at a higher level than the specific
/// AST node variants, making it easier to determine the general type of an expression
/// without matching on all variants.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ExprKind {
    /// A constant numerical value.
    Constant,
    
    /// A variable reference.
    Variable,
    
    /// A function call with a specific arity (number of arguments).
    Function { 
        /// Number of arguments the function takes
        arity: usize 
    },
    
    /// An array element access.
    Array,
    
    /// An object attribute access.
    Attribute,
}

/// Classifies the kind of token produced during lexical analysis.
/// 
/// These token types are used by the lexer to categorize different elements
/// in the expression string during the parsing phase.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TokenKind {
    /// A numerical literal.
    Number,
    
    /// A variable identifier.
    Variable,
    
    /// An operator such as +, -, *, /, ^, etc.
    Operator,
    
    /// An opening delimiter like '(' or '['.
    Open,
    
    /// A closing delimiter like ')' or ']'.
    Close,
    
    /// A separator between items, typically a comma.
    Separator,
    
    /// End of the expression.
    End,
    
    /// An error token representing invalid input.
    Error,
    
    /// A null or placeholder token.
    Null,
}

/*
    All legacy bitmasking, ExprType, and OperatorKind have been removed.
    All parser and evaluator logic now uses AstExpr and enums only.
    The old Expr struct and related types are no longer present.
    Next: Update and simplify the test suite to use the new AST parser and evaluator.
*/


/// Represents a native Rust function that can be registered with the evaluation context.
/// 
/// Native functions allow users to extend the expression evaluator with custom
/// functionality written in Rust. These functions can be called from within expressions
/// like any built-in function.
///
/// # Example
///
/// ```text
/// // Create a context
/// let mut ctx = EvalContext::new();
///
/// // Register a custom function that calculates the hypotenuse
/// ctx.register_native_function(
///     "hypotenuse",     // Function name
///     2,                // Takes 2 arguments
///     |args: &[Real]| { // Implementation
///         (args[0] * args[0] + args[1] * args[1]).sqrt()
///     }
/// );
///
/// // Use the function in an expression
/// let result = interp("hypotenuse(3, 4)", Some(Rc::new(ctx))).unwrap();
/// // Result: 5.0
/// ```
#[derive(Clone)]
pub struct NativeFunction<'a> {
    /// Number of arguments the function takes.
    pub arity: usize,
    
    /// The actual implementation of the function as a Rust closure.
    pub implementation: Rc<dyn Fn(&[Real]) -> Real>,
    
    /// The name of the function as it will be used in expressions.
    pub name: Cow<'a, str>,
    
    /// Optional description of what the function does.
    pub description: Option<String>,
}

/* We can't derive Clone for NativeFunction because Box<dyn Fn> doesn't implement Clone.
   Instead, we provide a shallow clone in context.rs for EvalContext, which is safe for read-only use.
   Do NOT call .clone() on NativeFunction directly. */

use alloc::borrow::Cow;

/// Represents a function defined by an expression string rather than Rust code.
/// 
/// Expression functions allow users to define custom functions using the expression
/// language itself. These functions are compiled once when registered and can be called
/// from other expressions. They support parameters and can access variables from the
/// evaluation context.
///
/// # Example
///
/// ```
/// # use exp_rs::{EvalContext, Real};
/// # use exp_rs::engine::interp;
/// # use alloc::rc::Rc;
/// let mut ctx = EvalContext::new();
///
/// // Register a function to calculate the area of a circle
/// ctx.register_expression_function(
///     "circle_area",            // Function name
///     &["radius"],              // Parameter names
///     "pi * radius * radius"    // Function body as an expression
/// ).unwrap();
///
/// // Use the function in another expression
/// let result = interp("circle_area(2)", Some(Rc::new(ctx))).unwrap();
/// assert_eq!(result, 3.141592653589793 * 4.0);
/// ```
#[derive(Clone)]
pub struct ExpressionFunction {
    /// The name of the function as it will be used in expressions.
    pub name: String,
    
    /// The parameter names that the function accepts.
    pub params: Vec<String>,
    
    /// The original expression string defining the function body.
    pub expression: String,
    
    /// The pre-compiled AST of the expression for faster evaluation.
    pub compiled_ast: AstExpr,
    
    /// Optional description of what the function does.
    pub description: Option<String>,
}


/// Internal representation of a variable in the evaluation system.
/// 
/// This is an implementation detail and should not be used directly by library users.
/// Variables are normally managed through the `EvalContext` interface.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct Variable<'a> {
    /// The name of the variable.
    pub name: Cow<'a, str>,
    
    /// Internal address/identifier for the variable.
    pub address: i8,
    
    /// Function associated with the variable (if any).
    pub function: fn(Real, Real) -> Real,
    
    /// Context or associated AST nodes.
    pub context: Vec<AstExpr>,
}

impl<'a> Variable<'a> {
    /// Creates a new variable with the given name and default values.
    pub fn new(name: &'a str) -> Variable<'a> {
        Variable {
            name: Cow::Borrowed(name),
            address: 0,
            function: crate::functions::dummy,
            context: Vec::<AstExpr>::new(),
        }
    }
}
