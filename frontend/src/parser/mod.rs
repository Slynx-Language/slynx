mod component;
pub mod error;
mod expr;
mod functions;
pub mod objects;
mod statement;
mod types;

use color_eyre::eyre::{Report, Result};

use crate::lexer::{
    TokenStream,
    tokens::{Token, TokenKind},
};
use crate::parser::error::ParseError;

use common::ast::ASTDeclaration;
pub struct Parser {
    stream: TokenStream,
}

impl Parser {
    ///Creates a new parser instance from the given `stream`
    pub fn new(stream: TokenStream) -> Self {
        Parser { stream }
    }

    /// Consumes the next token from the input stream and returns it.
    /// If the end of the input stream is reached, it returns an error indicating that there
    pub fn eat(&mut self) -> Result<Token> {
        self.stream
            .next()
            .ok_or(Report::new(ParseError::UnexpectedEndOfInput))
    }

    /// Peeks at the token at the specified index without consuming it.
    /// Returns a reference to the token at the given index if it exists, or an error if the end of the input stream is reached.
    pub fn peek_at(&self, idx: usize) -> Result<&Token> {
        self.stream
            .stream
            .get(idx)
            .ok_or(Report::new(ParseError::UnexpectedEndOfInput))
    }

    /// Peeks at the next token without consuming it.
    /// Returns a reference to the next token if it exists, or an error if the end of the input stream is reached.  
    pub fn peek(&self) -> Result<&Token> {
        self.peek_at(0)
    }
    /// Consumes the next token and checks if it matches the expected `kind`.
    /// If it does, it returns the token; otherwise, it returns an error indicating the mismatch. 
    /// The error message will specify what kind of token was expected, providing clarity for debugging purposes.

    pub fn expect(&mut self, kind: &TokenKind) -> Result<Token> {
        
        let token = self.eat()?;
        if std::mem::discriminant(&token.kind) == std::mem::discriminant(kind) {
            Ok(token)
        } else {
            let kind = match kind {
                TokenKind::Identifier(_) => "a name".to_string(),
                TokenKind::Int(_) => "an integer literal".to_string(),
                TokenKind::Float(_) => "a float literal".to_string(),
                TokenKind::String(_) => "a string literal".to_string(),
                _ => format!("'{kind}'",),
            };
            Err(ParseError::UnexpectedToken(token, kind).into())
        }
    }
    /// Parses the declarations in the source code and returns them as a vector of `ASTDeclaration`s.
    /// The parser will continue parsing until it reaches the end of the input stream.
    /// If it encounters an unexpected token, it will return an error indicating the expected token type.
    
    pub fn parse_declarations(&mut self) -> Result<Vec<ASTDeclaration>> {
        let mut out = Vec::new();
        while let Ok(token) = self.peek() {
            match &token.kind {
                TokenKind::Object => {
                    let Token { span, .. } = self.eat()?;
                    out.push(self.parse_object(span)?);
                }
                TokenKind::Component => {
                    let Token { span, .. } = self.eat()?;
                    out.push(self.parse_component(span)?)
                }
                TokenKind::Func => {
                    let Token {
                        kind: TokenKind::Func,
                        span,
                    } = self.eat()?
                    else {
                        unreachable!();
                    };
                    out.push(self.parse_func(span)?)
                }
                _ => {
                    return Err(ParseError::UnexpectedToken(
                        self.eat()?,
                        "Either a macro name(a name terminated by '!' such as 'js!'), 'Component' or 'Func'".to_string(),
                    ).into());
                }
            }
        }
        Ok(out)
    }
}
