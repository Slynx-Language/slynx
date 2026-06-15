mod ast;
mod component;
pub mod conditionals;
mod declarations;
pub mod error;
mod flags;
mod import;
pub use error::*;
mod expr;
mod functions;
pub mod objects;
mod statement;
mod styles;
mod types;
pub use ast::*;

use slynx_lexer::{Token, TokenKind, TokenStream};

use crate::flags::{ParserFlag, ParserFlags};

pub type Result<T> = std::result::Result<T, ParseError>;

pub struct Parser {
    flags: ParserFlags,
    component_expr_enabled: bool,
    stream: TokenStream,
}

impl Parser {
    ///Creates a new parser instance from the given `stream`
    pub fn new(stream: TokenStream) -> Self {
        Parser {
            stream,
            flags: ParserFlags::new(),
            component_expr_enabled: true,
        }
    }

    /// Consumes the next token from the input stream and returns it.
    /// If the end of the input stream is reached, it returns an error indicating that there
    pub fn eat(&mut self) -> Result<Token> {
        self.stream.next().ok_or(ParseError::UnexpectedEndOfInput)
    }

    /// Peeks at the token at the specified index without consuming it.
    /// Returns a reference to the token at the given index if it exists, or an error if the end of the input stream is reached.
    pub fn peek_at(&self, idx: usize) -> Result<&Token> {
        self.stream
            .stream
            .get(idx)
            .ok_or(ParseError::UnexpectedEndOfInput)
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
                _ => format!("'{kind:?}'",),
            };
            Err(ParseError::UnexpectedToken(token, kind))
        }
    }

    ///Does the same as `self.expect()` but expecting specifically an identifier
    pub fn expect_identifier(&mut self) -> Result<Token> {
        self.expect(&TokenKind::Identifier(String::new()))
    }
    ///Does the same as `self.expect()` but expecting specifically an identifier
    pub fn expect_string(&mut self) -> Result<Token> {
        self.expect(&TokenKind::String(String::new()))
    }
    pub fn reset_flags(&mut self) {
        self.flags.reset();
    }
    pub fn add_flag(&mut self, flag: ParserFlag) {
        self.flags.set_flag(flag);
    }
    pub fn remove_flag(&mut self, flag: ParserFlag) {
        self.flags.remove_flag(flag);
    }

    pub fn component_expr_enabled(&self) -> bool {
        self.component_expr_enabled
    }

    pub fn parse_without_component_expr<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        let previous = self.component_expr_enabled;
        self.component_expr_enabled = false;
        let result = parse(self);
        self.component_expr_enabled = previous;
        result
    }

    pub fn finish_current_parse(&mut self) -> Result<()> {
        if self.flags.has_flag(ParserFlag::RequireSemicolon) {
            self.expect(&TokenKind::SemiColon)?;
        }
        self.reset_flags();
        Ok(())
    }
}
