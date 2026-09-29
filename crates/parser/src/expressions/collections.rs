use common::{Span, Spanned, pool::DedupPoolId};
use slynx_lexer::TokenKind;
use smallvec::SmallVec;

use crate::{ASTExpression, Parser, RangeType, Result, SymbolPointer, flags::ParserFlags};

impl Parser<'_> {
    pub fn parse_array(
        &mut self,
        span: Span,
        type_params: &[SymbolPointer],
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let exprs: SmallVec<[Spanned<DedupPoolId<ASTExpression>>; 2]> = self
            .parse_separated(TokenKind::RBracket, TokenKind::Comma, true, |parser| {
                parser.parse_expression(type_params, ParserFlags::default())
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
        type_params: &[SymbolPointer],
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let exprs: SmallVec<[Spanned<DedupPoolId<ASTExpression>>; 2]> = self
            .parse_separated(TokenKind::RBrace, TokenKind::Comma, true, |parser| {
                parser.parse_expression(type_params, ParserFlags::default())
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
        type_params: &[SymbolPointer],
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
                let expr = self.parse_expression(type_params, ParserFlags::default())?;
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
                let expr = self.parse_expression(type_params, ParserFlags::default())?;
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
                        let end_expr =
                            self.parse_expression(type_params, ParserFlags::default())?;
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
}
