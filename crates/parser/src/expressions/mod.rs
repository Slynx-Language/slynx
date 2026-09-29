use crate::flags::ParserFlags;
use crate::{ASTExpression, Type};
use crate::{Parser, Result, error::ParseError};
use common::Spanned;
use common::pool::DedupPoolId;
use ordered_float::OrderedFloat;
use slynx_lexer::tokens::{Token, TokenKind};
use smallvec::SmallVec;
mod binary;
mod collections;
mod conditionals;
mod data_types;
use crate::SymbolPointer;
impl Parser<'_> {
    /// Parses a function call expression.
    /// It expects the current token to be an identifier, followed by a left parenthesis '(', then a list of expressions as arguments separated by commas, and finally a right parenthesis ')'.
    pub fn parse_funcall(
        &mut self,
        type_params: &[SymbolPointer],
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let identifier = self.parse_type(type_params)?;
        self.parse_funcall_args(identifier, type_params)
    }

    ///Parses the arguments of a function call, given the already-parsed call name.
    ///This allows generic calls such as `identity<i32>(x)`, whose name has
    ///already been parsed as a generic type.
    pub fn parse_funcall_args(
        &mut self,
        identifier: Spanned<DedupPoolId<Type>>,
        type_params: &[SymbolPointer],
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.expect(&TokenKind::LParen)?;
        let params: SmallVec<[Spanned<DedupPoolId<ASTExpression>>; 7]> = self
            .parse_separated(TokenKind::RParen, TokenKind::Comma, false, |parser| {
                parser.parse_expression(type_params, ParserFlags::empty())
            })?
            .into();
        let Token { span: last, .. } = self.expect(&TokenKind::RParen)?;
        let span = identifier.span.merge_with(last);
        let id = self.intern_expression(ASTExpression::FunctionCall {
            name: identifier,
            args: params,
        });
        Ok(Spanned::new(id, span))
    }

    ///Parses anything that comes prefixed by a identifier. This can be a function call, object creation, or a struct creation. This is executed without eating the identifier to be able to choose what to
    ///return
    pub fn parse_identifier_exprs(
        &mut self,
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Option<Spanned<DedupPoolId<ASTExpression>>>> {
        let after_identifier = &self.peek_at(1)?.kind;
        match after_identifier {
            TokenKind::Lt if self.is_generic_application()? => {
                let ty = self.parse_type(type_params)?;
                match self.peek()?.kind {
                    TokenKind::LParen => {
                        match (&self.peek_at(1)?.kind, &self.peek_at(2)?.kind) {
                            //check if its name<T>(a,b) or name<T>(a:b)
                            (TokenKind::Identifier(_), TokenKind::Colon) => Ok(Some(
                                self.parse_object_expression_with_name(ty, type_params)?,
                            )),
                            _ => Ok(Some(self.parse_funcall_args(ty, type_params)?)),
                        }
                    }
                    TokenKind::LBrace => {
                        let component = self.parse_component_expr_with_name(ty, type_params)?;
                        let span = component.span;
                        let id = self.intern_expression(ASTExpression::Component(component.data));
                        Ok(Some(Spanned::new(id, span)))
                    }
                    _ => self.unexpected("Instead was expecting '(' or '{' after a generic name"),
                }
            }
            TokenKind::Lt => Ok(None),
            TokenKind::LBrace if flags.contains(ParserFlags::COMPONENT_EXPR) => {
                let component = self.parse_component_expr(type_params)?;
                let id = self.intern_expression(ASTExpression::Component(component.data));
                Ok(Some(Spanned::new(id, component.span)))
            }
            TokenKind::LParen => {
                match (&self.peek_at(2)?.kind, &self.peek_at(3)?.kind) {
                    //check if its name(a,b) or name(a:b), or name(.a:b)
                    (TokenKind::Identifier(_), TokenKind::Colon) => {
                        Ok(Some(self.parse_object_expression(type_params)?))
                    }
                    _ => Ok(Some(self.parse_funcall(type_params)?)),
                }
            }
            _ => Ok(None),
        }
    }

    fn parse_postfix_chain(
        &mut self,
        mut expr: Spanned<DedupPoolId<ASTExpression>>,
        type_params: &[SymbolPointer],
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        // Keep postfix parsing iterative so tuple access and chained field access
        // share the same code path.
        loop {
            match self.peek()?.kind {
                TokenKind::Dot => {
                    self.eat()?;
                    expr = self.parse_dot_postfix(expr, type_params)?;
                }
                TokenKind::LBracket => {
                    expr = self.parse_array_access(expr, type_params)?;
                }
                _ => break Ok(expr),
            }
        }
    }

    ///Parses a postfix that comes after a '.'. This function initializes right after the '.'
    pub fn parse_dot_postfix(
        &mut self,
        prefix: Spanned<DedupPoolId<ASTExpression>>,
        type_params: &[SymbolPointer],
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        match &self.peek()?.kind {
            TokenKind::Int(index) if *index >= 0 => {
                let index = *index;
                let current = self.eat()?;
                let span = prefix.span.merge_with(current.span);
                let id = self.intern_expression(ASTExpression::TupleAccess {
                    tuple: prefix,
                    index: index as u8,
                });
                Ok(Spanned::new(id, span))
            }
            TokenKind::Identifier(_) if self.peek_at(1)?.kind == TokenKind::LParen => {
                let field = self.parse_funcall(type_params)?;
                let span = prefix.span.merge_with(field.span);
                let id = self.intern_expression(ASTExpression::FieldAccess {
                    parent: prefix,
                    field,
                });
                Ok(Spanned::new(id, span))
            }
            TokenKind::Identifier(_) => {
                let ident = self.expect_identifier()?;
                let field = Spanned::new(
                    self.intern_expression(ASTExpression::Identifier(ident.data)),
                    ident.span,
                );
                let span = prefix.span.merge_with(field.span);
                let parent = self.intern_expression(ASTExpression::FieldAccess {
                    parent: prefix,
                    field,
                });
                Ok(Spanned::new(parent, span))
            }
            _ => Err(ParseError::InvalidPostfix(self.eat()?.span)),
        }
    }
    /// Parses a primary expression, which can be a literal (integer, float, string, boolean), an identifier, a parenthesized expression, or a field access expression.
    pub fn parse_primary(
        &mut self,
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let current = self.peek()?;

        let expr = if let TokenKind::If = current.kind {
            let span = self.eat()?.span;
            self.parse_if(span, type_params)?
        } else if let TokenKind::Identifier(_) = self.peek()?.kind
            && let Some(value) = self.parse_identifier_exprs(type_params, flags)?
        {
            value
        } else {
            let current = self.eat()?;
            match current.kind {
                TokenKind::Int(i) => Ok(Spanned::new(
                    self.intern_expression(ASTExpression::IntLiteral(i)),
                    current.span,
                )),
                TokenKind::Float(f) => Ok(Spanned::new(
                    self.intern_expression(ASTExpression::FloatLiteral(OrderedFloat(f))),
                    current.span,
                )),
                TokenKind::Identifier(i) => Ok(Spanned::new(
                    self.intern_expression(ASTExpression::Identifier(self.intern(&i))),
                    current.span,
                )),

                TokenKind::String(s) => Ok(Spanned::new(
                    self.intern_expression(ASTExpression::StringLiteral(self.intern(&s))),
                    current.span,
                )),
                TokenKind::True => Ok(Spanned::new(
                    self.intern_expression(ASTExpression::True),
                    current.span,
                )),
                TokenKind::False => Ok(Spanned::new(
                    self.intern_expression(ASTExpression::False),
                    current.span,
                )),
                TokenKind::LParen => {
                    let first = self.parse_expression(type_params, flags)?;
                    if self.peek()?.kind == TokenKind::Comma {
                        self.eat()?;
                        self.parse_tuple_with_first(first, type_params)
                    } else {
                        self.expect(&TokenKind::RParen)?;
                        Ok(first)
                    }
                }

                _ => self.unexpected_with("Was expecting an expression", current),
            }?
        };

        self.parse_postfix_chain(expr, type_params)
    }

    /// Parses an expression, which is the top-level function for parsing any kind of expression. It starts by parsing a logical expression, which can include comparisons, additive, multiplicative, and primary expressions, and returns the resulting ASTExpression.
    pub fn parse_expression(
        &mut self,
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let expr = match self.peek()?.kind {
            TokenKind::If => {
                let span = self.eat()?.span;
                self.parse_if(span, type_params)
            }
            TokenKind::LBracket => {
                let span = self.eat()?.span;
                self.parse_array(span, type_params)
            }
            TokenKind::LBrace => {
                let span = self.eat()?.span;
                self.parse_vector(span, type_params)
            }

            TokenKind::Star => {
                let start = self.eat()?.span;
                let expr = self.parse_expression(type_params, flags)?;

                Ok(start
                    .merge_with(expr.span)
                    .make_spanned(self.intern_expression(ASTExpression::Deref(expr))))
            }
            TokenKind::BitAnd => {
                let start = self.eat()?.span;
                let (id, end) = if self.peek()?.kind == TokenKind::Mut {
                    self.expect(&TokenKind::Mut)?;
                    let expr = self.parse_expression(type_params, flags)?;
                    (
                        self.intern_expression(ASTExpression::Reference {
                            mutable: true,
                            expr,
                        }),
                        expr.span,
                    )
                } else {
                    let expr = self.parse_expression(type_params, flags)?;
                    (
                        self.intern_expression(ASTExpression::Reference {
                            mutable: false,
                            expr,
                        }),
                        expr.span,
                    )
                };
                Ok(start.merge_with(end).make_spanned(id))
            }
            _ => self.parse_logical(type_params, flags),
        }?;

        Ok(expr)
    }
}
