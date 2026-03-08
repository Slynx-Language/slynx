use color_eyre::eyre::Result;
use common::{ASTStatement, ASTStatementKind, Span};

use crate::{lexer::tokens::TokenKind, parser::Parser};

impl Parser {
    /// Parses an if statement. The provided `span` is the initial span for the 'if' keyword.
    pub fn parse_if(&mut self, span: Span) -> Result<ASTStatement> {
        self.set_flags(super::ParserFlags::None);
        let condition = self.parse_expression()?;
        let (body, block_span) = self.parse_block()?;
        Ok(ASTStatement {
            span: Span {
                start: span.start,
                end: block_span.end,
            },
            kind: ASTStatementKind::If {
                condition: Box::new(condition),
                body,
            },
        })
    }

    /// Parses an else statement. The provided `span` is the initial span for the 'else' keyword.
    pub fn parse_else(&mut self, span: Span) -> Result<ASTStatement> {
        self.set_flags(super::ParserFlags::None);
        let (body, block_span) = self.parse_block()?;
        Ok(ASTStatement {
            span: Span {
                start: span.start,
                end: block_span.end,
            },
            kind: ASTStatementKind::Else { body },
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
                    match  &body.last().unwrap().kind {
                        ASTStatementKind::If { .. } | ASTStatementKind::Else { .. } => {
                            continue;
                        }
                        _ => {}
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