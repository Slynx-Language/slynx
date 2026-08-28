use common::{Span, Spanned, pool::DedupPoolId};
use slynx_lexer::TokenKind;
use smallvec::SmallVec;

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
                let mut associated_types = SmallVec::new();
                while self.peek()?.kind != TokenKind::RParen {
                    let inner = self.parse_type(generics)?;
                    associated_types.push(inner);
                    if self.peek()?.kind == TokenKind::Comma {
                        self.eat()?;
                    }
                }
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
                let mut types = SmallVec::new();
                while self.peek()?.kind != TokenKind::RParen {
                    let inner = self.parse_typedname(generics)?;
                    types.push(inner);
                    if self.peek()?.kind == TokenKind::Comma {
                        self.eat()?;
                    }
                }
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
        let mut variants = Vec::new();
        while self.peek()?.kind != TokenKind::RBrace {
            let attributes = self.parse_attributes()?;
            variants.push(self.parse_enum_variant(attributes, generics)?);
            if self.peek()?.kind == TokenKind::Comma {
                self.eat()?;
            }
        }
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
