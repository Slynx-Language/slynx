use crate::{ASTAttribute, FuncDeclaration, ObjectDeclaration, ObjectMethod, Parser, Result};
use slynx_lexer::tokens::{Token, TokenKind};

use crate::ast::{ObjectField, VisibilityModifier};
use common::{Span, Spanned};

impl<'a> Parser<'a> {
    pub fn parse_method(
        &mut self,
        start: Span,
        attributes: Vec<Spanned<ASTAttribute>>,
    ) -> Result<ObjectMethod> {
        let func = self.parse_func(start, attributes)?;
        let FuncDeclaration {
            name,
            args,
            return_type,
            body,
            type_params,
            ..
        } = func;
        Ok(ObjectMethod {
            type_params,
            method_name: name,
            arguments: args,
            return_type,
            body,
            span: func.span,
        })
    }

    pub fn parse_object(
        &mut self,
        start: Span,
        attributes: Vec<Spanned<ASTAttribute>>,
    ) -> Result<ObjectDeclaration> {
        let (name, generics) = self.parse_generic_name()?;
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while self.peek()?.kind != TokenKind::RBrace {
            let attributes = self.parse_attributes()?;
            if self.peek()?.kind == TokenKind::Func {
                let start = self.eat()?.span;
                methods.push(self.parse_method(start, attributes)?);
                if let TokenKind::Comma = self.peek()?.kind {
                    self.eat()?;
                }
                continue;
            }
            let name = self.parse_typedname(&generics)?;
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
            type_params: generics,
            attributes: attributes,
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
