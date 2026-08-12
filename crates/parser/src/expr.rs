use crate::{
    ASTExpression, ComponentExpression, ComponentMemberValue, ExpectedContent, NamedExpr,
    RangeType, Type, TypeParamScope,
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
        let mut params = SmallVec::new();
        if self.peek()?.kind == TokenKind::RParen {
            let Token { span: last, .. } = self.expect(&TokenKind::RParen)?;
            let span = identifier.span.merge_with(last);
            let id = self.intern_expression(ASTExpression::FunctionCall {
                name: identifier,
                args: params,
            });
            return Ok(Spanned { data: id, span });
        }
        loop {
            let param = self.parse_expression(type_params)?;
            params.push(param);
            match self.peek()?.kind {
                TokenKind::RParen => break,
                TokenKind::Comma => {
                    self.eat()?;
                }
                _ => {
                    return Err(ParseError::UnexpectedToken(
                        self.eat()?,
                        ExpectedContent::Raw("Was expecting an ','".to_string()),
                    ));
                }
            }
        }
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
                    let (ident, _) = self.expect_identifier()?;
                    self.expect(&TokenKind::Colon)?;
                    let val = self.parse_expression(type_params)?;
                    values.push(ComponentMemberValue::Assign {
                        prop_name: ident,
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
        let (name, start) = self.expect_identifier()?;
        self.expect(&TokenKind::Colon)?;
        let expr = self.parse_expression(type_params)?;
        let span = start.merge_with(expr.span);
        Ok(Spanned::new(NamedExpr { name, expr }, span))
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
    ///Parses a tuple expression, which follows the rule (expr, expr, expr) or ()
    pub fn parse_tupleparse_tuple_with_first(
        &mut self,
        start_span: &Span,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        if self.peek()?.kind == TokenKind::RParen {
            let end = self.eat()?;
            let id = self.intern_expression(ASTExpression::Tuple(smallvec![]));
            return Ok(Spanned::new(id, start_span.merge_with(end.span)));
        }

        let first = self.parse_expression(type_params)?;
        if self.peek()?.kind == TokenKind::RParen {
            let _ = self.eat()?;
            return Ok(first);
        }
        self.expect(&TokenKind::Comma)?;
        let mut items = smallvec![first];
        while self.peek()?.kind != TokenKind::RParen {
            items.push(self.parse_expression(type_params)?);
            if self.peek()?.kind == TokenKind::Comma {
                self.eat()?;
            }
        }
        let end = self.expect(&TokenKind::RParen)?.span;
        let span = start_span.merge_with(end);
        let id = self.intern_expression(ASTExpression::Tuple(items));
        Ok(Spanned { data: id, span })
    }

    ///Parses an object expression, which follows the rule Object(field: expr, field: value)
    pub fn parse_object_expression(
        &mut self,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let name = self.parse_type(type_params)?;
        self.expect(&TokenKind::LParen)?;
        let mut fields = SmallVec::new();
        while self.peek()?.kind != TokenKind::RParen {
            let named_expr = self.parse_named_expr(type_params)?;
            fields.push(named_expr);
            if let TokenKind::RParen = self.peek()?.kind {
                break;
            } else {
                self.expect(&TokenKind::Comma)?;
            }
        }
        let end = self.expect(&TokenKind::RParen)?.span;
        let span = name.span.merge_with(end);
        let id = self.intern_expression(ASTExpression::ObjectExpression { name, fields });
        Ok(Spanned::new(id, span))
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
                    TokenKind::LParen => Ok(Some(self.parse_funcall_args(ty, type_params)?)),
                    TokenKind::LBrace => {
                        let component = self.parse_component_expr_with_name(ty, type_params)?;
                        let span = component.span;
                        let id = self.intern_expression(ASTExpression::Component(component.data));
                        Ok(Some(Spanned::new(id, span)))
                    }
                    _ => Err(ParseError::UnexpectedToken(
                        self.eat()?,
                        ExpectedContent::Raw(
                            "Instead was expecting '(' or '{' after a generic name".to_string(),
                        ),
                    )),
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
                let (ident, span) = self.expect_identifier()?;
                let field = Spanned::new(
                    self.intern_expression(ASTExpression::Identifier(ident)),
                    span,
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

                _ => Err(ParseError::UnexpectedToken(
                    current,
                    ExpectedContent::Raw("Was expecting an expression".to_string()),
                )),
            }?
        };

        self.parse_postfix_chain(expr, type_params)
    }

    /// Parses multiplicative expressions, which consist of primary expressions combined with multiplication '*' or division '/' operators. It handles operator precedence by first parsing the left-hand side (LHS) as a primary expression, and then repeatedly checking for multiplicative operators and parsing the right-hand side (RHS) as another primary expression until no more multiplicative operators are found.
    pub fn parse_multiplicative(
        &mut self,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let mut lhs = self.parse_primary(type_params)?;
        while let Ok(curr) = self.peek()
            && matches!(curr.kind, TokenKind::Star | TokenKind::Slash)
        {
            let op = if let TokenKind::Star = self.eat()?.kind {
                Operator::Star
            } else {
                Operator::Slash
            };
            let rhs = self.parse_primary(type_params)?;
            let span = lhs.span.merge_with(rhs.span);
            lhs = Spanned::new(
                self.intern_expression(ASTExpression::Binary { lhs, op, rhs }),
                span,
            );
        }
        Ok(lhs)
    }
    /// Parses additive expressions, which consist of multiplicative expressions combined with addition '+' or subtraction '-' operators. It handles operator precedence by first parsing the left-hand side (LHS) as a multiplicative expression, and then repeatedly checking for additive operators and parsing the right-hand side (RHS) as another multiplicative expression until no more additive operators are found.
    pub fn parse_additive(
        &mut self,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let mut lhs = self.parse_multiplicative(type_params)?;
        while let Ok(curr) = self.peek()
            && matches!(curr.kind, TokenKind::Plus | TokenKind::Sub)
        {
            let op = if let TokenKind::Plus = self.eat()?.kind {
                Operator::Add
            } else {
                Operator::Sub
            };
            let rhs = self.parse_multiplicative(type_params)?;
            let span = lhs.span.merge_with(rhs.span);
            lhs = Spanned::new(
                self.intern_expression(ASTExpression::Binary { lhs, op, rhs }),
                span,
            );
        }
        Ok(lhs)
    }

    ///Parses binary expressions, thus, anything that has a bit operator
    pub fn parse_bitoperation(
        &mut self,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let mut lhs = self.parse_additive(type_params)?;
        while let Ok(curr) = self.peek()
            && matches!(
                curr.kind,
                TokenKind::ShiftRight
                    | TokenKind::ShiftLeft
                    | TokenKind::BitAnd
                    | TokenKind::BitOr
                    | TokenKind::Xor
            )
        {
            let op = match self.eat()?.kind {
                TokenKind::ShiftRight => Operator::RightShift,
                TokenKind::ShiftLeft => Operator::LeftShift,
                TokenKind::BitAnd => Operator::And,
                TokenKind::BitOr => Operator::Or,
                TokenKind::Xor => Operator::Xor,
                _ => unreachable!(),
            };
            let rhs = self.parse_bitoperation(type_params)?;
            let span = Span {
                start: lhs.span.start,
                end: rhs.span.end,
            };
            lhs = Spanned::new(
                self.intern_expression(ASTExpression::Binary { lhs, op, rhs }),
                span,
            );
        }
        Ok(lhs)
    }

    ///Parses comparison expressions, thus, anything whose value returned is a boolean
    pub fn parse_comparison(
        &mut self,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let mut lhs = self.parse_bitoperation(type_params)?;
        while let Ok(curr) = self.peek()
            && matches!(
                curr.kind,
                TokenKind::Gt | TokenKind::GtEq | TokenKind::Lt | TokenKind::LtEq | TokenKind::EqEq
            )
        {
            let op = match self.eat()?.kind {
                TokenKind::EqEq => Operator::Equals,
                TokenKind::Lt => Operator::LessThan,
                TokenKind::Gt => Operator::GreaterThan,
                TokenKind::LtEq => Operator::LessThanOrEqual,
                TokenKind::GtEq => Operator::GreaterThanOrEqual,
                _ => unreachable!(),
            };
            let rhs = self.parse_bitoperation(type_params)?;
            let span = Span {
                start: lhs.span.start,
                end: rhs.span.end,
            };
            lhs = Spanned::new(
                self.intern_expression(ASTExpression::Binary { lhs, op, rhs }),
                span,
            );
        }
        Ok(lhs)
    }

    ///Parses logical expressions, thus, anything whose value returned is a boolean
    pub fn parse_logical(
        &mut self,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let mut lhs = self.parse_comparison(type_params)?;
        while let Ok(curr) = self.peek()
            && matches!(curr.kind, TokenKind::And | TokenKind::Or)
        {
            let op = match self.eat()?.kind {
                TokenKind::And => Operator::LogicAnd,
                TokenKind::Or => Operator::LogicOr,
                _ => unreachable!(),
            };
            let rhs = self.parse_comparison(type_params)?;
            let span = Span {
                start: lhs.span.start,
                end: rhs.span.end,
            };
            lhs = Spanned::new(
                self.intern_expression(ASTExpression::Binary { lhs, op, rhs }),
                span,
            );
        }
        Ok(lhs)
    }

    pub fn parse_array(
        &mut self,
        span: Span,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let mut exprs = SmallVec::new();
        loop {
            if self.peek()?.kind == TokenKind::RBracket {
                break;
            }
            let expr = self.parse_expression(type_params)?;
            exprs.push(expr);
            if self.peek()?.kind != TokenKind::RBracket {
                self.expect(&TokenKind::Comma)?;
            }
        }
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
        let mut exprs = SmallVec::new();
        loop {
            if self.peek()?.kind == TokenKind::RBrace {
                break;
            }
            let expr = self.parse_expression(type_params)?;
            exprs.push(expr);
            if self.peek()?.kind != TokenKind::RBrace {
                self.expect(&TokenKind::Comma)?;
            }
        }
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
            _ => self.parse_logical(type_params),
        }?;

        Ok(expr)
    }
}
