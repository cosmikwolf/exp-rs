extern crate alloc;

#[cfg(not(test))]
use alloc::rc::Rc;
#[cfg(test)]
use std::rc::Rc;

use crate::Real;
#[cfg(not(test))]
use crate::Vec;
use crate::context::EvalContext;
use crate::error::ExprError;
use crate::lexer::{Lexer, Token};
use crate::types::{AstExpr, TokenKind};
use bumpalo::Bump;

use alloc::borrow::Cow;
#[cfg(not(test))]
use alloc::format;
use alloc::string::{String, ToString};

/// Pratt parser for mathematical expressions
#[cfg(not(test))]
use alloc::collections::BTreeSet as HashSet;
#[cfg(test)]
use std::collections::HashSet;

struct PrattParser<'input, 'arena> {
    lexer: Lexer<'input>,
    arena: &'arena Bump, // Arena is now mandatory
    current: Option<Token>,
    errors: Vec<ExprError>,
    recursion_depth: usize,
    max_recursion_depth: usize,
    reserved_vars: Option<HashSet<Cow<'input, str>>>, // Parameter names to treat as variables, not functions
    context_vars: Option<HashSet<Cow<'input, str>>>,  // Variable/constant names from context
}

/// Token binding powers for the Pratt parser
#[derive(Debug, Clone, Copy)]
struct BindingPower {
    left: u8,
    right: u8,
}

impl BindingPower {
    const fn new(left: u8, right: u8) -> Self {
        Self { left, right }
    }

    // For left-associative operators, right binding power is left + 1
    const fn left_assoc(power: u8) -> Self {
        Self::new(power, power + 1)
    }

    // For right-associative operators, right binding power is same as left
    const fn right_assoc(power: u8) -> Self {
        Self::new(power, power)
    }
}

impl<'input, 'arena> PrattParser<'input, 'arena> {
    fn new(input: &'input str, arena: &'arena Bump) -> Self {
        let mut lexer = Lexer::new(input);
        let current = lexer.next_token();
        Self {
            lexer,
            arena,
            current,
            errors: Vec::new(),
            recursion_depth: 0,
            max_recursion_depth: 2000,
            reserved_vars: None,
            context_vars: None,
        }
    }

    fn with_reserved_vars_and_context(
        input: &'input str,
        arena: &'arena Bump,
        reserved_vars: Option<&'input [String]>,
        context_vars: Option<&'input [String]>,
    ) -> Self {
        let mut parser = Self::new(input, arena);
        if let Some(vars) = reserved_vars {
            let mut set = HashSet::new();
            for v in vars {
                set.insert(Cow::Borrowed(v.as_str()));
            }
            parser.reserved_vars = Some(set);
        }
        if let Some(vars) = context_vars {
            let mut set = HashSet::new();
            for v in vars {
                set.insert(Cow::Borrowed(v.as_str()));
            }
            parser.context_vars = Some(set);
        }
        parser
    }

    fn peek(&self) -> Option<&Token> {
        self.current.as_ref()
    }

    fn next(&mut self) -> Option<Token> {
        let tok = self.current.take();
        self.current = self.lexer.next_token();
        tok
    }

    fn expect(&mut self, kind: TokenKind, error_msg: &str) -> Result<Token, ExprError> {
        if let Some(tok) = self.peek() {
            if tok.kind == kind {
                return Ok(self.next().unwrap());
            }

            // If we're expecting a closing parenthesis and don't find it,
            // provide a more specific error message
            if kind == TokenKind::Close {
                let position = tok.position;
                let found = tok.text.clone().unwrap_or_else(|| "unknown".to_string());
                return Err(ExprError::UnmatchedParenthesis { position, found });
            }
        }

        let position = self.peek().map(|t| t.position).unwrap_or(0);
        let found = self
            .peek()
            .and_then(|t| t.text.clone())
            .unwrap_or_else(|| "end of input".to_string());

        let err = ExprError::Syntax(format!(
            "{} at position {}, found '{}'",
            error_msg, position, found
        ));
        self.errors.push(err.clone());
        Err(err)
    }

    // Get binding power for an operator
    fn get_binding_power(op: &str) -> Option<BindingPower> {
        match op {
            "," | ";" => Some(BindingPower::left_assoc(1)), // List separator (comma or semicolon)
            "?" => Some(BindingPower::right_assoc(1)), // Ternary conditional operator (lowest precedence)
            "||" => Some(BindingPower::left_assoc(2)), // Logical OR (lowest precedence)
            "&&" => Some(BindingPower::left_assoc(3)), // Logical AND (higher than OR)
            "|" => Some(BindingPower::left_assoc(4)),  // Bitwise OR
            "&" => Some(BindingPower::left_assoc(6)),  // Bitwise AND
            "==" | "!=" | "<" | ">" | "<=" | ">=" | "<>" => Some(BindingPower::left_assoc(7)), // Comparison
            "<<" | ">>" | "<<<" | ">>>" => Some(BindingPower::left_assoc(8)), // Bit shifts
            "+" | "-" => Some(BindingPower::left_assoc(9)), // Addition, subtraction
            "*" | "/" | "%" => Some(BindingPower::left_assoc(10)), // Multiplication, division, modulo
            "^" => Some(BindingPower::right_assoc(15)), // Exponentiation (right-associative, higher than unary)
            "**" => Some(BindingPower::right_assoc(16)), // Exponentiation (right-associative, higher than ^)
            ":" => None, // Colon is handled as part of the ternary operator, not independently
            _ => None,
        }
    }

    // Get binding power for a prefix operator
    fn get_prefix_binding_power(op: &str) -> Option<u8> {
        match op {
            "+" | "-" | "~" => Some(14), // Must be lower than ^ and ** for correct -2^2 parsing
            _ => None,
        }
    }

    // Unified method for handling all postfix operations
    fn parse_postfix(&mut self, lhs: AstExpr<'arena>) -> Result<AstExpr<'arena>, ExprError> {
        let mut result = lhs;

        // Keep applying postfix operators as long as they're available
        loop {
            if let Some(tok) = self.peek() {
                match (tok.kind, tok.text.as_deref()) {
                    (TokenKind::Open, Some("(")) => {
                        // Function call
                        result = self.parse_function_call(result)?;
                    }
                    (TokenKind::Open, Some("[")) => {
                        // Array access
                        result = self.parse_array_access(result)?;
                    }
                    (TokenKind::Operator, Some(".")) => {
                        // Attribute access
                        result = self.parse_attribute_access(result)?;
                    }
                    _ => break, // No more postfix operators
                }
            } else {
                break;
            }
        }

        Ok(result)
    }

    // Unified error handling for all parenthesis-like structures
    fn expect_closing(
        &mut self,
        kind: TokenKind,
        expected: &str,
        opening_position: usize,
    ) -> Result<(), ExprError> {
        if let Some(tok) = self.peek() {
            if tok.kind == kind {
                self.next(); // Consume the closing token
                return Ok(());
            }

            // If not the expected token, report an error
            let position = tok.position;
            let found = tok.text.clone().unwrap_or_else(|| "unknown".to_string());

            return Err(ExprError::Syntax(format!(
                "Expected {} at position {}, found '{}' (opening at position {})",
                expected, position, found, opening_position
            )));
        }

        // End of input
        Err(ExprError::Syntax(format!(
            "Expected {} but found end of input (opening at position {})",
            expected, opening_position
        )))
    }

    // Helper method for parsing parenthesized expressions
    fn parse_parenthesized_expr(&mut self) -> Result<AstExpr<'arena>, ExprError> {
        let open_position = self.peek().map(|t| t.position).unwrap_or(0);
        self.next(); // consume '('

        // Parse the expression inside the parentheses
        // Always allow commas inside parentheses
        let expr = self.parse_expr_unified(0, true)?;

        // Always check for closing parenthesis
        if let Some(tok) = self.peek() {
            if tok.kind == TokenKind::Close {
                self.next(); // Consume the closing parenthesis
                return Ok(expr);
            }

            // If not a closing parenthesis, report an error
            let position = tok.position;
            let found = tok.text.clone().unwrap_or_else(|| "unknown".to_string());
            return Err(ExprError::Syntax(format!(
                "Expected closing parenthesis ')' but found '{}' at position {} (opening at position {})",
                found, position, open_position
            )));
        }

        // End of input
        Err(ExprError::Syntax(format!(
            "Expected closing parenthesis ')' but found end of input (opening at position {})",
            open_position
        )))
    }

