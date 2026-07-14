use super::Parser;
use crate::error::ParseError;
use crate::{ASTExpression, AliasDeclaration, ExpectedContent, Type, TypedName};
use crate::{Result, ast::GenericIdentifier};
use common::pool::DedupPoolId;
use common::{Span, Spanned, VisibilityModifier};
use slynx_lexer::tokens::{Token, TokenKind};
use smallvec::{SmallVec, smallvec};
impl Parser<'_> {
    ///Parses a typed name. A typed name is `name: type`, which is a name that contains a type
    pub fn parse_typedname(&mut self) -> Result<Spanned<TypedName>> {
        let (name, span) = self.expect_identifier()?;
        if name == self.intern("self") {
            return Ok(Spanned::new(
                TypedName {
                    name,
                    kind: Spanned::new(
                        self.intern_type(Type::Plain(GenericIdentifier {
                            generic: SmallVec::new(),
                            identifier: self.intern("Self"),
                        })),
                        span,
                    ),
                },
                span,
            ));
        }
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        Ok(Spanned::new(TypedName { name, kind: ty }, span))
    }
    ///Parses an alias declaration which follows `alias ty = AnotherType`
    pub fn parse_alias(&mut self, init: Span) -> Result<AliasDeclaration> {
        let name = self.parse_type()?;
        self.expect(&TokenKind::Eq)?;
        let target = self.parse_type()?;
        self.expect(&TokenKind::SemiColon)?;
        Ok(AliasDeclaration {
            visibility: VisibilityModifier::default(),
            span: init.merge_with(target.span),
            name,
            target,
        })
    }

    ///Looking from where this function initializes, check is this is a generic one.
    ///Note that this will only work when the function initializes on something like: N<...
    ///the '...' is what this function will check. It won't eat anything, just look ahead.
    ///`ahead` is just a parameter to know how many tokens to look ahead. When using this, it should be initialized
    ///by the index where the token after '<' is at. This function will return weather it was a generic or not, and return the amount needed to look ahead to keep going
    pub fn is_generic(&self, mut ahead: usize) -> Result<(bool, usize)> {
        let initial_ahead = ahead;
        let Token {
            kind: TokenKind::Identifier(_),
            ..
        } = self.peek_at(ahead)?
        else {
            return Ok((false, ahead));
        };

        if let TokenKind::Lt = self.peek_at(ahead + 1)?.kind {
            match self.is_generic(ahead + 2)? {
                (true, n) => ahead += n,
                (false, n) => return Ok((false, n - initial_ahead)),
            }
        }
        Ok((
            matches!(self.peek_at(ahead + 1)?.kind, TokenKind::Gt),
            ahead - initial_ahead,
        ))
    }

    ///Parses a type.
    pub fn parse_type(&mut self) -> Result<Spanned<DedupPoolId<Type>>> {
        let token = self.peek()?;
        let start_span = token.span;
        if let TokenKind::LParen = &token.kind {
            self.eat()?;
            if let TokenKind::RParen = self.peek()?.kind {
                let end_span = self.eat()?.span;
                let id = self.intern_type(Type::Plain(GenericIdentifier {
                    identifier: self.intern("()"),
                    generic: smallvec![],
                }));
                return Ok(end_span.make_spanned(id));
            }
            let mut types = smallvec![];
            loop {
                types.push(self.parse_type()?);
                match self.peek()?.kind {
                    TokenKind::Comma => {
                        self.eat()?;
                    }
                    TokenKind::RParen => break,
                    _ => {
                        return Err(ParseError::UnexpectedToken(
                            self.eat()?,
                            ExpectedContent::Raw("Was expecting ',' or ')' in tuple type".into()),
                        ));
                    }
                }
            }
            let span = start_span.merge_with(self.eat()?.span);
            let ty = self.intern_type(Type::Plain(GenericIdentifier {
                identifier: self.intern("()"),
                generic: types,
            }));

            return Ok(span.make_spanned(ty));
        }
        if self.peek()?.kind == TokenKind::LBracket {
            enum TypeVariant {
                Vector,
                Array(DedupPoolId<ASTExpression>),
            }
            let start_span = self.eat()?.span;
            let ty = if self.peek()?.kind == TokenKind::RBracket {
                self.eat()?;
                TypeVariant::Vector
            } else {
                let expr = self.parse_expression()?;
                self.expect(&TokenKind::RBracket)?;
                TypeVariant::Array(expr.data)
            };
            let inner_type = self.parse_type()?;
            let span = start_span.merge_with(inner_type.span);
            let out = match ty {
                TypeVariant::Vector => self.intern_type(Type::Vector(inner_type.data)),
                TypeVariant::Array(size) => self.intern_type(Type::Array(inner_type.data, size)),
            };
            return Ok(span.make_spanned(out));
        }
        let (ident, mut span) = self.expect_identifier()?;
        if let Token {
            kind: TokenKind::Lt,
            ..
        } = self.peek()?
        {
            let mut generics = SmallVec::new();
            self.eat()?;
            loop {
                if let TokenKind::Gt = self.peek()?.kind {
                    let end = self.eat()?.span;
                    span.end = end.end;
                    break;
                }
                let ty = self.parse_type()?;
                generics.push(ty);
            }
            let id = self.intern_type(Type::Plain(GenericIdentifier {
                generic: generics,
                identifier: ident,
            }));
            Ok(span.make_spanned(id))
        } else {
            let id = self.intern_type(Type::Plain(GenericIdentifier {
                generic: smallvec![],
                identifier: ident,
            }));
            Ok(span.make_spanned(id))
        }
    }
}
