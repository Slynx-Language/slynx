use common::{Span, Spanned, pool::DedupPoolId};
use slynx_lexer::TokenKind;

use crate::{
    ASTAttribute, EnumDeclaration, EnumVariant, EnumVariantKind, Parser, Result, Type,
    TypeParamScope,
};

impl Parser<'_> {
    pub fn parse_enum_representation(
        &mut self,
        generics: TypeParamScope,
    ) -> Result<Option<Spanned<DedupPoolId<Type>>>> {
        if self.peek()?.kind == TokenKind::Colon {
            self.eat()?;
            let out = self.parse_type(generics)?;
            Ok(Some(out))
        } else {
            Ok(None)
        }
    }

    pub fn parse_enum_variant(
        &mut self,
        attributes: Vec<Spanned<ASTAttribute>>,
        generics: TypeParamScope,
    ) -> Result<EnumVariant> {
        let name = self.expect_identifier()?;
        match self.peek()?.kind {
            TokenKind::Eq => {
                self.eat()?;
                let rhs = self.parse_expression(generics)?;
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
                    .parse_separated(
                        TokenKind::RParen,
                        TokenKind::Comma,
                        true,
                        |parser| parser.parse_type(generics),
                    )?
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
                    .parse_separated(
                        TokenKind::RBrace,
                        TokenKind::Comma,
                        true,
                        |parser| parser.parse_typedname(generics),
                    )?
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

    pub fn parse_enum_variants(&mut self, generics: TypeParamScope) -> Result<Vec<EnumVariant>> {
        self.expect(&TokenKind::LBrace)?;
        let variants = self.parse_separated(
            TokenKind::RBrace,
            TokenKind::Comma,
            true,
            |parser| {
                let attributes = parser.parse_attributes()?;
                parser.parse_enum_variant(attributes, generics)
            },
        )?;
        self.expect(&TokenKind::RBrace)?;
        Ok(variants)
    }

    pub fn parse_enum(
        &mut self,
        span: Span,
        attributes: Vec<Spanned<ASTAttribute>>,
    ) -> Result<EnumDeclaration> {
        let (name, generics) = self.parse_generic_name()?;
        let representation = self.parse_enum_representation(&generics)?;
        let variants = self.parse_enum_variants(&generics)?;

        Ok(EnumDeclaration {
            name,
            type_params: generics,
            representation,
            variants,
            attributes,
            visibility: Default::default(),
            span,
        })
    }
}