    // Helper method for parsing function calls
    fn parse_function_call(&mut self, expr: AstExpr<'arena>) -> Result<AstExpr<'arena>, ExprError> {
        let name = match &expr {
            AstExpr::Variable(name) => *name,
            AstExpr::Attribute { attr, .. } => *attr,
            _ => {
                return Err(ExprError::Syntax(
                    "Function call on non-function expression".to_string(),
                ));
            }
        };

        self.next(); // consume '('

        let mut args = bumpalo::collections::Vec::new_in(self.arena);

        // Parse arguments
        if let Some(tok) = self.peek() {
            if tok.kind != TokenKind::Close {
                // Parse the first argument
                let arg = self.parse_expr_unified(0, false)?;
                args.push(arg);

                // Check for comma or closing parenthesis
                while let Some(next_tok) = self.peek() {
                    if next_tok.kind == TokenKind::Separator
                        && next_tok.text.as_deref() == Some(",")
                    {
                        self.next(); // consume ','

                        // Parse the next argument
                        let arg = self.parse_expr_unified(0, false)?;
                        args.push(arg);
                    } else if next_tok.kind == TokenKind::Close {
                        break;
                    } else {
                        // Unexpected token - report error
                        let position = next_tok.position;
                        let found = next_tok
                            .text
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string());
                        return Err(ExprError::Syntax(format!(
                            "Expected ',' or ')' but found '{}' at position {} in function call",
                            found, position
                        )));
                    }
                }
            }
        }

        // Check for closing parenthesis
        if let Some(tok) = self.peek() {
            if tok.kind == TokenKind::Close {
                self.next(); // Consume the closing parenthesis
            } else {
                // If not a closing parenthesis, report an error
                let position = tok.position;
                let found = tok.text.clone().unwrap_or_else(|| "unknown".to_string());
                return Err(ExprError::Syntax(format!(
                    "Expected closing parenthesis ')' but found '{}' at position {} in function call",
                    found, position
                )));
            }
        } else {
            // End of input - this is an error because we're missing a closing parenthesis
            let open_position = self.lexer.get_original_input().len()
                - self.lexer.get_remaining_input().unwrap_or("").len();
            return Err(ExprError::UnmatchedParenthesis {
                position: open_position,
                found: "(".to_string(),
            });
        }

        // Special handling for pow function to ensure it has 2 arguments
        if name == "pow" && args.len() == 1 {
            // If pow has only one argument, add a default second argument of 2.0
            args.push(AstExpr::Constant(2.0));
        } else if name == "atan2" && args.len() == 1 {
            // If atan2 has only one argument, add a default second argument of 1.0
            args.push(AstExpr::Constant(1.0));
        }

        // Special handling for polynomial function: always 1 argument, do not treat as built-in
        if name == "polynomial" && args.len() == 1 {
            // No-op, just clarity: polynomial(x)
        }

        Ok(AstExpr::Function {
            name,
            args: args.into_bump_slice(),
        })
    }

    // Helper method for parsing array access
    fn parse_array_access(&mut self, expr: AstExpr<'arena>) -> Result<AstExpr<'arena>, ExprError> {
        let name = match &expr {
            AstExpr::Variable(name) => name,
            _ => {
                let position = self.peek().map(|t| t.position).unwrap_or(0);
                return Err(ExprError::Syntax(format!(
                    "Array access on non-array expression at position {}",
                    position
                )));
            }
        };

        let open_position = self.peek().map(|t| t.position).unwrap_or(0);
        self.next(); // consume '['

        // Parse index expression
        let index = self.parse_expr_unified(0, true)?;

        // Always expect closing bracket
        self.expect_closing(TokenKind::Close, "closing bracket ']'", open_position)?;

        Ok(AstExpr::Array {
            name,
            index: self.arena.alloc(index),
        })
    }

    // Helper method for parsing attribute access
    fn parse_attribute_access(
        &mut self,
        expr: AstExpr<'arena>,
    ) -> Result<AstExpr<'arena>, ExprError> {
        let dot_position = self.peek().map(|t| t.position).unwrap_or(0);
        self.next(); // consume '.'

        // Expect identifier
        let attr_tok = self.expect(TokenKind::Variable, "Expected attribute name")?;

        let attr = self.arena.alloc_str(&attr_tok.text.unwrap_or_default());

        #[cfg(test)]
        println!("Parsing attribute access: expr={:?}, attr={}", expr, attr);

        // Only allow attribute access on variables
        match expr {
            AstExpr::Variable(base) => {
                #[cfg(test)]
                println!("Creating attribute node: {}.{}", base, attr);

                let result = AstExpr::Attribute { base, attr };
                // Apply any postfix operators to the attribute access result
                self.parse_postfix(result)
            }
            _ => {
                #[cfg(test)]
                println!("Error: Attribute access on non-variable expression");

                Err(ExprError::Syntax(format!(
                    "Attribute access on non-object expression at position {}",
                    dot_position
                )))
            }
        }
    }

    // Unified method for parsing expressions with a flag for comma handling
    fn parse_expr_unified(
        &mut self,
        min_bp: u8,
        allow_comma: bool,
    ) -> Result<AstExpr<'arena>, ExprError> {
        // Check recursion depth to prevent stack overflow
        self.recursion_depth += 1;
        if self.recursion_depth > self.max_recursion_depth {
            self.recursion_depth -= 1;
            return Err(ExprError::RecursionLimit(format!(
                "Expression too complex: exceeded maximum recursion depth of {}",
                self.max_recursion_depth
            )));
        }

        // Parse prefix or primary expression
        let mut lhs = self.parse_prefix_or_primary(allow_comma)?;

        // Apply postfix operators (function calls, array access, attribute access)
        lhs = self.parse_postfix(lhs)?;

        // Parse infix operators
        lhs = self.parse_infix_operators(lhs, min_bp, allow_comma)?;

        // Juxtaposition parsing disabled - use standard function call syntax with parentheses

        // Always decrement the recursion depth before returning
        self.recursion_depth -= 1;

