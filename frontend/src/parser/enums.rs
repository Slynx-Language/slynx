use color_eyre::eyre::Result;
use common::{ASTDeclaration, ASTDeclarationKind};

use crate::lexer::tokens::{Token, TokenKind};
use crate::parser::Parser;

impl Parser{
    pub fn parse_enum(&mut self) -> Result<ASTDeclaration> {
    let enum_token = self.expect(&TokenKind::Enum)?;
    let declaration = self.expect(&TokenKind::Identifier(String::new()))?;
    self.expect(&TokenKind::LBrace)?;
    let data = Vec::new();
    while let TokenKind::RBrace = self.peek()?.kind {
        let name = self.expect(&TokenKind::Identifier(String::new()))?;
        let comma = self.expect(&TokenKind::Comma)?;
        data.push(name);
    }
    let brace = self.expect(&TokenKind::RBrace)?;
    Ok(ASTDeclaration{
        span: Span{
            start: enum_token.span.start,
            end: brace.span.end,
        },
        kind: ASTDeclarationKind {
            
        }
        }
    };
} 




