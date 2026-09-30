use crate::{ASTExpression, ASTStatement, Parser, Result, flags::ParserFlags};
use common::{Span, Spanned, pool::DedupPoolId};

use slynx_lexer::TokenKind;

use crate::SymbolPointer;

impl Parser<'_> {
    /// Parses an if statement. The provided `span` is the initial span for the 'if' keyword.
    /// `flags` is the context of the expression the `if` itself is part of; it is
    /// forwarded to the block, but the condition is parsed with
    /// [`ParserFlags::COMPONENT_EXPR`] removed.
    pub fn parse_if(
        &mut self,
        span: Span,
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        // The condition must not allow component literals: `if x matches
        // Variant { field: 1 }` would otherwise read the struct-variant pattern
        // as a component expression and swallow the block that follows it.
        let condition =
            self.parse_expression(type_params, flags - ParserFlags::COMPONENT_EXPR)?;
        let (body, block_span) = self.parse_block(type_params, flags)?;

        let (else_body, end) = match self.peek()?.kind {
            TokenKind::Else if self.peek_at(1)?.kind == TokenKind::If => {
                self.eat()?;

                let if_span = self.eat()?.span;
                let expr = self.parse_if(if_span, type_params, flags)?;
                let end = expr.span;

                let span = expr.span;
                let id = self.intern_statement(ASTStatement::Expression(expr));
                (vec![Spanned::new(id, span)], end)
            }
            TokenKind::Else => {
                self.eat()?;
                self.parse_block(type_params, flags)?
            }
            _ => (vec![], block_span),
        };
        let id = self.intern_expression(ASTExpression::If {
            condition,
            body,
            else_body,
        });
        Ok(Spanned::new(id, span.merge_with(end)))
    }

    pub fn parse_block(
        &mut self,
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<(Vec<Spanned<DedupPoolId<ASTStatement>>>, Span)> {
        let lbrace = self.expect(&TokenKind::LBrace)?;
        let start = lbrace.span.start;
        let mut body = Vec::new();
        while !matches!(self.peek()?.kind, TokenKind::RBrace) {
            let stmt = self.parse_statement(type_params, flags)?;
            body.push(stmt);
            if let Some(ASTStatement::Expression(expr)) =
                body.last().map(|stmt| self.statements.get(stmt.data))
                && let ASTExpression::If { .. } = self.expressions.get(expr.data)
            {
                continue;
            }

            if self.peek()?.kind == TokenKind::RBrace {
                continue;
            }
            // Statements are always ';'-terminated inside a block, no matter which
            // flags the block itself was entered with.
            self.finish_current_parse(ParserFlags::REQUIRE_SEMICOLON)?;
        }
        let rbrace = self.expect(&TokenKind::RBrace)?;
        let end = rbrace.span.end;
        Ok((body, Span { start, end }))
    }
}