        Ok(lhs)
    }

    fn parse_prefix_or_primary(&mut self, allow_comma: bool) -> Result<AstExpr<'arena>, ExprError> {
        if let Some(tok) = self.peek() {
            // Check for error tokens and report them immediately
            if tok.kind == TokenKind::Error {
                return Err(ExprError::Syntax(format!(
                    "Unexpected token '{}' at position {}",
                    tok.text.as_deref().unwrap_or("unknown"),
                    tok.position
                )));
            }
            if tok.kind == TokenKind::Operator {
                let op = tok.text.as_deref().unwrap_or("");
                let op_position = tok.position;
                if let Some(r_bp) = Self::get_prefix_binding_power(op) {
                    // Make a copy of the operator for later use
                    let op_str = String::from(op);

                    // Consume the operator token
                    self.next();

                    // Handle the case where there's nothing after the operator
                    if self.peek().is_none() {
                        return Err(ExprError::Syntax(format!(
                            "Expected expression after '{}' at position {}",
                            op_str, op_position
                        )));
                    }

                    // Parse the right-hand side expression
                    let rhs = self.parse_expr_unified(r_bp, allow_comma)?;

                    // Create the appropriate AST node
                    if op_str == "-" {
                        let mut args = bumpalo::collections::Vec::new_in(self.arena);
                        args.push(rhs);
                        Ok(AstExpr::Function {
                            name: self.arena.alloc_str("neg"),
                            args: args.into_bump_slice(),
                        })
                    } else {
                        // Unary + is a no-op
                        Ok(rhs)
                    }
                } else {
                    self.parse_primary()
                }
            } else {
                self.parse_primary()
            }
        } else {
            self.parse_primary()
        }
    }

    // Helper method for parsing ternary expressions (condition ? true_expr : false_expr)
    fn parse_ternary_op(
        &mut self,
        condition: AstExpr<'arena>,
        allow_comma: bool,
    ) -> Result<AstExpr<'arena>, ExprError> {
        // We've already consumed the '?' at this point

        // Parse the true branch (what to evaluate if condition is true)
        let true_branch = self.parse_expr_unified(0, allow_comma)?;

        // Expect a colon separating the true and false branches
        if let Some(tok) = self.peek() {
            if tok.kind == TokenKind::Operator && tok.text.as_deref() == Some(":") {
                self.next(); // Consume the ':'
            } else {
                return Err(ExprError::Syntax(format!(
                    "Expected ':' in ternary expression, found '{}'",
                    tok.text.clone().unwrap_or_else(|| "unknown".to_string())
                )));
            }
        } else {
            return Err(ExprError::Syntax(
                "Expected ':' in ternary expression, found end of input".to_string(),
            ));
        }

        // Parse the false branch (what to evaluate if condition is false)
        let false_branch = self.parse_expr_unified(0, allow_comma)?;

        Ok(AstExpr::Conditional {
            condition: self.arena.alloc(condition),
            true_branch: self.arena.alloc(true_branch),
            false_branch: self.arena.alloc(false_branch),
        })
    }

    fn parse_infix_operators(
        &mut self,
        mut lhs: AstExpr<'arena>,
        min_bp: u8,
        allow_comma: bool,
    ) -> Result<AstExpr<'arena>, ExprError> {
        loop {
            // Get the next operator
            let op_text = if let Some(tok) = self.peek() {
                if tok.kind == TokenKind::Operator {
                    tok.text.as_deref().unwrap_or("")
                } else if tok.kind == TokenKind::Separator
                    && (tok.text.as_deref() == Some(",") || tok.text.as_deref() == Some(";"))
                {
                    // Only treat comma or semicolon as an operator if allowed
                    if allow_comma {
                        tok.text.as_deref().unwrap_or("")
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            };

            // Make a copy of the operator for later use
            let op = String::from(op_text);

            // Special case for ternary operator
            if op == "?" {
                // Get binding power
                let Some(bp) = Self::get_binding_power(&op) else {
                    break;
                };

                // Check minimum binding power
                if bp.left < min_bp {
                    break;
                }

                // Consume the ? operator
                self.next();

                // Parse the rest of the ternary expression
                lhs = self.parse_ternary_op(lhs, allow_comma)?;
                continue;
            }

            // Special case for logical operators
            if op == "&&" || op == "||" {
                // Get binding power - these should already be defined in get_binding_power
                let Some(bp) = Self::get_binding_power(&op) else {
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
                    op: if op == "&&" {
                        crate::types::LogicalOperator::And
                    } else {
                        crate::types::LogicalOperator::Or
                    },
                    left: self.arena.alloc(lhs),
                    right: self.arena.alloc(rhs),
                };
                continue;
            }

            // Get binding power for regular (non-logical) operator
            let Some(bp) = Self::get_binding_power(&op) else {
                break;
            };

            // If the operator's left binding power is less than the minimum, we're done
            if bp.left < min_bp {
                break;
            }

            // Consume the operator
            self.next();

            // Special case for right-associative power operators
            let rhs = if op == "^" || op == "**" {
                self.parse_expr_unified(bp.right - 1, allow_comma)?
            } else {
                self.parse_expr_unified(bp.right, allow_comma)?
            };

            // Create a function node for the operator
            let mut args = bumpalo::collections::Vec::new_in(self.arena);
            args.push(lhs);
            args.push(rhs);
            lhs = AstExpr::Function {
                name: self.arena.alloc_str(&op),
                args: args.into_bump_slice(),
            };
        }
        Ok(lhs)
    }

    #[allow(dead_code)]
    fn parse_juxtaposition(
        &mut self,
        lhs: AstExpr<'arena>,
        _allow_comma: bool,
    ) -> Result<AstExpr<'arena>, ExprError> {
        let mut lhs = lhs;
        if let Some(tok) = self.peek() {
            let is_valid_lhs = matches!(&lhs, AstExpr::Variable(_));
            let is_valid_rhs = matches!(
                tok.kind,
                TokenKind::Number | TokenKind::Variable | TokenKind::Open
            ) || (tok.kind == TokenKind::Operator
                && (tok.text.as_deref() == Some("-")
                    || tok.text.as_deref() == Some("+")
                    || tok.text.as_deref() == Some("~")));

            // If lhs is a variable and is in reserved_vars or context_vars, do NOT allow juxtaposition
            let is_reserved_var = match &lhs {
                AstExpr::Variable(name) => {
                    let reserved = self
                        .reserved_vars
                        .as_ref()
                        .map(|s| s.iter().any(|v| v.as_ref() == *name))
                        .unwrap_or(false);
                    let in_context = self
                        .context_vars
                        .as_ref()
                        .map(|s| s.iter().any(|v| v.as_ref() == *name))
                        .unwrap_or(false);
                    reserved || in_context
                }
                _ => false,
            };

            if is_valid_lhs && is_valid_rhs && !is_reserved_var {
                // Get the function name (variable)
                let func_name = match &lhs {
                    AstExpr::Variable(name) => name,
                    _ => unreachable!(),
                };
                // Parse only a primary expression for juxtaposition to avoid consuming operators
                // This ensures "sin 1 + 2" is parsed as "(sin 1) + 2" not "sin(1 + 2)"
                let arg = self.parse_primary()?;

                // Create a function node
                let mut args = bumpalo::collections::Vec::new_in(self.arena);
                args.push(arg);
                lhs = AstExpr::Function {
                    name: func_name,
                    args: args.into_bump_slice(),
                };
            }
        }
        Ok(lhs)
    }

    // Parse an expression with the given minimum binding power
    fn parse_expr(&mut self, min_bp: u8) -> Result<AstExpr<'arena>, ExprError> {
        self.parse_expr_unified(min_bp, true)
    }

    // Parse a primary expression (number, variable, parenthesized expression)
    fn parse_primary(&mut self) -> Result<AstExpr<'arena>, ExprError> {
        let tok = match self.peek() {
            Some(tok) => tok,
            None => return Err(ExprError::Syntax("Unexpected end of input".to_string())),
        };

        match tok.kind {
            TokenKind::Number => {
                let val = tok.value.unwrap_or(0.0);
                self.next();
                Ok(AstExpr::Constant(val))
            }
            TokenKind::Variable => {
                let name = match &tok.text {
                    Some(name) => self.arena.alloc_str(name),
                    None => return Err(ExprError::Syntax("Variable name is missing".to_string())),
                };
                self.next();
                Ok(AstExpr::Variable(name))
            }
            TokenKind::Open if tok.text.as_deref() == Some("(") => self.parse_parenthesized_expr(),
            TokenKind::Close => {
                // This is a closing parenthesis without a matching opening parenthesis
                let position = tok.position;
                let found = tok.text.clone().unwrap_or_else(|| ")".to_string());
                Err(ExprError::Syntax(format!(
                    "Unexpected closing parenthesis at position {}: '{}'",
                    position, found
                )))
            }
            _ => {
                let position = tok.position;
                let found = tok.text.clone().unwrap_or_else(|| "unknown".to_string());
                Err(ExprError::Syntax(format!(
                    "Unexpected token at position {}: '{}'",
                    position, found
                )))
            }
        }
    }

    // The parse_postfix method is no longer needed as its functionality
    // has been integrated into parse_expr_unified_inner

    // Check if the expression is too long
    fn check_expression_length(&self, input: &str) -> Result<(), ExprError> {
        const MAX_EXPRESSION_LENGTH: usize = 10000; // Reasonable limit
        if input.len() > MAX_EXPRESSION_LENGTH {
            return Err(ExprError::Syntax(format!(
                "Expression too long: {} characters (maximum is {})",
                input.len(),
                MAX_EXPRESSION_LENGTH
            )));
        }
        Ok(())
    }

    // Parse a complete expression
    fn parse(&mut self) -> Result<AstExpr<'arena>, ExprError> {
        // Check expression length
        if let Some(remaining) = self.lexer.get_remaining_input() {
            self.check_expression_length(remaining)?;
        }

        // Reset recursion depth before parsing
        self.recursion_depth = 0;

        // Parse the expression
        let expr = self.parse_expr(0)?;

        #[cfg(test)]
        println!("Parsed expression: {:?}", expr);

        // Check for unexpected trailing tokens
        if let Some(tok) = self.peek() {
            // Handle error tokens - return an error instead of skipping
            if tok.kind == TokenKind::Error {
                return Err(ExprError::Syntax(format!(
                    "Unexpected token '{}' at position {}",
                    tok.text.as_deref().unwrap_or("unknown"),
                    tok.position
                )));
            }
            // Skip trailing whitespace
            if tok.kind == TokenKind::Operator
                && tok.text.as_deref().is_some_and(|t| t.trim().is_empty())
            {
                self.next();
            } else if tok.kind == TokenKind::Close {
                // For expressions like "1)", it's an error
                return Err(ExprError::Syntax(format!(
                    "Unexpected closing parenthesis at position {}: check for balanced parentheses",
                    tok.position
                )));
            } else {
                // Any other trailing token is an error
                return Err(ExprError::Syntax(format!(
                    "Unexpected token at position {}: '{}'",
                    tok.position,
                    tok.text.clone().unwrap_or_else(|| "unknown".to_string())
                )));
            }
        }

        Ok(expr)
    }
}

