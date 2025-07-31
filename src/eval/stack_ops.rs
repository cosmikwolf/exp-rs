//! Stack-based operations for iterative AST evaluation
//! 
//! This module defines the operation types used by the iterative evaluator
//! to process expressions without recursion.

use crate::types::{AstExpr, HString, FunctionName};
use crate::error::ExprError;
use crate::Real;
use alloc::vec::Vec;
use alloc::format;

/// Operations that can be pushed onto the evaluation stack
#[derive(Clone)]
pub enum EvalOp<'arena> {
    /// Push an expression to evaluate
    Eval { 
        expr: &'arena AstExpr<'arena>, 
        ctx_id: usize 
    },
    
    /// Apply a unary operation
    ApplyUnary { 
        op: UnaryOp 
    },
    
    /// Apply a binary operation after both operands are evaluated
    CompleteBinary { 
        op: BinaryOp 
    },
    
    /// Short-circuit AND operation
    ShortCircuitAnd { 
        right_expr: &'arena AstExpr<'arena>, 
        ctx_id: usize 
    },
    
    /// Short-circuit OR operation  
    ShortCircuitOr { 
        right_expr: &'arena AstExpr<'arena>, 
        ctx_id: usize 
    },
    
    /// Complete AND operation (when not short-circuited)
    CompleteAnd,
    
    /// Complete OR operation (when not short-circuited)
    CompleteOr,
    
    /// Apply a function with N arguments
    ApplyFunction { 
        name: FunctionName,
        args_needed: usize,
        args_collected: Vec<Real>,
        ctx_id: usize,
    },
    
    /// Collect function arguments from the value stack
    CollectFunctionArgs {
        name: FunctionName,
        total_args: usize,
        args_so_far: Vec<Real>,
        ctx_id: usize,
    },
    
    /// Handle ternary operator - condition already evaluated
    TernaryCondition { 
        true_branch: &'arena AstExpr<'arena>,
        false_branch: &'arena AstExpr<'arena>,
        ctx_id: usize,
    },
    
    /// Variable lookup
    LookupVariable { 
        name: HString, 
        ctx_id: usize 
    },
    
    /// Array access - index already evaluated
    AccessArray { 
        array_name: HString, 
        ctx_id: usize 
    },
    
    /// Attribute access
    AccessAttribute { 
        object_name: HString,
        attr_name: HString,
        ctx_id: usize,
    },
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Negate,
    Not,
}

/// Binary operators  
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Equal,
    NotEqual,
    // Note: AND and OR are handled separately for short-circuiting
}

impl UnaryOp {
    /// Apply a unary operation to a value
    pub fn apply(self, operand: Real) -> Real {
        match self {
            UnaryOp::Negate => -operand,
            UnaryOp::Not => if operand == 0.0 { 1.0 } else { 0.0 },
        }
    }
}

impl BinaryOp {
    /// Apply a binary operation to two values
    pub fn apply(self, left: Real, right: Real) -> Real {
        match self {
            BinaryOp::Add => left + right,
            BinaryOp::Subtract => left - right,
            BinaryOp::Multiply => left * right,
            BinaryOp::Divide => left / right,
            BinaryOp::Modulo => left % right,
            BinaryOp::Power => {
                #[cfg(feature = "libm")]
                {
                    crate::functions::pow(left, right)
                }
                #[cfg(not(feature = "libm"))]
                {
                    // In no_std without libm, power operations must be handled by registered functions
                    // This should not be reached as the parser would create a function call instead
                    panic!("Power operation requires libm feature or registered pow function")
                }
            },
            BinaryOp::Less => if left < right { 1.0 } else { 0.0 },
            BinaryOp::Greater => if left > right { 1.0 } else { 0.0 },
            BinaryOp::LessEqual => if left <= right { 1.0 } else { 0.0 },
            BinaryOp::GreaterEqual => if left >= right { 1.0 } else { 0.0 },
            BinaryOp::Equal => if left == right { 1.0 } else { 0.0 },
            BinaryOp::NotEqual => if left != right { 1.0 } else { 0.0 },
        }
    }
}

