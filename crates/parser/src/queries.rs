use common::Span;
use slynx_lexer::{Token, TokenKind};

use crate::{ExpectedContent, ParseError, Parser, Result, SymbolPointer, flags::ParserFlag};

impl<'a> Parser<'a> {
    pub fn intern(&self, name: &str) -> SymbolPointer {
        self.symbols.intern(name)
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
    pub fn has_flag(&self, flag: ParserFlag) -> bool {
        self.flags.has_flag(flag)
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
                TokenKind::Identifier(_) => "Instead was expecting a name".to_string(),
                TokenKind::Int(_) => "Instead was expecting an integer literal".to_string(),
                TokenKind::Float(_) => "Instead was expecting a float literal".to_string(),
                TokenKind::String(_) => "Instead was expecting a string literal".to_string(),
                _ => format!("'{kind:?}'",),
            };
            Err(ParseError::UnexpectedToken(
                token,
                ExpectedContent::Raw(kind),
            ))
        }
    }

    ///Does the same as `self.expect()` but expecting specifically an identifier
    pub fn expect_identifier(&mut self) -> Result<(SymbolPointer, Span)> {
        let Token {
            kind: TokenKind::Identifier(name),
            span,
        } = self.expect(&TokenKind::Identifier(String::new()))?
        else {
            unreachable!()
        };
        let name = self.intern(&name);
        Ok((name, span))
    }
    ///Does the same as `self.expect()` but expecting specifically an identifier
    pub fn expect_string(&mut self) -> Result<(SymbolPointer, Span)> {
        let Token {
            kind: TokenKind::String(name),
            span,
        } = self.expect(&TokenKind::String(String::new()))?
        else {
            unreachable!()
        };
        let name = self.intern(&name);
        Ok((name, span))
    }
}