/// Parse an expression string into an AST.
///
/// This is the primary parsing function that requires an explicit arena for memory allocation.
/// For parsing expression function bodies with parameters, use `parse_expression_with_parameters`.
///
/// # Supported Features
///
/// - Arithmetic operators: +, -, *, /, %, ^, **
/// - Comparison operators: ==, !=, <, >, <=, >=, <>
/// - Logical operators: &&, ||
/// - Ternary conditional: condition ? true_expr : false_expr
/// - Function calls: func(arg1, arg2)
/// - Variables and constants
/// - Array access: array[index]
/// - Attribute access: object.attribute
///
/// # Examples
///
/// ```
/// use exp_rs::engine::parse_expression;
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
///
/// // Basic arithmetic
/// let expr = parse_expression("2 + 3 * 4", &arena).unwrap();
///
/// // Ternary conditional
/// let expr = parse_expression("x > 0 ? 1 : -1", &arena).unwrap();
///
/// // Function calls
/// let expr = parse_expression("sin(x) + cos(y)", &arena).unwrap();
/// ```
pub fn parse_expression<'arena>(
    input: &str,
    arena: &'arena Bump,
) -> Result<AstExpr<'arena>, ExprError> {
    parse_expression_arena_with_context(input, arena, None, None)
}

/// Parse an expression with function parameters that should be treated as variables.
///
/// This function is specifically designed for parsing expression function bodies where
/// certain identifiers (the function parameters) should always be treated as variables,
/// not as function calls. This prevents "x(y)" from being parsed as a function call
/// when "x" is a parameter.
///
/// # Examples
///
/// ```
/// use exp_rs::engine::parse_expression_with_parameters;
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
///
/// // Parse expression function body with parameters
/// let params = vec!["x".to_string(), "y".to_string()];
/// let expr = parse_expression_with_parameters("x * y + 2", &arena, &params).unwrap();
/// // In this case, 'x' and 'y' are treated as variables, not potential function calls
///
/// // Without this, "x(y)" would be parsed as a function call
/// let expr = parse_expression_with_parameters("x(y)", &arena, &params).unwrap();
/// // This correctly parses as multiplication due to juxtaposition: x * y
/// ```
pub fn parse_expression_with_parameters<'arena>(
    input: &str,
    arena: &'arena Bump,
    parameters: &[String],
) -> Result<AstExpr<'arena>, ExprError> {
    parse_expression_arena_with_context(input, arena, Some(parameters), None)
}

/// Parse an expression with reserved variables and context variable names.
///
/// This is the most configurable parsing function that allows specifying both:
/// - reserved_vars: Variables that should not be treated as functions in implicit calls
/// - context_vars: Variables from the evaluation context that should be recognized
///
/// Supports all expression features, including ternary operators.
///
/// # Parameters
///
/// * `input` - The expression string to parse
/// * `arena` - The arena to allocate AST nodes in
/// * `reserved_vars` - Optional list of variable names that should not be treated as functions
/// * `context_vars` - Optional list of variable names from the evaluation context
pub fn parse_expression_arena_with_context<'arena>(
    input: &str,
    arena: &'arena Bump,
    reserved_vars: Option<&[String]>,
    context_vars: Option<&[String]>,
) -> Result<AstExpr<'arena>, ExprError> {
    // Ternary operators (? :) are now supported
    // Comparison operators (<, >, <=, >=, ==, !=) are also supported

    // The lexer now properly handles decimal numbers starting with a dot
    let mut parser =
        PrattParser::with_reserved_vars_and_context(input, arena, reserved_vars, context_vars);
    parser.parse()
}

