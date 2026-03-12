use color_eyre::eyre::Result;

use crate::lexer::tokens::{Token, TokenKind};
use crate::parser::Parser;

use common::ast::{ASTStatement, ASTStatementKind, Span};

impl Parser {
    ///Parses a let Statement. Until now it's only for variable declaration, so, this only parses 'let name: t = value;' or 'let name = value;', same for mut variants
    ///Maybe, in the future, more things will be parsed.
    ///Obs: this function should initialize right after 'let' token, and the `letstan` the span of the 'let' token
    pub fn parse_let_statement(&mut self, letspan: Span) -> Result<ASTStatement> {
        self.set_flags(super::ParserFlags::RequireSemicolon);
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

    pub fn parse_statement(&mut self) -> Result<ASTStatement> {
        match self.peek()?.kind {
            TokenKind::Let => {
                let span = self.eat()?.span;
                self.parse_let_statement(span)
            }
            TokenKind::If => {
                let span = self.eat()?.span;

                self.parse_if(span)
            }
            _ => {
                let expr = self.parse_expression()?;
                if matches!(self.peek()?.kind, TokenKind::Eq) && expr.is_assignable() {
                    self.set_flags(super::ParserFlags::RequireSemicolon);
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
                        span: expr.span.clone(),
                        kind: ASTStatementKind::Expression(expr),
                    })
                }
            }
        }
    }
}
