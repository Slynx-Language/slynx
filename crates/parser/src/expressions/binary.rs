use common::{Operator, Spanned, pool::DedupPoolId};
use slynx_lexer::TokenKind;

use crate::{
    ASTExpression, ParseError, ParseErrorKind, Parser, Result, SymbolPointer, flags::ParserFlags,
};
impl<'a> Parser<'a> {
    /// Shared skeleton for the precedence cascade: parses a left-hand side with
    /// `lhs`, then folds consecutive `rhs` operands into a node as long as
    /// `next_op` keeps reporting an operator. `fold` builds the AST row from
    /// the operator (which is the unit type for operators without one, such as
    /// `matches`) and the two already-parsed operands.
    fn parse_infix<E>(
        &mut self,
        type_params: &[SymbolPointer],
        mut lhs: impl FnMut(&mut Self, &[SymbolPointer]) -> Result<Spanned<DedupPoolId<ASTExpression>>>,
        mut rhs: impl FnMut(&mut Self, &[SymbolPointer]) -> Result<Spanned<DedupPoolId<ASTExpression>>>,
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
                Err(ParseError {
                    kind: ParseErrorKind::UnexpectedEndOfInput,
                    ..
                }) => break,
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
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.parse_infix(
            type_params,
            |parser, tps| parser.parse_primary(tps, flags),
            |parser, tps| parser.parse_primary(tps, flags),
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
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.parse_infix(
            type_params,
            |parser, tps| parser.parse_multiplicative(tps, flags),
            |parser, tps| parser.parse_multiplicative(tps, flags),
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
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.parse_infix(
            type_params,
            |parser, tps| parser.parse_additive(tps, flags),
            |parser, tps| parser.parse_bitoperation(tps, flags),
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
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.parse_infix(
            type_params,
            |parser, tps| parser.parse_bitoperation(tps, flags),
            |parser, tps| parser.parse_bitoperation(tps, flags),
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
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.parse_infix(
            type_params,
            |parser, tps| parser.parse_comparison(tps, flags),
            |parser, tps| parser.parse_primary(tps, flags),
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
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.parse_infix(
            type_params,
            |parser, tps| parser.parse_match(tps, flags),
            |parser, tps| parser.parse_match(tps, flags),
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
}