/// Convert from AST representation to stack operation
pub fn ast_to_stack_op(op: &str) -> Result<BinaryOp, ExprError> {
    match op {
        "+" => Ok(BinaryOp::Add),
        "-" => Ok(BinaryOp::Subtract),
        "*" => Ok(BinaryOp::Multiply),
        "/" => Ok(BinaryOp::Divide),
        "%" => Ok(BinaryOp::Modulo),
        "^" | "**" => Ok(BinaryOp::Power),
        "<" => Ok(BinaryOp::Less),
        ">" => Ok(BinaryOp::Greater),
        "<=" => Ok(BinaryOp::LessEqual),
        ">=" => Ok(BinaryOp::GreaterEqual),
        "==" => Ok(BinaryOp::Equal),
        "!=" => Ok(BinaryOp::NotEqual),
        _ => Err(ExprError::Syntax(format!("Unknown operator: {}", op))),
    }
}

/// Check if a string is a binary operator
pub fn is_binary_operator(op: &str) -> bool {
    matches!(op, 
        "+" | "-" | "*" | "/" | "%" | "^" | "**" |
        "<" | ">" | "<=" | ">=" | "==" | "!="
    )
}

impl<'arena> core::fmt::Debug for EvalOp<'arena> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EvalOp::Eval { expr: _, ctx_id } => {
                write!(f, "Eval {{ expr: <AstExpr>, ctx_id: {} }}", ctx_id)
            }
            EvalOp::ApplyUnary { op } => {
                write!(f, "ApplyUnary {{ op: {:?} }}", op)
            }
            EvalOp::CompleteBinary { op } => {
                write!(f, "CompleteBinary {{ op: {:?} }}", op)
            }
            EvalOp::ShortCircuitAnd { right_expr: _, ctx_id } => {
                write!(f, "ShortCircuitAnd {{ right_expr: <AstExpr>, ctx_id: {} }}", ctx_id)
            }
            EvalOp::ShortCircuitOr { right_expr: _, ctx_id } => {
                write!(f, "ShortCircuitOr {{ right_expr: <AstExpr>, ctx_id: {} }}", ctx_id)
            }
            EvalOp::CompleteAnd => write!(f, "CompleteAnd"),
            EvalOp::CompleteOr => write!(f, "CompleteOr"),
            EvalOp::ApplyFunction { name, args_needed, args_collected, ctx_id } => {
                write!(f, "ApplyFunction {{ name: {:?}, args_needed: {}, args_collected: {:?}, ctx_id: {} }}", 
                    name, args_needed, args_collected, ctx_id)
            }
            EvalOp::CollectFunctionArgs { name, total_args, args_so_far, ctx_id } => {
                write!(f, "CollectFunctionArgs {{ name: {:?}, total_args: {}, args_so_far: {:?}, ctx_id: {} }}", 
                    name, total_args, args_so_far, ctx_id)
            }
            EvalOp::LookupVariable { name, ctx_id } => {
                write!(f, "LookupVariable {{ name: {:?}, ctx_id: {} }}", name, ctx_id)
            }
            EvalOp::TernaryCondition { true_branch: _, false_branch: _, ctx_id } => {
                write!(f, "TernaryCondition {{ true_branch: <AstExpr>, false_branch: <AstExpr>, ctx_id: {} }}", ctx_id)
            }
            EvalOp::AccessArray { array_name, ctx_id } => {
                write!(f, "AccessArray {{ array_name: {:?}, ctx_id: {} }}", array_name, ctx_id)
            }
            EvalOp::AccessAttribute { object_name, attr_name, ctx_id } => {
                write!(f, "AccessAttribute {{ object_name: {:?}, attr_name: {:?}, ctx_id: {} }}", 
                    object_name, attr_name, ctx_id)
            }
        }
    }
}