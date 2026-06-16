//! Module idealized for parsing general things related to declarations, such as visibility qualifiers, and attributes

use common::{Span, VisibilityModifier};
use slynx_lexer::{Token, TokenKind};

use crate::{
    ASTAttribute, ASTDeclaration, ASTDeclarationKind, ExpectedContent, ParseError, Parser, Result,
    flags::ParserFlag,
};

impl Parser {
    pub fn parse_static(&mut self, span: Span) -> Result<ASTDeclaration> {
        let TokenKind::Identifier(name) = self.expect_identifier()?.kind else {
            unreachable!()
        };
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let expr = if self.flags.has_flag(ParserFlag::OnlySignatures) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        Ok(ASTDeclaration {
            attributes: vec![],
            external: false,
            span: span.merge_with(expr.as_ref().map(|expr| expr.span).unwrap_or(ty.span)),
            visibility: VisibilityModifier::Private,
            kind: ASTDeclarationKind::Static {
                name,
                ty,
                value: expr,
            },
        })
    }

    pub fn parse_attributes(&mut self) -> Result<Vec<ASTAttribute>> {
        let mut out = Vec::new();
        while let Ok(Token {
            kind: TokenKind::Identifier(_),
            ..
        }) = self.peek()
        {
            let TokenKind::Identifier(name) = self.expect_identifier()?.kind else {
                unreachable!();
            };
            self.expect(&TokenKind::LParen)?;
            let args = {
                let mut args = Vec::new();
                loop {
                    if self.peek()?.kind == TokenKind::RParen {
                        break args;
                    }
                    let TokenKind::String(arg) = self.expect_string()?.kind else {
                        unreachable!()
                    };
                    args.push(arg);
                    if self.peek()?.kind == TokenKind::Comma {
                        self.eat()?;
                    }
                }
            };
            self.expect(&TokenKind::RParen)?;
            out.push(ASTAttribute { name, args });
        }
        Ok(out)
    }

    ///Parses a single declaration
    fn parse_declaration(&mut self) -> Result<ASTDeclaration> {
        let token = self.peek()?;
        let visibility = if matches!(token.kind, TokenKind::Pub) {
            self.eat()?;
            VisibilityModifier::Public
        } else {
            VisibilityModifier::Private
        };
        let mut out = match &self.peek()?.kind {
            TokenKind::Import => {
                let span = self.eat()?.span;
                self.parse_import(span)
            }
            TokenKind::Alias => {
                let Token { span, .. } = self.eat()?;
                self.parse_alias(span)
            }
            TokenKind::Object => {
                let Token { span, .. } = self.eat()?;
                self.parse_object(span)
            }
            TokenKind::Component => {
                let Token { span, .. } = self.eat()?;
                self.parse_component(span)
            }
            TokenKind::Func => {
                let Token { span, .. } = self.eat()?;
                self.parse_func(span)
            }
            TokenKind::StyleSheet => {
                let Token { span, .. } = self.eat()?;
                self.parse_stylesheet(span)
            }
            TokenKind::Static => {
                let Token { span, .. } = self.eat()?;
                self.parse_static(span)
            }
            _ => {
                return Err(ParseError::UnexpectedToken(
                    self.eat()?,
                    ExpectedContent::Raw(
                        "Unknown declaration that starts with it. Expected some valid declaration"
                            .to_owned(),
                    ),
                ));
            }
        }?;
        out.visibility = visibility;
        Ok(out)
    }

    ///Parse extern declarations and insert them on the given `declarations`. Returns the amount of declarations parsed. The main reason for this to not return a new Vec<> is to simply not allocate on a separated vector
    /// and then need to copy/move all the data to the correct vector
    fn parse_externs(&mut self, declarations: &mut Vec<ASTDeclaration>) -> Result<usize> {
        self.expect(&TokenKind::LBrace)?;
        self.add_flag(ParserFlag::OnlySignatures);
        let mut amount_parsed = 0;
        loop {
            if self.peek()?.kind == TokenKind::RBrace {
                self.eat()?;
                self.remove_flag(ParserFlag::OnlySignatures);
                break Ok(amount_parsed);
            }
            let mut declaration = self.parse_declaration()?;
            declaration.external = true;
            declarations.push(declaration);
            amount_parsed += 1;
        }
    }

    /// Parses the declarations in the source code and returns them as a vector of `ASTDeclaration`s.
    /// The parser will continue parsing until it reaches the end of the input stream.
    /// If it encounters an unexpected token, it will return an error indicating the expected token type.
    pub fn parse_declarations(&mut self) -> Result<Vec<ASTDeclaration>> {
        let mut out = Vec::new();
        while let Ok(token) = self.peek() {
            let attributes = if matches!(token.kind, TokenKind::At) {
                self.expect(&TokenKind::At)?;
                self.parse_attributes()?
            } else {
                vec![]
            };
            let token = self.peek()?;
            if matches!(token.kind, TokenKind::Extern) {
                self.eat()?;
                self.parse_externs(&mut out)?;
                continue;
            }

            let mut decl = self.parse_declaration()?;
            decl.attributes = attributes;
            out.push(decl);
        }
        Ok(out)
    }
}
