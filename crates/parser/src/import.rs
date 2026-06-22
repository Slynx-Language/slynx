use common::Span;
use slynx_lexer::TokenKind;

use crate::{ASTPath, FileImport, ImportUsage, Parser, Result};

impl Parser<'_> {
    fn parse_import_usage(&mut self) -> Result<ImportUsage> {
        let (name, _) = self.expect_identifier()?;
        if let TokenKind::Identifier(ref as_identifier) = self.peek()?.kind
            && as_identifier == "as"
        {
            self.eat()?;
            let (alias, _) = self.expect_identifier()?;
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

    pub fn parse_import(&mut self, import_span: Span) -> Result<FileImport> {
        let path = {
            let mut out = Vec::new();
            loop {
                let tok = self.peek()?;
                if let TokenKind::Identifier(ref name) = tok.kind {
                    if name == "using" {
                        break;
                    }
                    let (name, _) = self.expect_identifier()?;
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
        let span = import_span.merge_with(self.expect(&TokenKind::SemiColon)?.span);
        let import = FileImport { path, usages, span };
        Ok(import)
    }
}
