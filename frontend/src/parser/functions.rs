use color_eyre::eyre::Result;

use crate::lexer::tokens::{Token, TokenKind};
use crate::parser::{Parser, error::ParseError};

use common::ast::{
    ASTDeclaration, ASTDeclarationKind, ASTStatement, ASTStatementKind, Span, TypedName,
};
impl Parser {
    ///Parses a typed name. A typed name is `name: type`, which is a name that contains a type
    pub fn parse_typedname(&mut self) -> Result<TypedName> {
        let Token {
            kind: TokenKind::Identifier(name),
            span,
        } = self.expect(&TokenKind::Identifier(String::new()))?
        else {
            unreachable!();
        };
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        Ok(TypedName {
            name,
            span: Span {
                start: span.start,
                end: ty.span.end,
            },
            kind: ty,
        })
    }

    ///Parses the arguments of a function. It parses until the `)` of the function args.
    pub fn parse_args(&mut self) -> Result<Vec<TypedName>> {
        let mut names = Vec::new();
        while !matches!(self.peek()?.kind, TokenKind::RParen) {
            names.push(self.parse_typedname()?);
            if matches!(self.peek()?.kind, TokenKind::RParen) {
                break;
            } else {
                self.expect(&TokenKind::Comma)?;
            }
        }

        Ok(names)
    }

    ///Parses a function. The provided `span` is the initial span for the 'func' keyword.
    ///Parses both `func main(arg1:T): Q {...}` and `func main(arg1:T): Q -> ...`
    pub fn parse_func(&mut self, span: Span) -> Result<ASTDeclaration> {
        let name = self.parse_type()?;
        self.expect(&TokenKind::LParen)?;
        let args = self.parse_args()?;
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::Colon)?;
        let return_type = self.parse_type()?;
        let current = self.eat()?;
        //func main(arg:T):Q ->/{}
        match current.kind {
            TokenKind::Arrow => {
                let expr = self.parse_expression()?;
                let end = self.expect(&TokenKind::SemiColon)?.span.end;
                Ok(ASTDeclaration {
                    span: Span {
                        start: span.start,
                        end,
                    },
                    kind: ASTDeclarationKind::FuncDeclaration {
                        name,
                        args,
                        return_type,
                        body: vec![ASTStatement {
                            span: expr.span.clone(),
                            kind: ASTStatementKind::Expression(expr),
                        }],
                    },
                })
            }
            TokenKind::LBrace => {
                let mut body = Vec::new();
                while !matches!(self.peek()?.kind, TokenKind::RBrace) {
                    body.push(self.parse_statement()?);
                    if self.peek()?.kind == TokenKind::RBrace {
                        continue;
                    }
                    self.expect(&TokenKind::SemiColon)?;
                }
                let end = self.expect(&TokenKind::RBrace)?.span.end;
                Ok(ASTDeclaration {
                    span: Span {
                        start: span.start,
                        end,
                    },
                    kind: ASTDeclarationKind::FuncDeclaration {
                        name,
                        args,
                        return_type,
                        body,
                    },
                })
            }
            _ => Err(ParseError::UnexpectedToken(current, "'->' or '{'".to_string()).into()),
        }
    }
}