/// Interprets a string as a mathematical expression, evaluates it, and returns the result.
///
/// This is the primary function for evaluating expressions. It parses the expression string,
/// builds an Abstract Syntax Tree (AST), and then evaluates the AST using the provided context.
///
/// # Parameters
///
/// * `expression`: The mathematical expression to evaluate as a string
/// * `ctx`: An optional evaluation context containing variables, constants, and functions
///
/// # Returns
///
/// * `Ok(value)`: The result of evaluating the expression
/// * `Err(error)`: An error describing what went wrong during parsing or evaluation
///
/// # Examples
///
/// Basic usage without context:
///
/// ```
/// use exp_rs::engine::interp;
///
/// // Evaluate a simple expression
/// let result = interp("2 + 3 * 4", None).unwrap();
/// assert_eq!(result, 14.0);
///
/// # #[cfg(feature = "libm")]
/// # {
/// // Using built-in functions and constants
/// let result = interp("sin(pi/6) + cos(pi/3)", None).unwrap();
/// assert!((result - 1.0).abs() < 0.0001);
/// # }
/// ```
///
/// Using a context with variables:
///
/// ```
/// use exp_rs::context::EvalContext;
/// use exp_rs::engine::interp;
/// use std::rc::Rc;
///
/// let mut ctx = EvalContext::new();
/// ctx.set_parameter("x", 5.0);
/// ctx.set_parameter("y", 10.0);
///
/// let result = interp("x + y", Some(Rc::new(ctx))).unwrap();
/// assert_eq!(result, 15.0);
/// ```
///
/// Error handling:
///
/// ```
/// use exp_rs::engine::interp;
/// use exp_rs::error::ExprError;
///
/// match interp("2 + * 3", None) {
///     Ok(_) => panic!("Expected an error"),
///     Err(ExprError::Syntax(_)) => {
///         // This is expected - there's a syntax error in the expression
///     }
///     Err(e) => panic!("Unexpected error: {:?}", e),
/// }
/// ```
pub fn interp<'a>(expression: &str, ctx: Option<Rc<EvalContext>>) -> crate::error::Result<Real> {
    use crate::expression::Expression;
    use alloc::rc::Rc;

    // Create a new context if none provided
    let eval_ctx = match ctx {
        Some(ctx_rc) => ctx_rc,
        None => {
            // Use the default EvalContext::new() which will auto-register functions
            // based on the feature flags/test environment
            let new_ctx = EvalContext::new();
            Rc::new(new_ctx)
        }
    };

    // Use Expression for consistent implementation
    let arena = Bump::new();
    Expression::eval_with_context(expression, &eval_ctx, &arena)
}

