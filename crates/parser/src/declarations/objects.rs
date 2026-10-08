use crate::{
    ASTAttribute, FuncDeclaration, GenericsMetadata, ObjectDeclaration, ObjectMethod, Parser,
    Result, flags::ParserFlags,
};
use slynx_lexer::tokens::{Token, TokenKind};

use crate::ast::{ObjectField, VisibilityModifier};
use common::{Span, Spanned};

impl<'a> Parser<'a> {
    pub fn parse_method(
        &mut self,
        start: Span,
        attributes: Vec<Spanned<ASTAttribute>>,
        flags: ParserFlags,
    ) -> Result<ObjectMethod> {
        let func = self.parse_func(start, attributes, flags)?;
        let FuncDeclaration {
            name,
            args,
            return_type,
            body,
            generics,
            ..
        } = func;
        Ok(ObjectMethod {
            generics,
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
        flags: ParserFlags,
    ) -> Result<ObjectDeclaration> {
        let (name, generics) = self.parse_generic_name()?;
        let interface_implementations = self.parse_interface_implementations(&generics)?;
        let clauses = self.parse_clauses(&generics)?;
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while self.peek()?.kind != TokenKind::RBrace {
            let attributes = self.parse_attributes()?;
            if self.peek()?.kind == TokenKind::Func {
                let start = self.eat()?.span;
                methods.push(self.parse_method(start, attributes, flags)?);
                if self.peek()?.kind == TokenKind::Comma {
                    self.eat()?;
                }
                continue;
            }
            let name = self.parse_typedname(&generics)?;
            fields.push(ObjectField {
                visibility: VisibilityModifier::Public,
                name,
            });

            // Fields are comma separated, but a method may follow the last one
            // without a comma in between, so `func` ends the field list too.
            match self.peek()?.kind {
                TokenKind::Comma => {
                    self.eat()?;
                }
                TokenKind::Func | TokenKind::RBrace => {}
                _ => {
                    return self.unexpected(
                        "Was expecting a ',' between fields or a 'func' method declaration",
                    );
                }
            }
        }
        let Token { span, .. } = self.expect(&TokenKind::RBrace)?;
        Ok(ObjectDeclaration {
            generics: GenericsMetadata {
                type_params: generics,
                interface_implementations,
                clauses,
            },
            attributes,
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
