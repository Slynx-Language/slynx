use crate::{
    ASTExpression, ComponentExpression, ComponentMemberValue, NamedExpr, RangeType, Type,
    TypeParamScope,
};
use crate::{Parser, Result, error::ParseError};
use common::pool::DedupPoolId;
use common::{Operator, Span, Spanned};
use ordered_float::OrderedFloat;
use slynx_lexer::tokens::{Token, TokenKind};
use smallvec::{SmallVec, smallvec};

impl Parser<'_> {
    /// Parses a function call expression.
    /// It expects the current token to be an identifier, followed by a left parenthesis '(', then a list of expressions as arguments separated by commas, and finally a right parenthesis ')'.
    pub fn parse_funcall(
        &mut self,
        type_params: TypeParamScope,
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
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.expect(&TokenKind::LParen)?;
        let params: SmallVec<[Spanned<DedupPoolId<ASTExpression>>; 7]> = self
            .parse_separated(TokenKind::RParen, TokenKind::Comma, false, |parser| {
                parser.parse_expression(type_params)
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

    ///Parses an component expression but, starting from the LBrace, assuming the name of the component is the provided `name`
    pub fn parse_component_expr_with_name(
        &mut self,
        name: Spanned<DedupPoolId<Type>>,
        type_params: TypeParamScope,
    ) -> Result<Spanned<ComponentExpression>> {
        let mut span = name.span;
        self.expect(&TokenKind::LBrace)?;
        let mut values = Vec::new();
        loop {
            if let Ok(curr) = self.peek()
                && curr.kind == TokenKind::RBrace
            {
                span.end = curr.span.end;
                break;
            };

            match self.peek_at(1)?.kind {
                TokenKind::Colon => {
                    let ident = self.expect_identifier()?;
                    self.expect(&TokenKind::Colon)?;
                    let val = self.parse_expression(type_params)?;
                    values.push(ComponentMemberValue::Assign {
                        prop_name: ident.data,
                        rhs: val,
                    });
                    if self.peek()?.kind == TokenKind::Comma {
                        self.eat()?;
                    }
                }
                _ => {
                    let val = self.parse_component_expr(type_params)?;
                    values.push(ComponentMemberValue::Child(val.data));
                }
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Spanned::new(ComponentExpression { name, values }, span))
    }
    pub fn parse_component_expr(
        &mut self,
        type_params: TypeParamScope,
    ) -> Result<Spanned<ComponentExpression>> {
        let ty = self.parse_type(type_params)?;
        self.parse_component_expr_with_name(ty, type_params)
    }

    ///From the current token parses a `NamedExpr`. It starts from the current token supposing it's a identifier,
    ///and parses expecting ':' and then another expression
    pub fn parse_named_expr(&mut self, type_params: TypeParamScope) -> Result<Spanned<NamedExpr>> {
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::Colon)?;
        let expr = self.parse_expression(type_params)?;
        let span = name.span.merge_with(expr.span);
        Ok(Spanned::new(
            NamedExpr {
                name: name.data,
                expr,
            },
            span,
        ))
    }
    ///Parses a tuple expression, which follows the rule (expr, expr, expr) or ()
    pub fn parse_tuple_with_first(
        &mut self,
        start: Spanned<DedupPoolId<ASTExpression>>,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let start_span = start.span;

        if self.peek()?.kind == TokenKind::RParen {
            let _ = self.eat()?;
            return Ok(start);
        }
        let mut vec = smallvec![start];
        while self.peek()?.kind != TokenKind::RParen {
            vec.push(self.parse_expression(type_params)?);
            if self.peek()?.kind == TokenKind::Comma {
                self.eat()?;
            }
        }
        let end = self.expect(&TokenKind::RParen)?.span;
        let span = start_span.merge_with(end);
        let id = self.intern_expression(ASTExpression::Tuple(vec));
        Ok(Spanned { data: id, span })
    }
    ///Parses an object expression, which follows the rule Object(field: expr, field: value)
    pub fn parse_object_expression_with_name(
        &mut self,
        name: Spanned<DedupPoolId<Type>>,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.expect(&TokenKind::LParen)?;
        let fields: SmallVec<[Spanned<NamedExpr>; 4]> = self
            .parse_separated(TokenKind::RParen, TokenKind::Comma, true, |parser| {
                parser.parse_named_expr(type_params)
            })?
            .into();
        let end = self.expect(&TokenKind::RParen)?.span;
        let span = name.span.merge_with(end);
        let id = self.intern_expression(ASTExpression::ObjectExpression { name, fields });
        Ok(Spanned::new(id, span))
    }

    fn parse_object_expression(
        &mut self,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let name = self.parse_type(type_params)?;
        self.parse_object_expression_with_name(name, type_params)
    }

    ///Parses anything that comes prefixed by a identifier. This can be a function call, object creation, or a struct creation. This is executed without eating the identifier to be able to choose what to
    ///return
    pub fn parse_identifier_exprs(
        &mut self,
        type_params: TypeParamScope,
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
            TokenKind::LBrace if self.has_flag(crate::flags::ParserFlag::ComponentExpr) => {
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
        type_params: TypeParamScope,
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
                    let span = self.eat()?.span;
                    expr = self.parse_array_access(expr, span, type_params)?;
                }
                _ => break Ok(expr),
            }
        }
    }

    ///Parses a postfix that comes after a '.'. This function initializes right after the '.'
    pub fn parse_dot_postfix(
        &mut self,
        prefix: Spanned<DedupPoolId<ASTExpression>>,
        type_params: TypeParamScope,
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
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let current = self.peek()?;

        let expr = if let TokenKind::If = current.kind {
            let span = self.eat()?.span;
            self.parse_if(span, type_params)?
        } else if let TokenKind::Identifier(_) = self.peek()?.kind
            && let Some(value) = self.parse_identifier_exprs(type_params)?
        {
            value
        } else {
            let current = self.eat()?;
            match current.kind {
                TokenKind::Null => Ok(Spanned::new(
                    self.intern_expression(ASTExpression::Null),
                    current.span,
                )),
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
                    let first = self.parse_expression(type_params)?;
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

    /// Shared skeleton for the precedence cascade: parses a left-hand side with
    /// `lhs`, then folds consecutive `rhs` operands into a node as long as
    /// `next_op` keeps reporting an operator. `fold` builds the AST row from
    /// the operator (which is the unit type for operators without one, such as
    /// `matches`) and the two already-parsed operands.
    fn parse_infix<E>(
        &mut self,
        type_params: TypeParamScope,
        mut lhs: impl FnMut(&mut Self, TypeParamScope) -> Result<Spanned<DedupPoolId<ASTExpression>>>,
        mut rhs: impl FnMut(&mut Self, TypeParamScope) -> Result<Spanned<DedupPoolId<ASTExpression>>>,
        mut next_op: impl FnMut(&mut Self) -> Result<Option<E>>,
        mut fold: impl FnMut(
            E,
            Spanned<DedupPoolId<ASTExpression>>,
            Spanned<DedupPoolId<ASTExpression>>,
        ) -> ASTExpression,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let mut current = lhs(self, type_params)?;
        loop {
            let op = match next_op(self) {
                Ok(Some(op)) => op,
                Ok(None) => break,
                // The original loops stopped on end of input instead of failing.
                Err(ParseError::UnexpectedEndOfInput) => break,
                Err(err) => return Err(err),
            };
            let rhs = rhs(self, type_params)?;
            let span = current.span.merge_with(rhs.span);
            let id = self.intern_expression(fold(op, current, rhs));
            current = Spanned::new(id, span);
        }
        Ok(current)
    }

    /// Parses multiplicative expressions, which consist of primary expressions combined with multiplication '*' or division '/' operators. It handles operator precedence by first parsing the left-hand side (LHS) as a primary expression, and then repeatedly checking for multiplicative operators and parsing the right-hand side (RHS) as another primary expression until no more multiplicative operators are found.
    pub fn parse_multiplicative(
        &mut self,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.parse_infix(
            type_params,
            |parser, tps| parser.parse_primary(tps),
            |parser, tps| parser.parse_primary(tps),
            |parser| {
                let op = match parser.peek()?.kind {
                    TokenKind::Star => Operator::Star,
                    TokenKind::Slash => Operator::Slash,
                    _ => return Ok(None),
                };
                parser.eat()?;
                Ok(Some(op))
            },
            |op, lhs, rhs| ASTExpression::Binary { lhs, op, rhs },
        )
    }
    /// Parses additive expressions, which consist of multiplicative expressions combined with addition '+' or subtraction '-' operators. It handles operator precedence by first parsing the left-hand side (LHS) as a multiplicative expression, and then repeatedly checking for additive operators and parsing the right-hand side (RHS) as another multiplicative expression until no more additive operators are found.
    pub fn parse_additive(
        &mut self,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.parse_infix(
            type_params,
            |parser, tps| parser.parse_multiplicative(tps),
            |parser, tps| parser.parse_multiplicative(tps),
            |parser| {
                let op = match parser.peek()?.kind {
                    TokenKind::Plus => Operator::Add,
                    TokenKind::Sub => Operator::Sub,
                    _ => return Ok(None),
                };
                parser.eat()?;
                Ok(Some(op))
            },
            |op, lhs, rhs| ASTExpression::Binary { lhs, op, rhs },
        )
    }

    ///This function simply checks if the current and the next token are '>' which makes a '>>'
    pub fn is_shiftright(&self) -> Result<bool> {
        Ok(self.peek()?.kind == TokenKind::Gt && self.peek_at(1)?.kind == TokenKind::Gt)
    }

    ///Parses binary expressions, thus, anything that has a bit operator
    pub fn parse_bitoperation(
        &mut self,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.parse_infix(
            type_params,
            |parser, tps| parser.parse_additive(tps),
            |parser, tps| parser.parse_bitoperation(tps),
            |parser| {
                let op = match parser.peek()?.kind {
                    TokenKind::ShiftLeft => Operator::LeftShift,
                    TokenKind::BitAnd => Operator::And,
                    TokenKind::BitOr => Operator::Or,
                    TokenKind::Xor => Operator::Xor,
                    // The lexer has no single `>>` token: a right shift is two
                    // consecutive `>` tokens, both consumed here.
                    TokenKind::Gt if parser.is_shiftright()? => Operator::RightShift,
                    _ => return Ok(None),
                };
                parser.eat()?;
                if op == Operator::RightShift {
                    parser.eat()?;
                }
                Ok(Some(op))
            },
            |op, lhs, rhs| ASTExpression::Binary { lhs, op, rhs },
        )
    }

    ///Parses comparison expressions, thus, anything whose value returned is a boolean
    pub fn parse_comparison(
        &mut self,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.parse_infix(
            type_params,
            |parser, tps| parser.parse_bitoperation(tps),
            |parser, tps| parser.parse_bitoperation(tps),
            |parser| {
                let op = match parser.peek()?.kind {
                    TokenKind::EqEq => Operator::Equals,
                    TokenKind::Lt => Operator::LessThan,
                    TokenKind::Gt => Operator::GreaterThan,
                    TokenKind::LtEq => Operator::LessThanOrEqual,
                    TokenKind::GtEq => Operator::GreaterThanOrEqual,
                    _ => return Ok(None),
                };
                parser.eat()?;
                Ok(Some(op))
            },
            |op, lhs, rhs| ASTExpression::Binary { lhs, op, rhs },
        )
    }

    ///Parses `matches` expressions, i.e. `lhs matches Pattern`.
    ///
    /// `matches` binds more tightly than `&&`/`||` but looser than comparisons,
    /// so `a == b matches C && d` groups as `((a == b) matches C) && d`. The
    /// pattern on the right is parsed as a primary expression: a bare variant
    /// name, an associated variant reference `Some(4)`, or a struct variant
    /// reference `Foo { x: 1 }`.
    pub fn parse_match(
        &mut self,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.parse_infix(
            type_params,
            |parser, tps| parser.parse_comparison(tps),
            |parser, tps| parser.parse_primary(tps),
            |parser| {
                if parser.peek()?.kind == TokenKind::Matches {
                    parser.eat()?;
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            },
            |(), lhs, pattern| ASTExpression::Matches { lhs, pattern },
        )
    }

    ///Parses logical expressions, thus, anything whose value returned is a boolean
    pub fn parse_logical(
        &mut self,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.parse_infix(
            type_params,
            |parser, tps| parser.parse_match(tps),
            |parser, tps| parser.parse_match(tps),
            |parser| {
                let op = match parser.peek()?.kind {
                    TokenKind::And => Operator::LogicAnd,
                    TokenKind::Or => Operator::LogicOr,
                    _ => return Ok(None),
                };
                parser.eat()?;
                Ok(Some(op))
            },
            |op, lhs, rhs| ASTExpression::Binary { lhs, op, rhs },
        )
    }

    pub fn parse_array(
        &mut self,
        span: Span,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let exprs: SmallVec<[Spanned<DedupPoolId<ASTExpression>>; 2]> = self
            .parse_separated(TokenKind::RBracket, TokenKind::Comma, true, |parser| {
                parser.parse_expression(type_params)
            })?
            .into();
        let end = self.expect(&TokenKind::RBracket)?.span;
        let id = self.intern_expression(ASTExpression::Array(exprs));
        Ok(span.merge_with(end).make_spanned(id))
    }
    ///Parses a vector literal, which is delimited by `{` and `}`. Unlike array
    ///literals, the size of a vector is not known, so it is dynamic.
    pub fn parse_vector(
        &mut self,
        span: Span,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let exprs: SmallVec<[Spanned<DedupPoolId<ASTExpression>>; 2]> = self
            .parse_separated(TokenKind::RBrace, TokenKind::Comma, true, |parser| {
                parser.parse_expression(type_params)
            })?
            .into();
        let end = self.expect(&TokenKind::RBrace)?.span;
        let id = self.intern_expression(ASTExpression::Vector(exprs));
        Ok(span.merge_with(end).make_spanned(id))
    }
    ///Parses an array access on the given `arr_expression` expression. And the given ¯span` its the span of the left bracket.This function starts right after the '['. So a[5], this function starts looking up to '5'
    pub fn parse_array_access(
        &mut self,
        arr_expression: Spanned<DedupPoolId<ASTExpression>>,
        _: Span,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        match (self.peek()?.kind.clone(), self.peek_at(1)?.kind.clone()) {
            (TokenKind::Colon, TokenKind::RBracket) => {
                //parses [:]
                self.eat()?; //:
                let end = self.eat()?.span; //]
                let expr = self.intern_expression(ASTExpression::IndexExpression(
                    arr_expression,
                    RangeType::All,
                ));
                Ok(arr_expression.span.merge_with(end).make_spanned(expr))
            }
            (TokenKind::Colon, _) => {
                self.eat()?;
                let expr = self.parse_expression(type_params)?;
                let expr = self.intern_expression(ASTExpression::IndexExpression(
                    arr_expression,
                    RangeType::To(expr),
                ));
                let bracket_span = self.expect(&TokenKind::RBracket)?.span;
                Ok(arr_expression
                    .span
                    .merge_with(bracket_span)
                    .make_spanned(expr))
            }
            _ => {
                let expr = self.parse_expression(type_params)?;
                match (self.peek()?.kind.clone(), self.peek_at(1)?.kind.clone()) {
                    (TokenKind::Colon, TokenKind::RBracket) => {
                        self.eat()?;
                        let bracket_span = self.eat()?.span;
                        let expr = self.intern_expression(ASTExpression::IndexExpression(
                            arr_expression,
                            RangeType::From(expr),
                        ));
                        Ok(arr_expression
                            .span
                            .merge_with(bracket_span)
                            .make_spanned(expr))
                    }
                    (TokenKind::Colon, _) => {
                        self.eat()?;
                        let end_expr = self.parse_expression(type_params)?;
                        let bracket_span = self.eat()?.span;
                        let expr = self.intern_expression(ASTExpression::IndexExpression(
                            arr_expression,
                            RangeType::Normal {
                                from: expr,
                                to: end_expr,
                            },
                        ));
                        Ok(arr_expression
                            .span
                            .merge_with(bracket_span)
                            .make_spanned(expr))
                    }
                    (_, _) => {
                        let expr = self.intern_expression(ASTExpression::IndexExpression(
                            arr_expression,
                            RangeType::NoRange(expr),
                        ));
                        let bracket_span = self.expect(&TokenKind::RBracket)?.span;
                        Ok(arr_expression
                            .span
                            .merge_with(bracket_span)
                            .make_spanned(expr))
                    }
                }
            }
        }
    }

    /// Parses an expression, which is the top-level function for parsing any kind of expression. It starts by parsing a logical expression, which can include comparisons, additive, multiplicative, and primary expressions, and returns the resulting ASTExpression.
    pub fn parse_expression(
        &mut self,
        type_params: TypeParamScope,
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
                let expr = self.parse_expression(type_params)?;

                Ok(start
                    .merge_with(expr.span)
                    .make_spanned(self.intern_expression(ASTExpression::Deref(expr))))
            }
            TokenKind::BitAnd => {
                let start = self.eat()?.span;
                let (id, end) = if self.peek()?.kind == TokenKind::Mut {
                    self.expect(&TokenKind::Mut)?;
                    let expr = self.parse_expression(type_params)?;
                    (
                        self.intern_expression(ASTExpression::Reference {
                            mutable: true,
                            expr,
                        }),
                        expr.span,
                    )
                } else {
                    let expr = self.parse_expression(type_params)?;
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
            _ => self.parse_logical(type_params),
        }?;

        Ok(expr)
    }
}
