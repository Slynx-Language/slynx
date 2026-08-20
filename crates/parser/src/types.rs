use super::Parser;
use crate::error::ParseError;
use crate::{
    ASTExpression, AliasDeclaration, ExpectedContent, SymbolPointer, Type, TypeParamScope,
    TypedName,
};
use crate::{Result, ast::GenericIdentifier};
use common::pool::DedupPoolId;
use common::{Span, Spanned, VisibilityModifier};
use slynx_lexer::tokens::{Token, TokenKind};
use smallvec::{SmallVec, smallvec};
impl Parser<'_> {
    pub fn type_name(&self, ty: DedupPoolId<Type>, type_params: &[SymbolPointer]) -> SymbolPointer {
        crate::type_name(self.types, self.symbols, self.expressions, ty, type_params)
    }

    ///Parses a typed name. A typed name is `name: type`, which is a name that contains a type
    pub fn parse_typedname(&mut self, type_params: TypeParamScope) -> Result<Spanned<TypedName>> {
        let name = self.expect_identifier()?;
        if name.data == self.intern("self") {
            return Ok(Spanned::new(
                TypedName {
                    name,
                    kind: Spanned::new(
                        self.intern_type(Type::Plain(GenericIdentifier {
                            generic: SmallVec::new(),
                            identifier: self.intern("Self"),
                        })),
                        name.span,
                    ),
                },
                name.span,
            ));
        }
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type(type_params)?;
        Ok(Spanned::new(TypedName { name, kind: ty }, name.span))
    }
    ///Parses an alias declaration which follows `alias ty = AnotherType`
    pub fn parse_alias(&mut self, init: Span) -> Result<AliasDeclaration> {
        let (name, generics) = self.parse_generic_name()?;

        self.expect(&TokenKind::Eq)?;
        let target = self.parse_type(
            &generics
                .iter()
                .enumerate()
                .map(|(idx, name)| (*name, idx as u8))
                .collect::<Vec<_>>(),
        )?;

        self.expect(&TokenKind::SemiColon)?;
        Ok(AliasDeclaration {
            type_params: generics,
            visibility: VisibilityModifier::default(),
            span: init.merge_with(target.span),
            name,
            target,
        })
    }

    ///Looks up the given identifier in the currently in-scope type parameters.
    ///Returns the index of the type parameter, so that `T` in
    ///`func A<T>(arg: T)` maps to `Generic(0)`.
    fn generic_param_index(
        &self,
        type_params: &[(SymbolPointer, u8)],
        ident: SymbolPointer,
    ) -> Option<u8> {
        type_params
            .iter()
            .rev()
            .find(|(name, _)| *name == ident)
            .map(|(_, index)| *index)
    }

    ///Parsing a generic name means that it will parse a name which contains after it generics, such as func F<T,K,Q>(){}, this function will then be called to parse F<T,K,Q> which is the name of the
    ///function, and returns the name of the function, and a vector containing the names of the generics
    pub fn parse_generic_name(&mut self) -> Result<(SymbolPointer, Vec<SymbolPointer>)> {
        let name = self.expect_identifier()?;
        if self.peek()?.kind != TokenKind::Lt {
            return Ok((name.data, Vec::new()));
        }
        self.expect(&TokenKind::Lt)?;
        let mut generics = Vec::new();
        while self.peek()?.kind != TokenKind::Gt {
            let name = self.expect_identifier()?;
            if self.peek()?.kind == TokenKind::Comma {
                self.eat()?;
            }
            generics.push(name.data);
        }
        self.expect(&TokenKind::Gt)?;

        Ok((name.data, generics))
    }

    ///Splits the parsed name of a generic declaration into its plain name and
    ///the list of declared type parameters. For example, `identity<T, U>`
    ///becomes `(identity, [Plain("T"), Plain("U")])`. Non-generic names return
    ///the name unchanged and an empty parameter list.
    pub fn split_type_params(
        &self,
        name: Spanned<DedupPoolId<Type>>,
    ) -> (Spanned<DedupPoolId<Type>>, Vec<SymbolPointer>) {
        let Type::Plain(gi) = &self.types[name.data] else {
            return (name, Vec::new());
        };
        if gi.generic.is_empty() {
            return (name, Vec::new());
        }
        let params = gi
            .generic
            .iter()
            .filter_map(|generic| {
                if let Type::Plain(generic_name) = &self.types[generic.data] {
                    Some(generic_name.identifier)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let plain = self.intern_type(Type::Plain(GenericIdentifier {
            generic: SmallVec::new(),
            identifier: gi.identifier,
        }));
        (name.span.make_spanned(plain), params)
    }

    ///Pushes the given type parameters into scope so that references to them
    ///inside the declaration parse as [`Type::Generic`]. Must be paired with a
    ///`type_params.truncate(...)` once the declaration is fully parsed.
    pub fn push_type_params(
        &mut self,
        type_params: &[SymbolPointer],
        out: &mut Vec<(SymbolPointer, u8)>,
    ) {
        for (index, param) in type_params.iter().enumerate() {
            out.push((*param, index as u8));
        }
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
    pub fn parse_type(
        &mut self,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<Type>>> {
        let token = self.peek()?;
        let start_span = token.span;

        let ty = match &token.kind {
            TokenKind::BitAnd => {
                let span = self.expect(&TokenKind::BitAnd)?.span;
                match self.peek()?.kind {
                    TokenKind::Mut => {
                        self.expect(&TokenKind::Mut)?;
                        let ty = self.parse_type(type_params)?;
                        let id = self.intern_type(Type::MutableReference(ty.data));
                        span.merge_with(ty.span).make_spanned(id)
                    }
                    _ => {
                        let ty = self.parse_type(type_params)?;
                        let id = self.intern_type(Type::Reference(ty.data));
                        span.merge_with(ty.span).make_spanned(id)
                    }
                }
            }
            TokenKind::LParen if self.peek_at(1)?.kind == TokenKind::RParen => {
                self.expect(&TokenKind::LParen)?;
                self.expect(&TokenKind::RBrace)?;
                let end_span = self.eat()?.span;
                let id = self.intern_type(Type::Plain(GenericIdentifier {
                    identifier: self.intern("()"),
                    generic: smallvec![],
                }));
                start_span.merge_with(end_span).make_spanned(id)
            }
            TokenKind::LParen => {
                self.eat()?;
                let mut types = smallvec![];
                loop {
                    types.push(self.parse_type(type_params)?);
                    match self.peek()?.kind {
                        TokenKind::Comma => {
                            self.eat()?;
                        }
                        TokenKind::RParen => break,
                        _ => {
                            return Err(ParseError::UnexpectedToken(
                                self.eat()?,
                                ExpectedContent::Raw(
                                    "Was expecting ',' or ')' in tuple type".into(),
                                ),
                            ));
                        }
                    }
                }
                let span = start_span.merge_with(self.eat()?.span);
                let ty = if types.len() == 1 {
                    (types[0] as Spanned<DedupPoolId<Type>>).data
                } else {
                    self.intern_type(Type::Plain(GenericIdentifier {
                        identifier: self.intern("()"),
                        generic: types,
                    }))
                };

                span.make_spanned(ty)
            }
            TokenKind::LBracket => {
                enum TypeVariant {
                    Vector,
                    Array(DedupPoolId<ASTExpression>),
                }
                let start_span = self.eat()?.span;
                let ty = if self.peek()?.kind == TokenKind::RBracket {
                    self.eat()?;
                    TypeVariant::Vector
                } else {
                    let expr = self.parse_expression(type_params)?;
                    self.expect(&TokenKind::RBracket)?;
                    TypeVariant::Array(expr.data)
                };
                let inner_type = self.parse_type(type_params)?;
                let span = start_span.merge_with(inner_type.span);
                let out = match ty {
                    TypeVariant::Vector => self.intern_type(Type::Vector(inner_type.data)),
                    TypeVariant::Array(size) => {
                        self.intern_type(Type::Array(inner_type.data, size))
                    }
                };
                span.make_spanned(out)
            }

            _ => {
                let ident = self.expect_identifier()?;
                let span = ident.span;
                if let Token {
                    kind: TokenKind::Lt,
                    ..
                } = self.peek()?
                {
                    let mut generics = SmallVec::new();
                    self.eat()?;
                    let span = loop {
                        if let TokenKind::Gt = self.peek()?.kind {
                            let end = self.eat()?.span;
                            break ident.span.merge_with(end);
                        }
                        let ty = self.parse_type(type_params)?;
                        generics.push(ty);
                        if self.peek()?.kind == TokenKind::Comma {
                            self.eat()?;
                        }
                    };
                    let id = self.intern_type(Type::Plain(GenericIdentifier {
                        generic: generics,
                        identifier: ident.data,
                    }));
                    span.make_spanned(id)
                } else if let Some(index) = self.generic_param_index(type_params, ident.data) {
                    span.make_spanned(self.intern_type(Type::Generic(index)))
                } else {
                    let id = self.intern_type(Type::Plain(GenericIdentifier {
                        generic: smallvec![],
                        identifier: ident.data,
                    }));
                    span.make_spanned(id)
                }
            }
        };
        if self.peek()?.kind == TokenKind::Question {
            let end = self.eat()?.span;
            let span = ty.span.merge_with(end);
            let ty = self.intern_type(Type::Nullable(ty.data));
            Ok(span.make_spanned(ty))
        } else {
            Ok(ty)
        }
    }

    ///Looks ahead without consuming to check whether the current identifier is a
    ///generic application (`Name<...>`), that is, a closing `>` followed by `(`
    ///for a function call or `{` for a component expression. Must be called when
    ///the token right after the identifier is `<`. Unlike [`is_generic`](Self::is_generic),
    ///this also recognizes type arguments that do not start with an identifier,
    ///such as `funcall<[4]int>`.
    pub fn is_generic_application(&self) -> Result<bool> {
        if !matches!(self.peek_at(1)?.kind, TokenKind::Lt) {
            return Ok(false);
        }
        let mut depth = 0usize;
        let mut i = 2usize;
        loop {
            let Some(token) = self.stream.stream.get(i) else {
                return Ok(false);
            };
            match &token.kind {
                TokenKind::Lt => depth += 1,
                TokenKind::Gt => {
                    if depth == 0 {
                        return Ok(matches!(
                            self.stream.stream.get(i + 1).map(|t| &t.kind),
                            Some(TokenKind::LParen) | Some(TokenKind::LBrace)
                        ));
                    }
                    depth -= 1;
                }
                TokenKind::Question
                | TokenKind::Identifier(_)
                | TokenKind::Int(_)
                | TokenKind::Float(_)
                | TokenKind::LBracket
                | TokenKind::RBracket
                | TokenKind::LParen
                | TokenKind::RParen
                | TokenKind::Comma
                | TokenKind::Dot => {}
                _ => return Ok(false),
            }
            i += 1;
        }
    }
}