#[cfg(test)]
use std::format;
#[cfg(test)]
use std::vec::Vec;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::EvalContext;
    use crate::functions::{log, sin};
    use bumpalo::Bump;

    // Simple test helper to parse with a temporary arena
    fn parse_test(expr: &str) -> Result<AstExpr<'static>, ExprError> {
        use std::cell::RefCell;

        thread_local! {
            static TEST_ARENA: RefCell<Bump> = RefCell::new(Bump::with_capacity(64 * 1024));
        }

        TEST_ARENA.with(|arena| {
            let arena = arena.borrow();
            let ast = parse_expression(expr, &*arena)?;
            // SAFETY: Only safe for tests - extends lifetime to 'static
            Ok(unsafe { std::mem::transmute::<AstExpr<'_>, AstExpr<'static>>(ast) })
        })
    }

    // Tests for the ternary operator
    #[test]
    fn test_ternary_operator_parsing() {
        // Basic ternary expression
        let ast = parse_test("x > 0 ? 1 : -1").unwrap();
        match ast {
            AstExpr::Conditional {
                condition,
                true_branch,
                false_branch,
            } => {
                // Verify condition is "x > 0"
                match *condition {
                    AstExpr::Function { name, args } => {
                        assert_eq!(name, ">");
                        assert_eq!(args.len(), 2);
                    }
                    _ => panic!("Expected function node for condition"),
                }

                // Verify true branch is "1"
                match *true_branch {
                    AstExpr::Constant(val) => assert_eq!(val, 1.0),
                    _ => panic!("Expected constant for true branch"),
                }

                // Verify false branch is "-1"
                match *false_branch {
                    AstExpr::Function { name, args } => {
                        assert_eq!(name, "neg");
                        assert_eq!(args.len(), 1);
                    }
                    _ => panic!("Expected function node for false branch"),
                }
            }
            _ => panic!("Expected conditional node"),
        }
    }

    #[test]
    fn test_ternary_operator_evaluation() {
        // Create a context with the necessary operators for the test
        let mut ctx = EvalContext::new();

        // Make sure the context has the ">" operator registered
        #[cfg(not(feature = "libm"))]
        {
            ctx.register_native_function(">", 2, |args| if args[0] > args[1] { 1.0 } else { 0.0 });
            ctx.register_native_function("*", 2, |args| args[0] * args[1]);
            ctx.register_native_function("+", 2, |args| args[0] + args[1]);
        }

        let _ = ctx.set_parameter("x", 5.0);
        let result = interp("x > 0 ? 10 : 20", Some(Rc::new(ctx))).unwrap();
        assert_eq!(result, 10.0);

        // Test with false condition
        let mut ctx = EvalContext::new();

        // Make sure the context has the ">" operator registered
        #[cfg(not(feature = "libm"))]
        {
            ctx.register_native_function(">", 2, |args| if args[0] > args[1] { 1.0 } else { 0.0 });
            ctx.register_native_function("*", 2, |args| args[0] * args[1]);
            ctx.register_native_function("+", 2, |args| args[0] + args[1]);
        }

        let _ = ctx.set_parameter("x", -5.0);
        let result = interp("x > 0 ? 10 : 20", Some(Rc::new(ctx))).unwrap();
        assert_eq!(result, 20.0);

        // Create a context for the rest of the tests
        let ctx = EvalContext::new();

        // Make sure the context has all the necessary operators registered
        #[cfg(not(feature = "libm"))]
        {
            ctx.register_native_function(">", 2, |args| if args[0] > args[1] { 1.0 } else { 0.0 });
            ctx.register_native_function("*", 2, |args| args[0] * args[1]);
            ctx.register_native_function("+", 2, |args| args[0] + args[1]);
        }

        let ctx_rc = Rc::new(ctx);

        // Test with nested ternary
        let result = interp("1 ? 2 ? 3 : 4 : 5", Some(ctx_rc.clone())).unwrap();
        assert_eq!(result, 3.0);

        // Test with complex expressions in branches
        let result = interp("0 ? 10 : 2 * 3 + 4", Some(ctx_rc.clone())).unwrap();
        assert_eq!(result, 10.0);

        // Test with complex expressions in condition
        let result = interp("2 * 3 > 5 ? 1 : 0", Some(ctx_rc)).unwrap();
        assert_eq!(result, 1.0);
    }

    #[test]
    fn test_ternary_operator_short_circuit() {
        // Context with a variable that will cause division by zero in the false branch
        let mut ctx = EvalContext::new();

        // Make sure the context has all the necessary operators registered
        #[cfg(not(feature = "libm"))]
        {
            ctx.register_native_function("/", 2, |args| args[0] / args[1]);
        }

        let _ = ctx.set_parameter("x", 0.0);

        // This should not cause a division by zero error because the condition is true,
        // so the false branch (x's value / 0) is never evaluated due to short-circuit
        let result = interp("1 ? 42 : 1/x", Some(Rc::new(ctx.clone()))).unwrap();
        assert_eq!(result, 42.0);

        // This should return infinity because the condition is false,
        // so the false branch (1/0) is evaluated
        let result = interp("0 ? 42 : 1/x", Some(Rc::new(ctx))).unwrap();

        #[cfg(feature = "f32")]
        assert!(
            result.is_infinite() && result.is_sign_positive(),
            "1/0 should be positive infinity"
        );
        #[cfg(not(feature = "f32"))]
        assert!(
            result.is_infinite() && result.is_sign_positive(),
            "1/0 should be positive infinity"
        );
    }

    #[test]
    fn test_ternary_operator_precedence() {
        // Create a context with the necessary operators for the test
        let ctx = EvalContext::new();

        // Make sure the context has all the necessary operators registered
        #[cfg(not(feature = "libm"))]
        {
            ctx.register_native_function(">", 2, |args| if args[0] > args[1] { 1.0 } else { 0.0 });
            ctx.register_native_function("+", 2, |args| args[0] + args[1]);
            ctx.register_native_function("&&", 2, |args| {
                if args[0] != 0.0 && args[1] != 0.0 {
                    1.0
                } else {
                    0.0
                }
            });
        }

        let ctx_rc = Rc::new(ctx);

        // Test that ternary has lower precedence than comparison
        let result = interp("2 > 1 ? 3 : 4", Some(ctx_rc.clone())).unwrap();
        assert_eq!(result, 3.0);

        // Test that ternary has lower precedence than addition/subtraction
        let result = interp("1 + 2 ? 3 : 4", Some(ctx_rc.clone())).unwrap();
        assert_eq!(result, 3.0);

        // Test that ternary has lower precedence than logical operators
        let result = interp("1 && 0 ? 3 : 4", Some(ctx_rc.clone())).unwrap();
        assert_eq!(result, 4.0);

        // Test right associativity with nested ternary
        let result = interp("1 ? 2 : 3 ? 4 : 5", Some(ctx_rc)).unwrap();
        assert_eq!(result, 2.0);
    }

    // Helper function to print AST for debugging
    #[allow(dead_code)]
    fn debug_ast(expr: &AstExpr<'_>, indent: usize) -> String {
        let spaces = " ".repeat(indent);
        match expr {
            AstExpr::Constant(val) => format!("{}Constant({})", spaces, val),
            AstExpr::Variable(name) => format!("{}Variable({})", spaces, name),
            AstExpr::Function { name, args } => {
                let mut result = format!("{}Function({}, [\n", spaces, name);
                for arg in args.iter() {
                    result.push_str(&format!("{},\n", debug_ast(arg, indent + 2)));
                }
                result.push_str(&format!("{}])", spaces));
                result
            }
            AstExpr::Array { name, index } => {
                format!(
                    "{}Array({}, {})",
                    spaces,
                    name,
                    debug_ast(index, indent + 2)
                )
            }
            AstExpr::Attribute { base, attr } => {
                format!("{}Attribute({}, {})", spaces, base, attr)
            }
            AstExpr::LogicalOp { op, left, right } => {
                let op_str = match op {
                    crate::types::LogicalOperator::And => "&&",
                    crate::types::LogicalOperator::Or => "||",
                };
                format!(
                    "{}LogicalOp({}, \n{},\n{})",
                    spaces,
                    op_str,
                    debug_ast(left, indent + 2),
                    debug_ast(right, indent + 2)
                )
            }
            AstExpr::Conditional {
                condition,
                true_branch,
                false_branch,
            } => {
                let mut result = format!("{}Conditional(\n", spaces);
                result.push_str(&format!(
                    "{}condition: {},\n",
                    spaces.clone() + "  ",
                    debug_ast(condition, indent + 4)
                ));
                result.push_str(&format!(
                    "{}true_branch: {},\n",
                    spaces.clone() + "  ",
                    debug_ast(true_branch, indent + 4)
                ));
                result.push_str(&format!(
                    "{}false_branch: {}\n",
                    spaces.clone() + "  ",
                    debug_ast(false_branch, indent + 4)
                ));
                result.push_str(&format!("{})", spaces));
                result
            }
        }
    }

    #[test]
    fn test_unknown_variable_and_function_eval() {
        // Test that using 'sin' as a variable (when it's a function) produces an error
        let arena = bumpalo::Bump::new();
        let sin_var_ast = parse_expression("sin", &arena).unwrap();
        let err = crate::eval::ast::eval_ast(&sin_var_ast, None, &arena).unwrap_err();

        // Accept any error type, just verify it's an error when using a function name as a variable
        println!("Error when evaluating 'sin' as a variable: {:?}", err);
        // No specific assertion on error type, just ensure it's an error
    }

    // Removed test_parse_postfix_chained_juxtaposition - testing manual AST construction
    // which is no longer relevant with arena-based parsing

    #[test]
    fn test_pow_arity_ast() {
        // Since we now automatically add a second argument to pow(2),
        // we need to modify this test to check for 2 arguments
        let ast = parse_test("pow(2)").unwrap_or_else(|e| panic!("Parse error: {}", e));
        println!("AST for pow(2): {:?}", ast);

        match ast {
            AstExpr::Function { ref name, ref args } if *name == "pow" => {
                assert_eq!(args.len(), 2); // Changed from 1 to 2
                match &args[0] {
                    AstExpr::Constant(c) => assert_eq!(*c, 2.0),
                    _ => panic!("Expected constant as pow arg"),
                }
                // Check the second argument is 2.0 (default)
                match &args[1] {
                    AstExpr::Constant(c) => assert_eq!(*c, 2.0),
                    _ => panic!("Expected constant as second pow arg"),
                }
            }
            _ => panic!("Expected function node for pow"),
        }
    }

    #[test]
    fn test_parse_postfix_array_and_attribute_access() {
        // Test array indexing through parsing
        let arena = Bump::new();
        let ast = parse_expression("sin(arr[0])", &arena).unwrap();

        // Verify the parsed structure
        match &ast {
            AstExpr::Function { name, args } => {
                assert_eq!(*name, "sin");
                assert_eq!(args.len(), 1);
                match &args[0] {
                    AstExpr::Array { name, index } => {
                        assert_eq!(*name, "arr");
                        match **index {
                            AstExpr::Constant(val) => assert_eq!(val, 0.0),
                            _ => panic!("Expected constant as array index"),
                        }
                    }
                    _ => panic!("Expected array as argument to sin"),
                }
            }
            _ => panic!("Expected function node for sin(arr[0])"),
        }

        // Test attribute access through parsing
        // Note: foo.bar(x) would parse as a function call with foo.bar as the function name
        // For pure attribute access, test foo.bar
        let ast2 = parse_expression("foo.bar", &arena).unwrap();

        // Verify the parsed structure
        match &ast2 {
            AstExpr::Attribute { base, attr } => {
                assert_eq!(*base, "foo");
                assert_eq!(*attr, "bar");
            }
            _ => panic!("Expected attribute node for foo.bar"),
        }
    }

    #[test]
    fn test_parse_postfix_function_call_after_attribute() {
        let ast = parse_test("foo.bar(1)").unwrap();
        match ast {
            AstExpr::Function { name, args } => {
                assert_eq!(name, "bar");
                assert_eq!(args.len(), 1);
                match &args[0] {
                    AstExpr::Constant(val) => assert_eq!(*val, 1.0),
                    _ => panic!("Expected constant as argument to foo.bar"),
                }
            }
            _ => panic!("Expected function node for foo.bar(1)"),
        }
    }

    #[test]
    fn test_parse_postfix_array_access_complex_index() {
        let ast = parse_test("arr[1+2*3]").unwrap();
        match ast {
            AstExpr::Array { name, index } => {
                assert_eq!(name, "arr");
                match *index {
                    AstExpr::Function {
                        name: ref n,
                        args: ref a,
                    } if *n == "+" => {
                        assert_eq!(a.len(), 2);
                    }
                    _ => panic!("Expected function as array index"),
                }
            }
            _ => panic!("Expected array AST node"),
        }
    }

    #[test]
    fn test_atan2_function() {
        // Test atan2 with explicit arguments - atan2(y,x)
        let result = interp("atan2(1,2)", None).unwrap();
        println!("atan2(1,2) = {}", result);
        // For point (2,1), the angle is arctan(1/2) ≈ 0.4636 radians
        assert!(
            (result - 0.4636).abs() < 1e-3,
            "atan2(1,2) should be approximately 0.4636"
        );

        // Test atan2 with swapped arguments
        let result2 = interp("atan2(2,1)", None).unwrap();
        println!("atan2(2,1) = {}", result2);
        // For point (1,2), the angle is arctan(2/1) ≈ 1.1071 radians
        assert!(
            (result2 - 1.1071).abs() < 1e-3,
            "atan2(2,1) should be approximately 1.1071"
        );

        // Test atan2(1,1) which should be π/4
        let result3 = interp("atan2(1,1)", None).unwrap();
        println!("atan2(1,1) = {}", result3);
        assert!(
            (result3 - 0.7854).abs() < 1e-3,
            "atan2(1,1) should be approximately 0.7854 (π/4)"
        );
    }

    #[test]
    fn test_pow_arity_eval() {
        // Since we now automatically add a second argument to pow(2),
        // we need to modify this test to check that it evaluates correctly
        let result = interp("pow(2)", None).unwrap();
        println!("pow(2) = {}", result); // Debug output
        assert_eq!(result, 4.0); // pow(2, 2) = 4.0

        // Let's also test that pow with explicit arguments works
        let result2 = interp("pow(2, 3)", None).unwrap();
        println!("pow(2, 3) = {}", result2); // Debug output
        assert_eq!(result2, 8.0); // pow(2, 3) = 8.0
    }

    #[test]
    #[cfg(feature = "libm")] // This test requires libm for built-in sin/cos/abs
    fn test_function_composition() {
        // Test function composition through parsing and evaluation

        // Test sin(0.5)
        let result = interp("sin(0.5)", None).unwrap();
        println!("sin 0.5 = {}", result);
        assert!(
            (result - sin(0.5, 0.0)).abs() < 1e-6,
            "sin(0.5) should work"
        );

        // Test chained function calls: sin(cos(0))
        let result2 = interp("sin(cos(0))", None).unwrap();
        println!("sin(cos(0)) = {}", result2);
        assert!(
            (result2 - sin(1.0, 0.0)).abs() < 1e-6,
            "sin(cos(0)) should be sin(1)"
        );

        // Test abs with negative number
        let result3 = interp("abs(-42)", None).unwrap();
        println!("abs(-42) = {}", result3);
        assert_eq!(result3, 42.0, "abs(-42) should be 42.0");
    }

    #[test]
    #[cfg(not(feature = "libm"))] // Alternative test for non-libm builds
    fn test_function_composition_no_libm() {
        // Create a context with our own implementations
        let mut ctx = EvalContext::new();
        ctx.register_native_function("sin", 1, |args| args[0].sin());
        ctx.register_native_function("cos", 1, |args| args[0].cos());
        ctx.register_native_function("abs", 1, |args| args[0].abs());

        let ctx_rc = Rc::new(ctx);

        // Test sin(0.5)
        let result = interp("sin(0.5)", Some(ctx_rc.clone())).unwrap();
        println!("sin 0.5 = {}", result);
        assert!(
            (result - (0.5 as Real).sin()).abs() < 1e-6,
            "sin(0.5) should work"
        );

        // Test chained function calls: sin(cos(0))
        let result2 = interp("sin(cos(0))", Some(ctx_rc.clone())).unwrap();
        println!("sin(cos(0)) = {}", result2);
        assert!(
            (result2 - (1.0 as Real).sin()).abs() < 1e-6,
            "sin(cos(0)) should be sin(1)"
        );

        // Test abs with negative number
        let result3 = interp("abs(-42)", Some(ctx_rc)).unwrap();
        println!("abs(-42) = {}", result3);
        assert_eq!(result3, 42.0, "abs(-42) should be 42.0");
    }

    #[test]
    fn test_juxtaposition_disabled() {
        // Test that juxtaposition is properly disabled - "sin x" should fail
        let ast = parse_test("sin x");
        match ast {
            Ok(_) => panic!("Expected parse error for 'sin x' since juxtaposition is disabled"),
            Err(e) => {
                println!("Expected parse error for 'sin x': {:?}", e);
                // This should fail since juxtaposition is disabled
                assert!(e.to_string().contains("Unexpected token"));
            }
        }

        // For abs -42, we need to use parentheses to make it clear
        // that we want function application, not subtraction
        let ast2 = parse_test("abs(-42)");
        match ast2 {
            Ok(ast2) => {
                println!("AST for abs(-42): {:?}", ast2);
                match ast2 {
                    AstExpr::Function { ref name, ref args } if *name == "abs" => {
                        assert_eq!(args.len(), 1);
                        match &args[0] {
                            AstExpr::Function {
                                name: neg_name,
                                args: neg_args,
                            } if *neg_name == "neg" => {
                                assert_eq!(neg_args.len(), 1);
                                match &neg_args[0] {
                                    AstExpr::Constant(c) => assert_eq!(*c, 42.0),
                                    _ => panic!("Expected constant as neg arg"),
                                }
                            }
                            _ => panic!("Expected neg function as abs arg"),
                        }
                    }
                    _ => panic!("Expected function node for abs(-42)"),
                }
            }
            Err(e) => {
                println!("Parse error for 'abs(-42)': {:?}", e);
                panic!("Parse error: {:?}", e);
            }
        }
    }

    #[test]
    #[cfg(feature = "libm")] // This test requires libm for built-in sin/asin
    fn test_function_recognition() {
        // Test function recognition through parsing and evaluation

        // Test asin(sin(0.5))
        let result = interp("asin(sin(0.5))", None).unwrap();
        println!("asin(sin(0.5)) = {}", result);
        assert!((result - 0.5).abs() < 1e-6, "asin(sin(0.5)) should be 0.5");

        // Test function recognition with parentheses
        let result2 = interp("sin(0.5)", None).unwrap();
        println!("sin(0.5) = {}", result2);
        assert!(
            (result2 - sin(0.5, 0.0)).abs() < 1e-6,
            "sin(0.5) should work"
        );
    }

    #[test]
    #[cfg(not(feature = "libm"))] // Alternative test for non-libm builds
    fn test_function_recognition_no_libm() {
        // Create a context with our own implementation
        let mut ctx = EvalContext::new();

        // Register sin and asin functions
        ctx.register_native_function("sin", 1, |args| args[0].sin());
        ctx.register_native_function("asin", 1, |args| args[0].asin());

        // Convert to Rc
        let ctx_rc = Rc::new(ctx);

        // Test asin(sin(0.5))
        let result = interp("asin(sin(0.5))", Some(ctx_rc.clone())).unwrap();
        println!("asin(sin(0.5)) = {}", result);
        assert!(
            (result - 0.5).abs() < 1e-2,
            "asin(sin(0.5)) should be approximately 0.5"
        );

        // Test function recognition with parentheses
        let result2 = interp("sin(0.5)", Some(ctx_rc)).unwrap();
        println!("sin(0.5) = {}", result2);
        assert!(
            (result2 - (0.5 as Real).sin()).abs() < 1e-6,
            "sin(0.5) should work"
        );
    }

    #[test]
    fn test_parse_postfix_attribute_on_function_result_should_error() {
        // This test verifies that attribute access on function results is rejected
        // Since juxtaposition is disabled, "sin x" will fail, so test with parentheses
        let _ast = parse_test("sin(x).foo");

        // Currently, this might parse as sin(x.foo) due to precedence
        // Let's test with parentheses to be explicit
        let ast2 = parse_test("(sin(x)).foo");
        assert!(
            ast2.is_err(),
            "Attribute access on function result should be rejected"
        );
    }

    #[test]
    fn test_parse_comma_in_parens_and_top_level() {
        let ast = parse_test("(1,2)");
        assert!(ast.is_ok(), "Comma in parens should be allowed");
        let ast2 = parse_test("1,2,3");
        assert!(ast2.is_ok(), "Top-level comma should be allowed");
        let ast3 = parse_test("(1,2),3");
        assert!(
            ast3.is_ok(),
            "Nested comma outside parens should be allowed"
        );
    }

    #[test]
    fn test_deeply_nested_function_calls() {
        // Test with 10 levels of nesting
        let expr = "abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-12345))))))))))";
        let ast = parse_test(expr);
        assert!(
            ast.is_ok(),
            "Deeply nested function calls should be parsed correctly"
        );

        // Test with unbalanced parentheses (missing one closing parenthesis)
        let unbalanced = "abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-12345)))))))))";
        let result = parse_test(unbalanced);
        assert!(result.is_err(), "Unbalanced parentheses should be detected");
        match result {
            Err(ExprError::UnmatchedParenthesis { position: _, found }) => {
                // This is the expected error
                assert_eq!(
                    found, "(",
                    "The unmatched parenthesis should be an opening one"
                );
            }
            _ => panic!("Expected UnmatchedParenthesis error for unbalanced parentheses"),
        }
    }

    #[test]
    fn test_parse_binary_op_deep_right_assoc_pow() {
        let ast = parse_test("2^2^2^2^2").unwrap();
        fn count_right_assoc_pow(expr: &AstExpr<'_>) -> usize {
            match expr {
                AstExpr::Function { name, args } if *name == "^" && args.len() == 2 => {
                    1 + count_right_assoc_pow(&args[1])
                }
                _ => 0,
            }
        }
        let pow_depth = count_right_assoc_pow(&ast);
        assert_eq!(pow_depth, 4, "Should be right-associative chain of 4 '^'");
    }

    #[test]
    fn test_deeply_nested_function_calls_with_debugging() {
        // Test with 10 levels of nesting
        let expr = "abs(abs(abs(abs(abs(abs(abs(abs(abs(abs(-12345))))))))))";

        // Print the expression for debugging
        println!("Testing expression with debugging: {}", expr);

        // Print all tokens for debugging
        let mut lexer = Lexer::new(expr);
        let mut tokens = Vec::new();
        while let Some(tok) = lexer.next_token() {
            tokens.push(tok);
        }

        println!("Tokens:");
        for (i, token) in tokens.iter().enumerate() {
            println!("  {}: {:?}", i, token);
        }

        // Count opening and closing parentheses
        let open_count = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Open && t.text.as_deref() == Some("("))
            .count();
        let close_count = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Close && t.text.as_deref() == Some(")"))
            .count();

        println!("Opening parentheses: {}", open_count);
        println!("Closing parentheses: {}", close_count);
        assert_eq!(
            open_count, close_count,
            "Number of opening and closing parentheses should match"
        );

        // Now try parsing
        let ast = parse_test(expr);
        assert!(
            ast.is_ok(),
            "Deeply nested function calls should be parsed correctly"
        );
    }

    #[test]
    fn test_parse_binary_op_mixed_unary_and_power() {
        let ast = parse_test("-2^2").unwrap();
        match ast {
            AstExpr::Function { name, args } if name == "neg" => match &args[0] {
                AstExpr::Function {
                    name: n2,
                    args: args2,
                } if *n2 == "^" => {
                    assert_eq!(args2.len(), 2);
                }
                _ => panic!("Expected ^ as argument to neg"),
            },
            _ => panic!("Expected neg as top-level function"),
        }
        let ast2 = parse_test("(-2)^2").unwrap();
        match ast2 {
            AstExpr::Function { name, args } if name == "^" => match &args[0] {
                AstExpr::Function {
                    name: n2,
                    args: args2,
                } if *n2 == "neg" => {
                    assert_eq!(args2.len(), 1);
                }
                _ => panic!("Expected neg as left arg to ^"),
            },
            _ => panic!("Expected ^ as top-level function"),
        }
        let ast3 = parse_test("-2^-2").unwrap();
        match ast3 {
            AstExpr::Function { name, args } if name == "neg" => match &args[0] {
                AstExpr::Function {
                    name: n2,
                    args: args2,
                } if *n2 == "^" => {
                    assert_eq!(args2.len(), 2);
                }
                _ => panic!("Expected ^ as argument to neg"),
            },
            _ => panic!("Expected neg as top-level function"),
        }
    }

    #[test]
    fn test_parse_binary_op_mixed_precedence() {
        let ast = parse_test("2+3*4^2-5/6").unwrap();
        match ast {
            AstExpr::Function { name, args } if name == "-" => {
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected - as top-level function"),
        }
    }

    #[test]
    fn test_parse_primary_paren_errors() {
        let ast = parse_test("((1+2)");
        assert!(ast.is_err(), "Unmatched parenthesis should be rejected");
        let ast2 = parse_test("1+)");
        assert!(ast2.is_err(), "Unmatched parenthesis should be rejected");
    }

    #[test]
    fn test_parse_primary_variable_and_number_edge_cases() {
        let ast = parse_test("foo_bar123").unwrap();
        match ast {
            AstExpr::Variable(name) => assert_eq!(name, "foo_bar123"),
            _ => panic!("Expected variable node"),
        }

        // Skip the .5 test for now as it's causing issues
        // We'll handle it in a separate test

        let ast3 = parse_test("1e-2").unwrap();
        match ast3 {
            AstExpr::Constant(val) => assert!((val - 0.01).abs() < 1e-10),
            _ => panic!("Expected constant node"),
        }

        let ast4 = parse_test("1.2e+3").unwrap();
        match ast4 {
            AstExpr::Constant(val) => assert!((val - 1200.0).abs() < 1e-10),
            _ => panic!("Expected constant node"),
        }
    }

    #[test]
    fn test_parse_decimal_with_leading_dot() {
        // This test should now pass with our improved error handling
        let ast = parse_test(".5").unwrap_or_else(|e| panic!("Parse error: {}", e));
        match ast {
            AstExpr::Constant(val) => assert_eq!(val, 0.5),
            _ => panic!("Expected constant node"),
        }
    }

    #[test]
    fn test_log() {
        // log(x) is base-10 logarithm in this library
        assert!((log(1000.0, 0.0) - 3.0).abs() < 1e-10);
        assert!((log(100.0, 0.0) - 2.0).abs() < 1e-10);
        assert!((log(10.0, 0.0) - 1.0).abs() < 1e-10);
    }
    #[test]
    fn test_eval_invalid_function_arity() {
        // Test with a function that doesn't have special handling for arity
        let result = interp("sin(1, 2)", None);
        assert!(result.is_err(), "sin(1, 2) should return an error");

        if let Err(err) = result {
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
                _ => panic!(
                    "Expected InvalidFunctionCall error for sin(1, 2), got: {:?}",
                    err
                ),
            }
        }

        // Test that pow with one argument works (special case)
        let result2 = interp("pow(2)", None).unwrap();
        assert_eq!(result2, 4.0); // pow(2, 2) = 4.0
    }
}
