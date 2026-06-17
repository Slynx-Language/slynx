use crate::{Parser, Result, flags::ParserFlag};
use slynx_lexer::tokens::{Token, TokenKind};

use crate::ast::{ASTStatement, ASTStatementKind};
use common::Span;

impl Parser {
    ///Parses a let Statement. Until now it's only for variable declaration, so, this only parses 'let name: t = value;' or 'let name = value;', same for mut variants
    ///Maybe, in the future, more things will be parsed.
    ///Obs: this function should initialize right after 'let' token, and the `letstan` is the span of the 'let' token
    pub fn parse_let_statement(&mut self, letspan: Span) -> Result<ASTStatement> {
        self.add_flag(ParserFlag::RequireSemicolon);
        let mut mutable = false;
        if let TokenKind::Mut = self.peek()?.kind {
            self.eat()?;
            mutable = true;
        }
        let Token {
            kind: TokenKind::Identifier(name),
            ..
        } = self.expect(&TokenKind::Identifier("".into()))?
        else {
            unreachable!();
        };
        let vartype = match self.peek()?.kind {
            TokenKind::Colon => {
                self.eat()?;
                Some(self.parse_type()?)
            }
            _ => None,
        };
        self.eat()?; //eat '='
        let rhs = self.parse_expression()?;
        Ok(ASTStatement {
            span: Span {
                start: letspan.start,
                end: rhs.span.end,
            },
            kind: if mutable {
                ASTStatementKind::MutableVar {
                    name,
                    ty: vartype,
                    rhs,
                }
            } else {
                ASTStatementKind::Var {
                    name,
                    ty: vartype,
                    rhs,
                }
            },
        })
    }

    pub fn parse_while_statement(&mut self, span: Span) -> Result<ASTStatement> {
        self.reset_flags();

        let condition = self.parse_expression()?;

        let (body, block_span) = self.parse_block()?;

        // 3. AST
        Ok(ASTStatement {
            span: Span {
                start: span.start,   // start "while"
                end: block_span.end, // end "}"
            },
            kind: ASTStatementKind::While { condition, body },
        })
    }

    pub fn parse_statement(&mut self) -> Result<ASTStatement> {
        match self.peek()?.kind {
            TokenKind::Let => {
                let span = self.eat()?.span;
                self.parse_let_statement(span)
            }

            TokenKind::While => {
                let span = self.eat()?.span; //Consume "While"
                self.parse_while_statement(span)
            }

            _ => {
                let expr = self.parse_expression()?;
                self.add_flag(ParserFlag::RequireSemicolon);
                if matches!(self.peek()?.kind, TokenKind::Eq) && expr.is_assignable() {
                    self.eat()?;
                    let rhs = self.parse_expression()?;
                    Ok(ASTStatement {
                        span: Span {
                            start: expr.span.start,
                            end: rhs.span.end,
                        },
                        kind: ASTStatementKind::Assign { lhs: expr, rhs },
                    })
                } else {
                    Ok(ASTStatement {
                        span: expr.span,
                        kind: ASTStatementKind::Expression(expr),
                    })
                }
            }
        }
    }
}
