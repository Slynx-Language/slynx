use color_eyre::eyre::Result;
use common::{ASTExpression, ASTExpressionKind, ASTStatement, ASTStatementKind, Span};

use crate::{lexer::tokens::TokenKind, parser::Parser};

impl Parser {
    /// Parses an if statement. The provided `span` is the initial span for the 'if' keyword.
    pub fn parse_if(&mut self, span: Span) -> Result<ASTExpression> {
        self.set_flags(super::ParserFlags::None);

        let condition = self.parse_without_component_expr(Self::parse_expression)?;
        let (body, block_span) = self.parse_block()?;

        let (else_body, end) = if self.peek()?.kind == TokenKind::Else {
            self.eat()?;

            if self.peek()?.kind == TokenKind::If {
                let if_span = self.eat()?.span;
                let expr = self.parse_if(if_span)?;
                let end = expr.span.end;

                let span = expr.span;

                (
                    Some(vec![ASTStatement {
                        span,
                        kind: ASTStatementKind::Expression(expr),
                    }]),
                    end,
                )
            } else {
                let (eb, es) = self.parse_block()?;
                (Some(eb), es.end)
            }
        } else {
            (None, block_span.end)
        };

        Ok(ASTExpression {
            span: Span {
                start: span.start,
                end,
            },
            kind: ASTExpressionKind::If {
                condition: Box::new(condition),
                body,
                else_body,
            },
        })
    }

    pub fn parse_block(&mut self) -> Result<(Vec<ASTStatement>, Span)> {
        self.set_flags(super::ParserFlags::None);
        let lbrace = self.expect(&TokenKind::LBrace)?;
        let start = lbrace.span.start;
        let mut body = Vec::new();
        while !matches!(self.peek()?.kind, TokenKind::RBrace) {
            let stmt = self.parse_statement()?;
            body.push(stmt);
            if let ASTStatementKind::Expression(expr) = &body.last().unwrap().kind
                && matches!(expr.kind, ASTExpressionKind::If { .. })
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
