use crate::{FuncDeclaration, ObjectDeclaration, ObjectMethod, Parser, Result};
use slynx_lexer::tokens::{Token, TokenKind};

use crate::ast::{ObjectField, VisibilityModifier};
use common::Span;

impl<'a> Parser<'a> {
    pub fn parse_method(&mut self, start: Span) -> Result<ObjectMethod> {
        let func = self.parse_func(start)?;
        let FuncDeclaration {
            name,
            args,
            return_type,
            body,
            ..
        } = func;
        Ok(ObjectMethod {
            method_name: name,
            arguments: args,
            return_type,
            body,
            span: func.span,
        })
    }

    pub fn parse_object(&mut self, start: Span) -> Result<ObjectDeclaration> {
        let name = self.parse_type()?;
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        while self.peek()?.kind != TokenKind::RBrace {
            if self.peek()?.kind == TokenKind::Func {
                let start = self.eat()?.span;
                methods.push(self.parse_method(start)?);
                if let TokenKind::Comma = self.peek()?.kind {
                    self.eat()?;
                }
                continue;
            }
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
        Ok(ObjectDeclaration {
            attributes: Vec::new(),
            visibility: Default::default(),
            name,
            fields,
            methods,
            span: Span {
                start: start.start,
                end: span.end,
            },
            external: false,
        })
    }
}
