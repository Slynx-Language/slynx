use crate::{ASTExpression, ASTStatement, Parser, Result};
use common::{Span, Spanned, pool::DedupPoolId};
use slynx_lexer::tokens::TokenKind;

impl Parser<'_> {
    /// Parses an if statement. The provided `span` is the initial span for the 'if' keyword.
    pub fn parse_if(&mut self, span: Span) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.flags.reset();

        let condition = self.parse_without_component_expr(Self::parse_expression)?;
        let (body, block_span) = self.parse_block()?;

        let (else_body, end) = match self.peek()?.kind {
            TokenKind::Else if self.peek_at(1)?.kind == TokenKind::If => {
                self.eat()?;

                let if_span = self.eat()?.span;
                let expr = self.parse_if(if_span)?;
                let end = expr.span;

                let span = expr.span;
                let id = self.intern_statment(ASTStatement::Expression(expr));
                (vec![Spanned::new(id, span)], end)
            }
            TokenKind::Else => {
                self.eat()?;
                self.parse_block()?
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

    pub fn parse_block(&mut self) -> Result<(Vec<Spanned<DedupPoolId<ASTStatement>>>, Span)> {
        self.flags.reset();
        let lbrace = self.expect(&TokenKind::LBrace)?;
        let start = lbrace.span.start;
        let mut body = Vec::new();
        while !matches!(self.peek()?.kind, TokenKind::RBrace) {
            let stmt = self.parse_statement()?;
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
            self.finish_current_parse()?;
        }
        let rbrace = self.expect(&TokenKind::RBrace)?;
        let end = rbrace.span.end;
        Ok((body, Span { start, end }))
    }
}
