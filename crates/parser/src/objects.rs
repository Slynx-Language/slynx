use color_eyre::eyre::Result;

use crate::Parser;
use slynx_lexer::tokens::{Token, TokenKind};

use common::ast::{ASTDeclaration, ASTDeclarationKind, ObjectField, Span, VisibilityModifier};

impl Parser {
    pub fn parse_object(&mut self, start: Span) -> Result<ASTDeclaration> {
        let name = self.parse_type()?;
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while self.peek()?.kind != TokenKind::RBrace {
            let name = self.parse_typedname()?;
            fields.push(ObjectField {
                visibility: VisibilityModifier::Public,
                name,
            });

            if self.peek()?.kind == TokenKind::RBrace {
                break;
            } else {
                self.expect(&TokenKind::Comma)?;
            }
        }
        let Token { span, .. } = self.expect(&TokenKind::RBrace)?;
        Ok(ASTDeclaration {
            kind: ASTDeclarationKind::ObjectDeclaration { name, fields },
            span: Span {
                start: start.start,
                end: span.end,
            },
        })
    }
}
