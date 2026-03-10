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

    pub fn eat(&mut self) -> Result<Token> {
        self.stream
            .next()
            .ok_or(Report::new(ParseError::UnexpectedEndOfInput))
    }

    pub fn peek_at(&self, idx: usize) -> Result<&Token> {
        self.stream
            .stream
            .get(idx)
            .ok_or(Report::new(ParseError::UnexpectedEndOfInput))
    }

    pub fn peek(&self) -> Result<&Token> {
        self.peek_at(0)
    }

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

    pub fn parse_declarations(&mut self) -> Result<Vec<ASTDeclaration>> {
        let mut out = Vec::new();
        let mut pending_doc: Option<String> = None;
        while let Ok(token) = self.peek() {
            match &token.kind {
                TokenKind::DocComment(doc) => {
                    let doc = doc.clone();
                    self.eat()?;
                    pending_doc = Some(match pending_doc.take() {
                        Some(prev) if !prev.is_empty() && !doc.is_empty() => {
                            format!("{prev}\n\n{doc}")
                        }
                        Some(prev) if !prev.is_empty() => prev,
                        _ => doc,
                    });
                }
                TokenKind::Object => {
                    let Token { span, .. } = self.eat()?;
                    let mut decl = self.parse_object(span)?;
                    decl.doc = pending_doc.take();
                    out.push(decl);
                }
                TokenKind::Component => {
                    let Token { span, .. } = self.eat()?;
                    let mut decl = self.parse_component(span)?;
                    decl.doc = pending_doc.take();
                    out.push(decl)
                }
                TokenKind::Func => {
                    let Token {
                        kind: TokenKind::Func,
                        span,
                    } = self.eat()?
                    else {
                        unreachable!();
                    };
                    let mut decl = self.parse_func(span)?;
                    decl.doc = pending_doc.take();
                    out.push(decl)
                }
                _ => {
                    return Err(ParseError::UnexpectedToken(
                        self.eat()?,
                        "Either a macro name(a name terminated by '!' such as 'js!'), 'Component' or 'Func'".to_string(),
                    ).into());
                }
            }
        }
        if pending_doc.is_some() {
            return Err(ParseError::UnexpectedEndOfInput.into());
        }
        Ok(out)
    }
}
