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

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TokenKind {
    Number,
    Variable,
    Operator,
    Open,
    Close,
    Separator,
    End,
    Error,
    Null,
    // Add more as needed
}

/*
    All legacy bitmasking, ExprType, and OperatorKind have been removed.
    All parser and evaluator logic now uses AstExpr and enums only.
    The old Expr struct and related types are no longer present.
    Next: Update and simplify the test suite to use the new AST parser and evaluator.
*/


/// Represents a native function that can be registered with the evaluation context.
#[derive(Clone)]
pub struct NativeFunction<'a> {
    pub arity: usize,
    pub implementation: Rc<dyn Fn(&[Real]) -> Real>,
    pub name: Cow<'a, str>,
    pub description: Option<String>,
}

/* We can't derive Clone for NativeFunction because Box<dyn Fn> doesn't implement Clone.
   Instead, we provide a shallow clone in context.rs for EvalContext, which is safe for read-only use.
   Do NOT call .clone() on NativeFunction directly. */

use alloc::borrow::Cow;

/// Represents a function defined by an expression string.
#[derive(Clone)]
pub struct ExpressionFunction {
    pub name: String,
    pub params: Vec<String>,
    pub expression: String,
    pub compiled_ast: AstExpr,
    pub description: Option<String>,
}


#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct Variable<'a> {
    pub name: Cow<'a, str>,
    pub address: i8,
    pub function: fn(Real, Real) -> Real,
    pub context: Vec<AstExpr>,
}

impl<'a> Variable<'a> {
    pub fn new(name: &'a str) -> Variable<'a> {
        Variable {
            name: Cow::Borrowed(name),
            address: 0,
            function: crate::functions::dummy,
            context: Vec::<AstExpr>::new(),
        }
    }
}
