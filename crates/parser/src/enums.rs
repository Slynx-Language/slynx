use common::Span;
use slynx_lexer::TokenKind;

use crate::{ASTDeclaration, ASTDeclarationKind, Parser, Result};

impl Parser {
    pub fn parse_enum(&mut self) -> Result<ASTDeclaration> {
        let enum_token = self.expect(&TokenKind::Enum)?;
        let name = self.parse_type()?;
        self.expect(&TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while let TokenKind::RBrace = self.peek()?.kind {
            let tokens = self.expect(&TokenKind::Identifier(String::new()))?;
            if let TokenKind::Identifier(string_name) = tokens.kind {
                variants.push(string_name);
            }
            if self.peek()?.kind != TokenKind::RBrace {
                self.expect(&TokenKind::Comma)?;
            } else {
                let _ = self.expect(&TokenKind::Comma);
            }
        }
        let brace = self.expect(&TokenKind::RBrace)?;
        Ok(ASTDeclaration {
            span: Span {
                start: enum_token.span.start,
                end: brace.span.end,
            },
            kind: ASTDeclarationKind::EnumDeclaration { name, variants },
        })
    }
}
