use common::{Span, Spanned, pool::DedupPoolId};
use slynx_lexer::TokenKind;
use smallvec::SmallVec;

use crate::{ASTExpression, Parser, RangeType, Result, SymbolPointer, flags::ParserFlags};

impl Parser<'_> {
    pub fn parse_array(
        &mut self,
        span: Span,
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let exprs: SmallVec<[Spanned<DedupPoolId<ASTExpression>>; 2]> = self
            .parse_separated(TokenKind::RBracket, TokenKind::Comma, true, |parser| {
                parser.parse_expression(type_params, flags)
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
        flags: ParserFlags,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let exprs: SmallVec<[Spanned<DedupPoolId<ASTExpression>>; 2]> = self
            .parse_separated(TokenKind::RBrace, TokenKind::Comma, true, |parser| {
                parser.parse_expression(type_params, flags)
            })?
            .into();
        let end = self.expect(&TokenKind::RBrace)?.span;
        let id = self.intern_expression(ASTExpression::Vector(exprs));
        Ok(span.merge_with(end).make_spanned(id))
    }
    ///Parses an index access on the given `arr_expression` expression. The given
    ///`span` is the span of the left bracket. This function starts right after the
    ///'[', so for `a[5]` it starts looking at the `5`; the caller is responsible
    ///for consuming the bracket. `flags` are the ones of the surrounding
    ///expression and are used for the index/range sub-expressions.
    pub fn parse_array_access(
        &mut self,
        arr_expression: Spanned<DedupPoolId<ASTExpression>>,
        span: Span,
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        // The second token is looked up leniently: an index access at the very end
        // of the input has no following token, and that must not fail the parse.
        match (
            self.peek()?.kind.clone(),
            self.peek_at_opt(1).map(|token| token.kind.clone()),
        ) {
            (TokenKind::Colon, Some(TokenKind::RBracket)) => {
                //parses [:]
                self.eat()?; //:
                let end = self.eat()?.span; //]
                let expr = self.intern_expression(ASTExpression::IndexExpression(
                    arr_expression,
                    RangeType::All,
                ));
                Ok(span.merge_with(end).make_spanned(expr))
            }
            (TokenKind::Colon, _) => {
                //parses [:expr]
                self.eat()?;
                let expr = self.parse_expression(type_params, flags)?;
                let expr = self.intern_expression(ASTExpression::IndexExpression(
                    arr_expression,
                    RangeType::To(expr),
                ));
                let bracket_end = self.expect(&TokenKind::RBracket)?.span;
                Ok(span.merge_with(bracket_end).make_spanned(expr))
            }
            _ => {
                let expr = self.parse_expression(type_params, flags)?;
                match (
                    self.peek()?.kind.clone(),
                    self.peek_at_opt(1).map(|token| token.kind.clone()),
                ) {
                    (TokenKind::Colon, Some(TokenKind::RBracket)) => {
                        self.eat()?;
                        let bracket_end = self.eat()?.span;
                        let expr = self.intern_expression(ASTExpression::IndexExpression(
                            arr_expression,
                            RangeType::From(expr),
                        ));
                        Ok(span.merge_with(bracket_end).make_spanned(expr))
                    }
                    (TokenKind::Colon, _) => {
                        self.eat()?;
                        let end_expr = self.parse_expression(type_params, flags)?;
                        let bracket_end = self.expect(&TokenKind::RBracket)?.span;
                        let expr = self.intern_expression(ASTExpression::IndexExpression(
                            arr_expression,
                            RangeType::Normal {
                                from: expr,
                                to: end_expr,
                            },
                        ));
                        Ok(span.merge_with(bracket_end).make_spanned(expr))
                    }
                    (_, _) => {
                        let expr = self.intern_expression(ASTExpression::IndexExpression(
                            arr_expression,
                            RangeType::NoRange(expr),
                        ));
                        let bracket_end = self.expect(&TokenKind::RBracket)?.span;
                        Ok(span.merge_with(bracket_end).make_spanned(expr))
                    }
                }
            }
        }
    }
}
