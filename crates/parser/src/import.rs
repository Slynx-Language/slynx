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
            while let TokenKind::Identifier(_) = self.peek()?.kind {
                let TokenKind::Identifier(name) = self.expect_identifier()?.kind else {
                    unreachable!()
                };
                if let TokenKind::Dot = self.peek()?.kind {
                    self.eat()?;
                }
                out.push(name);
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
                        while self.peek()?.kind != TokenKind::RBrace {
                            out.push(self.parse_import_usage()?);
                            if self.peek()?.kind == TokenKind::Comma {
                                self.eat()?;
                            }
                        }
                    }
                    _ => out.push(self.parse_import_usage()?),
                }
            }
            out
        };
        self.expect(&TokenKind::SemiColon)?;
        let import = FileImport { path, usages };
        Ok(ASTDeclaration {
            kind: crate::ASTDeclarationKind::Import(import),
            span,
        })
    }
}
