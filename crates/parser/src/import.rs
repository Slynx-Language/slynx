use common::Span;
use slynx_lexer::TokenKind;

use crate::{ASTDeclaration, ASTPath, FileImport, ImportUsage, Parser, Result};

impl Parser {
    fn parse_import_usage(&mut self) -> Result<ImportUsage> {
        let TokenKind::Identifier(name) = self.expect_identifier()?.kind else {
            unreachable!()
        };
        if let TokenKind::Identifier(ref as_identifier) = self.peek()?.kind
            && as_identifier == "as"
        {
            self.eat()?;
            let TokenKind::Identifier(alias) = self.expect_identifier()?.kind else {
                unreachable!();
            };
            Ok(ImportUsage {
                content_name: name,
                alias: Some(alias),
            })
        } else {
            Ok(ImportUsage {
                content_name: name,
                alias: None,
            })
        }
    }

    pub fn parse_import(&mut self, span: Span) -> Result<ASTDeclaration> {
        let path = {
            let mut out = Vec::new();
            loop {
                let tok = self.peek()?;
                if let TokenKind::Identifier(ref name) = tok.kind {
                    if name == "using" {
                        break;
                    }
                    let TokenKind::Identifier(name) = self.expect_identifier()?.kind else {
                        unreachable!()
                    };
                    if let TokenKind::Dot = self.peek()?.kind {
                        self.eat()?;
                    }
                    out.push(name);
                } else {
                    break;
                }
            }
            ASTPath { module_names: out }
        };
        let usages = {
            let mut out = Vec::new();
            if let TokenKind::Identifier(ref name) = self.peek()?.kind
                && name == "using"
            {
                self.eat()?;
                match self.peek()?.kind {
                    TokenKind::LBrace => {
                        self.eat()?;
                        while self.peek()?.kind != TokenKind::RBrace {
                            out.push(self.parse_import_usage()?);
                            if self.peek()?.kind == TokenKind::Comma {
                                self.eat()?;
                            }
                        }
                        self.eat()?;
                    }
                    _ => out.push(self.parse_import_usage()?),
                }
            }
            out
        };
        self.expect(&TokenKind::SemiColon)?;
        let import = FileImport { path, usages };
        Ok(ASTDeclaration { visibility: Default::default(),
            kind: crate::ASTDeclarationKind::Import(import),
            span,
        })
    }
}
