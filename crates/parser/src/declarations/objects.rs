use crate::{
    ASTAttribute, CallbackDefinition, CallbackOrder, CallbackTarget, FuncDeclaration,
    GenericsMetadata, ObjectDeclaration, ObjectMethod, ParseCollectionDescriptor, ParseError,
    Parser, Result, SymbolPointer, Type, flags::ParserFlags,
};
use slynx_lexer::tokens::{Token, TokenKind};

use crate::ast::{ObjectField, VisibilityModifier};
use common::{Span, Spanned, pool::DedupPoolId};

impl<'a> Parser<'a> {
    pub fn parse_method(
        &mut self,
        start: Span,
        attributes: Vec<Spanned<ASTAttribute>>,
        flags: ParserFlags,
        visibility: VisibilityModifier,
    ) -> Result<ObjectMethod> {
        let func = self.parse_func(start, attributes, visibility, flags)?;
        let FuncDeclaration {
            name,
            args,
            return_type,
            body,
            generics,
            ..
        } = func;
        Ok(ObjectMethod {
            visibility,
            generics,
            name,
            arguments: args,
            return_type,
            body,
            span: func.span,
        })
    }

    fn parse_callback_argument_name(&mut self) -> Result<Spanned<SymbolPointer>> {
        let token = self.eat()?;
        match token.kind {
            TokenKind::Identifier(name) => Ok(token.span.make_spanned(self.intern(&name))),
            TokenKind::Return => Ok(token.span.make_spanned(self.intern("return"))),
            _ => self
                .unexpected("Was expecting an argument name or 'return' in the callback mapping"),
        }
    }

    fn parse_callback_order(&mut self) -> Result<CallbackOrder> {
        let identifier = self.expect_identifier()?;
        let after = match () {
            _ if identifier.data == self.intern("after") => true,
            _ if identifier.data == self.intern("before") => false,
            _ => return Err(ParseError::expected_callback_ordering(identifier.span)),
        };
        let target = {
            let target_function = self.expect_identifier()?;
            self.expect(&TokenKind::LParen)?;
            let arg_names = self.parse_collection(ParseCollectionDescriptor {
                stop_tokens: &[TokenKind::RParen],
                separator_token: Some(TokenKind::Comma),
                parse_item: |parser| parser.expect_identifier(),
            })?;
            self.expect(&TokenKind::RParen)?;
            self.expect(&TokenKind::FatArrow)?;
            self.expect(&TokenKind::LParen)?;
            let order = self.parse_collection(ParseCollectionDescriptor {
                stop_tokens: &[TokenKind::RParen],
                separator_token: Some(TokenKind::Comma),
                parse_item: |parser| parser.parse_callback_argument_name(),
            })?;
            self.expect(&TokenKind::RParen)?;
            CallbackTarget {
                target_function,
                arg_names,
                order,
            }
        };
        Ok(if after {
            CallbackOrder::After(target)
        } else {
            CallbackOrder::Before(target)
        })
    }

    pub fn parse_callback_definition(
        &mut self,
        start: Span,
        attributes: Vec<Spanned<ASTAttribute>>,
    ) -> Result<CallbackDefinition> {
        let (name, generics) = self.parse_generic_name()?;
        if !generics.is_empty() {
            return Err(ParseError::new(
                crate::error::ParseErrorKind::UnsupportedCallbackGenerics,
                start,
            ));
        }
        self.expect(&TokenKind::LParen)?;
        let argument_types: Vec<Spanned<DedupPoolId<Type>>> =
            self.parse_collection(ParseCollectionDescriptor {
                stop_tokens: &[TokenKind::RParen],
                separator_token: Some(TokenKind::Comma),
                parse_item: |parser| parser.parse_type(&generics),
            })?;
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::Colon)?;
        let return_type = self.parse_type(&generics)?;
        let order = self.parse_callback_order()?;
        Ok(CallbackDefinition {
            order,
            span: start.merge_with(return_type.span),
            attributes,
            name,
            argument_types,
            return_type,
        })
    }

    pub fn parse_object(
        &mut self,
        start: Span,
        attributes: Vec<Spanned<ASTAttribute>>,
        external: bool,
        visibility: VisibilityModifier,
        flags: ParserFlags,
    ) -> Result<ObjectDeclaration> {
        let (name, generics) = self.parse_generic_name()?;
        let interface_implementations = self.parse_interface_implementations(&generics)?;
        let clauses = self.parse_clauses(&generics)?;
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut callbacks = Vec::new();

        while self.peek()?.kind != TokenKind::RBrace {
            let attributes = self.parse_attributes()?;
            let visibility = self.parse_visibility()?;
            match self.peek()?.kind {
                TokenKind::Func => {
                    let start = self.eat()?.span;
                    methods.push(self.parse_method(start, attributes, flags, visibility)?);
                    if self.peek()?.kind == TokenKind::Comma {
                        self.eat()?;
                    }
                    continue;
                }
                TokenKind::Callback => {
                    let start = self.eat()?.span;
                    callbacks.push(self.parse_callback_definition(start, attributes)?);
                    if matches!(self.peek()?.kind, TokenKind::Comma | TokenKind::SemiColon) {
                        self.eat()?;
                    }
                    continue;
                }
                _ => {
                    let name = self.parse_typedname(&generics)?;
                    fields.push(ObjectField { visibility, name });
                }
            }

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
            visibility,
            name,
            fields,
            methods,
            callbacks,
            span: Span {
                start: start.start,
                end: span.end,
            },
            external,
        })
    }
}
