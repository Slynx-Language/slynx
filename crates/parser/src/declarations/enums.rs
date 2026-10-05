use common::{Span, Spanned, pool::DedupPoolId};
use slynx_lexer::TokenKind;

use crate::{
    ASTAttribute, EnumDeclaration, EnumVariant, EnumVariantKind, GenericsMetadata, ObjectMethod,
    Parser, Result, SymbolPointer, Type, flags::ParserFlags,
};

impl Parser<'_> {
    pub fn parse_enum_representation(
        &mut self,
        generics: &[SymbolPointer],
    ) -> Result<Option<Spanned<DedupPoolId<Type>>>> {
        //enum Slaoq(u8): Interface {}
        if self.peek()?.kind == TokenKind::LParen {
            self.eat()?;
            let out = self.parse_type(generics)?;
            self.expect(&TokenKind::RParen)?;
            Ok(Some(out))
        } else {
            Ok(None)
        }
    }

    pub fn parse_enum_variant(
        &mut self,
        attributes: Vec<Spanned<ASTAttribute>>,
        generics: &[SymbolPointer],
    ) -> Result<EnumVariant> {
        let name = self.expect_identifier()?;
        match self.peek()?.kind {
            TokenKind::Eq => {
                self.eat()?;
                let rhs = self.parse_expression(generics, ParserFlags::default())?;
                Ok(EnumVariant {
                    name,
                    kind: EnumVariantKind::RawValued(rhs),
                    attributes,
                    span: name.span.merge_with(rhs.span),
                })
            }
            TokenKind::LParen => {
                self.eat()?;
                let associated_types = self
                    .parse_separated(TokenKind::RParen, TokenKind::Comma, true, |parser| {
                        parser.parse_type(generics)
                    })?
                    .into();
                let endspan = self.expect(&TokenKind::RParen)?.span;
                Ok(EnumVariant {
                    name,
                    kind: EnumVariantKind::Associated(associated_types),
                    attributes,
                    span: name.span.merge_with(endspan),
                })
            }
            TokenKind::LBrace => {
                self.eat()?;
                let types = self
                    .parse_separated(TokenKind::RBrace, TokenKind::Comma, true, |parser| {
                        parser.parse_typedname(generics)
                    })?
                    .into();
                let endspan = self.expect(&TokenKind::RBrace)?.span;
                Ok(EnumVariant {
                    name,
                    kind: EnumVariantKind::Struct(types),
                    attributes,
                    span: name.span.merge_with(endspan),
                })
            }
            _ => Ok(EnumVariant {
                name,
                kind: EnumVariantKind::Raw,
                attributes,
                span: name.span,
            }),
        }
    }

    ///Parses the body of an enum, returning its variants and its methods
    ///separately. Variants and methods share the body, so they are read in one
    ///loop and split apart by the token that introduces them.
    pub fn parse_enum_variants(
        &mut self,
        generics: &[SymbolPointer],
    ) -> Result<(Vec<EnumVariant>, Vec<ObjectMethod>)> {
        self.expect(&TokenKind::LBrace)?;
        let mut variants = Vec::new();
        let mut methods = Vec::new();
        while self.peek()?.kind != TokenKind::RBrace {
            let attributes = self.parse_attributes()?;
            if self.peek()?.kind == TokenKind::Func {
                let start = self.eat()?.span;
                methods.push(self.parse_method(start, attributes, ParserFlags::empty())?);
            } else {
                variants.push(self.parse_enum_variant(attributes, generics)?);
            }
            if self.peek()?.kind == TokenKind::Comma {
                self.eat()?;
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok((variants, methods))
    }

    pub fn parse_enum(
        &mut self,
        span: Span,
        attributes: Vec<Spanned<ASTAttribute>>,
    ) -> Result<EnumDeclaration> {
        //enum E(int): InterfaceA, InterfaceB where T: InterfaceC { ... }
        let (name, generics) = self.parse_generic_name()?;
        let representation = self.parse_enum_representation(&generics)?;
        let interface_implementations = self.parse_interface_implementations(&generics)?;
        let clauses = self.parse_clauses(&generics)?;
        let (variants, methods) = self.parse_enum_variants(&generics)?;

        Ok(EnumDeclaration {
            name,
            generics: GenericsMetadata {
                type_params: generics,
                interface_implementations,
                clauses,
            },
            representation,
            variants,
            attributes,
            visibility: Default::default(),
            span,
            methods,
        })
    }
}
