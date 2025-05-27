//! Error types and handling for the exp-rs crate.
//!
//! This module defines the error types used throughout the exp-rs crate for expression parsing
//! and evaluation. It provides detailed error information to help diagnose issues in expressions.

extern crate alloc;
use alloc::string::String;
#[cfg(not(test))]
use core::num::ParseFloatError;
#[cfg(test)]
use std::num::ParseFloatError;

#[cfg(not(test))]
use core::result;
#[cfg(test)]
use std::result;

/// Result type used throughout the crate.
///
/// This is a convenience type alias that uses the `ExprError` type for the error variant.
pub type Result<T> = result::Result<T, ExprError>;

/// Error type for expression parsing and evaluation.
///
/// This enum represents all possible errors that can occur during expression parsing,
/// tokenization, and evaluation. It provides specific error variants with detailed
/// information to help diagnose and fix issues.
#[derive(Debug, Clone)]
pub enum ExprError {
    /// Error when parsing a floating point number.
    ///
    /// This occurs when a string cannot be converted to a floating point number.
    /// For example, "3.a" is not a valid floating point number.
    Parse(ParseFloatError),
    
    /// Error during lexical analysis (tokenization).
    ///
    /// This occurs when the tokenizer encounters invalid tokens or unknown characters
    /// that cannot be processed. The string contains a detailed error message.
    Tokenizer(String),
    
    /// Error during syntax analysis.
    ///
    /// This occurs when the parser encounters unexpected tokens, incorrect expression
    /// structure, or other syntax issues. The string contains a detailed error message.
    Syntax(String),
    
    /// Error for unmatched parentheses in an expression.
    ///
    /// This provides the position of the unmatched parenthesis and the specific
    /// parenthesis character that was found without a matching pair.
    UnmatchedParenthesis { position: usize, found: String },
    
    /// Error when a variable referenced in an expression is not defined.
    ///
    /// To resolve this error, make sure the variable is registered in the
    /// evaluation context using `EvalContext::set_parameter`.
    UnknownVariable { name: String },
    /// Unknown function error
    ///
    /// This error is returned when a function is called that is not registered in the context
    /// and is not a built-in (if built-ins are enabled). If the `libm` feature is not enabled,
    /// users must register their own native functions for all required math operations.
    /// 
    /// To resolve this error, register a native function with `EvalContext::register_native_function`
    /// or an expression function with `EvalContext::register_expression_function`.
    UnknownFunction { name: String },
    /// Error when a function is called with the wrong number of arguments.
    ///
    /// This occurs when a function is called with fewer or more arguments than it expects.
    /// The error includes the function name, the expected number of arguments, and the
    /// actual number of arguments provided.
    InvalidFunctionCall {
        /// Name of the function that was called
        name: String,
        /// Expected number of arguments
        expected: usize,
        /// Actual number of arguments provided
        found: usize,
    },
    /// Error when an array index is out of bounds.
    ///
    /// This occurs when trying to access an array element with an index that exceeds
    /// the array's length. The error includes the array name, the attempted index,
    /// and the actual length of the array.
    ArrayIndexOutOfBounds {
        /// Name of the array being accessed
        name: String,
        /// Index that was attempted to be accessed
        index: usize,
        /// Actual length of the array
        len: usize,
    },
    
    /// Error when an attribute access is attempted on an object that doesn't have that attribute.
    ///
    /// This occurs when using the dot notation (e.g., `object.attribute`) and the attribute
    /// does not exist on the specified object.
    AttributeNotFound { 
        /// The base object name
        base: String, 
        /// The attribute name that was not found
        attr: String 
    },
    
    /// Error when division by zero is attempted.
    ///
    /// This occurs when a division operation has a zero divisor.
    DivideByZero,
    
    /// General-purpose error for any other error conditions.
    ///
    /// This is used for errors that don't fit into other specific categories.
    /// The string contains a detailed error message.
    Other(String),
    
    /// Error when the recursion limit is exceeded during expression evaluation.
    ///
    /// This usually happens with deeply nested expressions or recursive function calls.
    /// To resolve this, simplify the expression or increase the recursion limit if possible.
    RecursionLimit(String),
    
    /// Error when capacity is exceeded for a heapless container.
    ///
    /// This occurs when trying to insert into a full heapless container.
    /// The string indicates which container type exceeded capacity.
    CapacityExceeded(&'static str),
    
    /// Error when a string is too long for heapless string buffer.
    ///
    /// This occurs when trying to create a heapless string that exceeds
    /// the maximum string length limit.
    StringTooLong,
}

#[cfg(not(test))]
use core::fmt;
#[cfg(test)]
use std::fmt;

impl fmt::Display for ExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExprError::Parse(_) => write!(f, "Parse error"),
            ExprError::Tokenizer(err) => write!(f, "Tokenizer error: {}", err),
            ExprError::Syntax(err) => write!(f, "Syntax error: {}", err),
            ExprError::UnmatchedParenthesis { position, found } => {
                write!(
                    f,
                    "Unmatched parenthesis at position {}: found '{}'",
                    position, found
                )
            }
            ExprError::UnknownVariable { name } => {
                write!(f, "Unknown variable: '{}'", name)
            }
            ExprError::UnknownFunction { name } => {
                write!(f, "Unknown function: '{}'", name)
            }
            ExprError::InvalidFunctionCall {
                name,
                expected,
                found,
            } => {
                write!(
                    f,
                    "Invalid function call to '{}': expected {} arguments, found {}",
                    name, expected, found
                )
            }
            ExprError::ArrayIndexOutOfBounds { name, index, len } => {
                write!(
                    f,
                    "Array index out of bounds: index {} out of bounds for '{}', length {}",
                    index, name, len
                )
            }
            ExprError::AttributeNotFound { base, attr } => {
                write!(f, "Attribute not found: '{}' in '{}'", attr, base)
            }
            ExprError::DivideByZero => write!(f, "Division by zero"),
            ExprError::Other(err) => write!(f, "{}", err),
            ExprError::RecursionLimit(err) => write!(f, "Recursion limit exceeded: {}", err),
            ExprError::CapacityExceeded(container_type) => {
                write!(f, "Capacity exceeded for {}", container_type)
            }
            ExprError::StringTooLong => write!(f, "String too long for heapless buffer"),
        }
    }
}

impl From<String> for ExprError {
    fn from(err: String) -> ExprError {
        ExprError::Other(err)
    }
}

impl From<ParseFloatError> for ExprError {
    fn from(err: ParseFloatError) -> ExprError {
        ExprError::Parse(err)
    }
}
