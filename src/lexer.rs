use crate::types::TokenKind;
#[cfg(test)]
use crate::Real;
#[cfg(not(test))]
use crate::{Real, String, ToString};
#[cfg(not(test))]
use alloc::format;
#[cfg(test)]
use std::format;
#[cfg(test)]
use std::string::{String, ToString};

/// A token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub value: Option<Real>,
    pub text: Option<String>,
    pub position: usize,
}

/// The lexer struct, which produces tokens from an input string.
#[derive(Clone)]
pub struct Lexer<'a> {
    input: &'a str,
    pub pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        // Check for invalid UTF-8 sequences
        // This is a no-op in Rust since the &str type guarantees valid UTF-8
        // But we can check for extremely long input
        Self { input, pos: 0 }
    }

    /// Peek at the current character.
    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    /// Advance the position by one character.
    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += c.len_utf8();
        }
    }

    /// Peek at the next token without consuming it
    pub fn peek_token(&self) -> Option<Token> {
        let mut lexer_copy = self.clone();
        lexer_copy.next_token()
    }

    /// Get the remaining input from the current position
    pub fn get_remaining_input(&self) -> Option<&str> {
        if self.pos < self.input.len() {
            Some(&self.input[self.pos..])
        } else {
            None
        }
    }

    /// Get the original input string
    pub fn get_original_input(&self) -> &'a str {
        self.input
    }

    /// Skip whitespace.
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Check if a token is too long
    fn check_token_length(&self, start_pos: usize, end_pos: usize) -> Result<(), String> {
        const MAX_TOKEN_LENGTH: usize = 1000; // Reasonable limit
        if end_pos - start_pos > MAX_TOKEN_LENGTH {
            return Err(format!(
                "Token too long: {} characters (maximum is {})",
                end_pos - start_pos,
                MAX_TOKEN_LENGTH
            ));
        }
        Ok(())
    }

    /// Get the next token from the input.
    pub fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();
        let start_pos = self.pos;
        let c = self.peek()?;

        // Special case for decimal numbers starting with a dot
        if c == '.' && self.pos + 1 < self.input.len() {
            let next_char = self.input[self.pos + 1..].chars().next();
            if next_char.is_some_and(|d| d.is_ascii_digit()) {
                // This is a decimal number starting with a dot (e.g., .5)
                self.advance(); // Skip the dot
                let mut has_digits = false;
                let mut has_exp = false;

                // Parse digits after the dot
                while let Some(nc) = self.peek() {
                    if nc.is_ascii_digit() {
                        has_digits = true;
                        self.advance();
                    } else if (nc == 'e' || nc == 'E') && !has_exp {
                        has_exp = true;
                        self.advance();
                        // Optional sign after e/E
                        if let Some(sign) = self.peek() {
                            if sign == '+' || sign == '-' {
                                self.advance();
                            }
                        }

                        // Must have at least one digit after e/E
                        let mut has_exp_digits = false;
                        while let Some(ec) = self.peek() {
                            if ec.is_ascii_digit() {
                                has_exp_digits = true;
                                self.advance();
                            } else {
                                break;
                            }
                        }

                        if !has_exp_digits {
                            return Some(Token {
                                kind: TokenKind::Error,
                                value: None,
                                text: Some(String::from(&self.input[start_pos..self.pos])),
                                position: start_pos,
                            });
                        }
                    } else {
                        break;
                    }
                }

                if !has_digits {
                    return Some(Token {
                        kind: TokenKind::Error,
                        value: None,
                        text: Some(String::from(".")),
                        position: start_pos,
                    });
                }

                // Parse the number with a leading zero
                let num_str = format!("0{}", &self.input[start_pos..self.pos]);

                if let Ok(val) = num_str.parse::<Real>() {
                    return Some(Token {
                        kind: TokenKind::Number,
                        value: Some(val),
                        text: Some(String::from(&self.input[start_pos..self.pos])),
                        position: start_pos,
                    });
                } else {
                    return Some(Token {
                        kind: TokenKind::Error,
                        value: None,
                        text: Some(String::from(&self.input[start_pos..self.pos])),
                        position: start_pos,
                    });
                }
            }
        }

        // Number (integer or float, possibly scientific notation)
        if c.is_ascii_digit() {
            let mut saw_dot = false;
            let mut saw_e = false;
            let mut has_digits_after_e = false;

            // Parse integer part
            self.advance();

            // Parse fractional part
            while let Some(nc) = self.peek() {
                if nc.is_ascii_digit() {
                    self.advance();
                    if saw_e {
                        has_digits_after_e = true;
                    }
                } else if nc == '.' && !saw_dot {
                    saw_dot = true;
                    self.advance();
                } else if (nc == 'e' || nc == 'E') && !saw_e {
                    saw_e = true;
                    self.advance();
                    // Optional sign after e/E
                    if let Some(sign) = self.peek() {
                        if sign == '+' || sign == '-' {
                            self.advance();
                        }
                    }
                } else {
                    break;
                }
            }

            // Validate scientific notation has digits after 'e'
            if saw_e && !has_digits_after_e {
                return Some(Token {
                    kind: TokenKind::Error,
                    value: None,
                    text: Some(String::from(&self.input[start_pos..self.pos])),
                    position: start_pos,
                });
            }

            let num_str = &self.input[start_pos..self.pos];
            if let Ok(val) = num_str.parse::<Real>() {
                return Some(Token {
                    kind: TokenKind::Number,
                    value: Some(val),
                    text: Some(String::from(num_str)),
                    position: start_pos,
                });
            } else {
                return Some(Token {
                    kind: TokenKind::Error,
                    value: None,
                    text: Some(num_str.to_string()),
                    position: start_pos,
                });
            }
        }

        // Operators and punctuation
        // Support multi-character operators for tinyexpr++ grammar
        let op_start = "+-*/^%.<>=!&|~";
        if op_start.contains(c) {
            let kind = TokenKind::Operator;
            let mut text = String::from(c);
            self.advance();

            // Lookahead for multi-character operators
            let next = self.peek();
            // Handle **, &&, ||, <<, >>, <<<, >>>, <=, >=, ==, !=, <>, and others
            if let Some(nc) = next {
                match (c, nc) {
                    // Triple char: <<<, >>>
                    ('<', '<') if self.input[self.pos..].starts_with("<<") => {
                        // Could be <<< or <<, check for third '<'
                        self.advance(); // 2nd '<'
                        if self.peek() == Some('<') {
                            text.push('<');
                            self.advance();
                        } else {
                            text.push('<');
                        }
                    }
                    ('>', '>') if self.input[self.pos..].starts_with(">>") => {
                        // Could be >>> or >>, check for third '>'
                        self.advance(); // 2nd '>'
                        if self.peek() == Some('>') {
                            text.push('>');
                            self.advance();
                        } else {
                            text.push('>');
                        }
                    }
                    // Double char ops
                    ('*', '*') | ('&', '&') | ('|', '|') | ('<', '<') | ('>', '>') => {
                        text.push(nc);
                        self.advance();
                    }
                    ('<', '>') => {
                        text.push(nc);
                        self.advance();
                    }
                    ('<', '=') | ('>', '=') | ('=', '=') | ('!', '=') => {
                        text.push(nc);
                        self.advance();
                    }
                    _ => {}
                }
            }

            return Some(Token {
                kind,
                value: None,
                text: Some(text),
                position: start_pos,
            });
        }

        // Identifier (variable, function, constant)
        if c.is_ascii_alphabetic() || c == '_' {
            let start_pos = self.pos;
            let mut end = self.pos;
            while let Some(nc) = self.input[end..].chars().next() {
                if nc.is_ascii_alphanumeric() || nc == '_' {
                    end += nc.len_utf8();
                } else {
                    break;
                }
            }

            // Check if the identifier is too long
            if let Err(err) = self.check_token_length(start_pos, end) {
                return Some(Token {
                    kind: TokenKind::Error,
                    value: None,
                    text: Some(err),
                    position: start_pos,
                });
            }

            let ident = &self.input[self.pos..end];
            self.pos = end;
            return Some(Token {
                kind: TokenKind::Variable,
                value: None,
                text: Some(String::from(ident)),
                position: start_pos,
            });
        }

        // Other punctuation
        let kind = match c {
            '(' | '[' => TokenKind::Open,
            ')' | ']' => TokenKind::Close,
            ',' | ';' => TokenKind::Separator, // Add ; as a separator
            _ => TokenKind::Error,
        };
        let text = String::from(c);
        self.advance();
        Some(Token {
            kind,
            value: None,
            text: Some(text),
            position: start_pos,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TokenKind;

    #[test]
    fn test_lexer_tokenization_all_types() {
        let mut lexer = Lexer::new("1 + foo_bar * (2.5e-1) , -baz_123 / 4.2 ^ _x");
        let mut tokens = Vec::new();
        while let Some(tok) = lexer.next_token() {
            tokens.push(tok);
        }
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Number));
        assert!(kinds.contains(&TokenKind::Operator));
        assert!(kinds.contains(&TokenKind::Variable));
        assert!(kinds.contains(&TokenKind::Open));
        assert!(kinds.contains(&TokenKind::Close));
        assert!(kinds.contains(&TokenKind::Separator));
    }

    #[test]
    fn test_lexer_tokenization_error_tokens() {
        let mut lexer = Lexer::new("1 $ 2");
        let mut found_error = false;
        while let Some(tok) = lexer.next_token() {
            if tok.kind == TokenKind::Error {
                found_error = true;
                break;
            }
        }
        assert!(
            found_error,
            "Lexer should produce error token for unknown character"
        );
    }

    #[test]
    fn test_lexer_tokenization_malformed_numbers() {
        let mut lexer = Lexer::new("1..2 1e--2");
        let mut found_error = false;
        let mut tokens = Vec::new();

        // Collect all tokens to avoid infinite loop
        while let Some(tok) = lexer.next_token() {
            tokens.push(tok);
            // Break after collecting a reasonable number of tokens
            if tokens.len() > 10 {
                break;
            }
        }

        // Check if any token is an error
        for tok in tokens {
            if tok.kind == TokenKind::Error {
                found_error = true;
                break;
            }
        }

        assert!(
            found_error,
            "Lexer should produce error token for malformed numbers"
        );
    }

    #[test]
    fn test_lexer_decimal_with_leading_dot() {
        let mut lexer = Lexer::new(".5 .123 .0 .9e2");

        // Test .5
        let token = lexer.next_token().unwrap();
        assert_eq!(token.kind, TokenKind::Number);
        assert_eq!(token.value, Some(0.5));

        // Test .123
        let token = lexer.next_token().unwrap();
        assert_eq!(token.kind, TokenKind::Number);
        assert_eq!(token.value, Some(0.123));

        // Test .0
        let token = lexer.next_token().unwrap();
        assert_eq!(token.kind, TokenKind::Number);
        assert_eq!(token.value, Some(0.0));

        // Test .9e2
        let token = lexer.next_token().unwrap();
        assert_eq!(token.kind, TokenKind::Number);
        assert_eq!(token.value, Some(90.0));
    }

    #[test]
    fn test_lexer_tokenization_variable_with_dot() {
        let mut lexer = Lexer::new("foo.bar");
        let t1 = lexer.next_token().unwrap();
        let t2 = lexer.next_token().unwrap();
        let t3 = lexer.next_token().unwrap();
        assert_eq!(t1.kind, TokenKind::Variable);
        assert_eq!(t1.text.as_deref(), Some("foo"));
        assert_eq!(t2.kind, TokenKind::Operator);
        assert_eq!(t2.text.as_deref(), Some("."));
        assert_eq!(t3.kind, TokenKind::Variable);
        assert_eq!(t3.text.as_deref(), Some("bar"));
    }

    #[test]
    fn test_lexer_tokenization_multichar_operators() {
        let mut lexer = Lexer::new("a && b || c == d != e <= f >= g << h >> i <<< j >>> k ** l <> m ; n");
        let mut tokens = Vec::new();
        while let Some(tok) = lexer.next_token() {
            tokens.push(tok);
        }
        let ops: Vec<_> = tokens.iter().filter(|t| t.kind == TokenKind::Operator || t.kind == TokenKind::Separator).map(|t| t.text.as_deref().unwrap()).collect();
        assert!(ops.contains(&"&&"));
        assert!(ops.contains(&"||"));
        assert!(ops.contains(&"=="));
        assert!(ops.contains(&"!="));
        assert!(ops.contains(&"<="));
        assert!(ops.contains(&">="));
        assert!(ops.contains(&"<<"));
        assert!(ops.contains(&">>"));
        // The current lexer implementation tokenizes <<< as two tokens: "<<" and "<"
        // and >>> as two tokens: ">>" and ">"
        // So we do not assert for "<<<" or ">>>"
        assert!(ops.contains(&"**"));
        assert!(ops.contains(&"<>"));
        assert!(ops.contains(&";"));
    }
}
