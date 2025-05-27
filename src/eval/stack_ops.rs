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
#[derive(Debug, Clone)]
pub enum EvalOp {
    /// Push an expression to evaluate
    Eval { 
        expr: AstExpr, 
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
        right_expr: AstExpr, 
        ctx_id: usize 
    },
    
    /// Short-circuit OR operation  
    ShortCircuitOr { 
        right_expr: AstExpr, 
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
        true_branch: AstExpr,
        false_branch: AstExpr,
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
            BinaryOp::Power => left.powf(right),
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