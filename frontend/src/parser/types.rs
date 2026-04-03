use super::Parser;
use crate::lexer::tokens::{Token, TokenKind};
use color_eyre::eyre::Result;
use common::{ASTDeclaration, ASTDeclarationKind, Span, ast::GenericIdentifier};
impl Parser {
    ///Parses an alias declaration which follows `alias ty = AnotherType`
    pub fn parse_alias(&mut self, init: Span) -> Result<ASTDeclaration> {
        let name = self.parse_type()?;
        self.expect(&TokenKind::Eq)?;
        let target = self.parse_type()?;
        Ok(ASTDeclaration {
            span: Span {
                start: init.start,
                end: target.span.end,
            },
            kind: ASTDeclarationKind::Alias { name, target },
        })
    }

    ///Looking from where this function initializes, check is this is a generic one.
    ///Note that this will only work when the function initializes on something like: N<...
    ///the '...' is what this function will check. It won't eat anything, just look ahead.
    ///`ahead` is just a parameter to know how many tokens to look ahead. When using this, it should be initialized
    ///by the index where the token after '<' is at. This function will return weather it was a generic or not, and return the amount needed to look ahead to keep going
    pub fn is_generic(&self, mut ahead: usize) -> Result<(bool, usize)> {
        let initial_ahead = ahead;
        let Token {
            kind: TokenKind::Identifier(_),
            ..
        } = self.peek_at(ahead)?
        else {
            return Ok((false, ahead));
        };

        if let TokenKind::Lt = self.peek_at(ahead + 1)?.kind {
            match self.is_generic(ahead + 2)? {
                (true, n) => ahead += n,
                (false, n) => return Ok((false, n - initial_ahead)),
            }
        }
        return Ok((
            matches!(self.peek_at(ahead + 1)?.kind, TokenKind::Gt),
            ahead - initial_ahead,
        ));
    }

    ///Parses a type.
    pub fn parse_type(&mut self) -> Result<GenericIdentifier> {
        let Token {
            kind: TokenKind::Identifier(ident),
            mut span,
        } = self.expect(&TokenKind::Identifier("".to_string()))?
        else {
            unreachable!()
        };
        if let Token {
            kind: TokenKind::Lt,
            ..
        } = self.peek()?
        {
            let mut generics = Vec::new();
            self.eat()?;
            loop {
                if let TokenKind::Gt = self.peek()?.kind {
                    let end = self.eat()?.span;
                    span.end = end.end;
                    break;
                }
                let ty = self.parse_type()?;
                generics.push(ty);
            }
            Ok(GenericIdentifier {
                generic: Some(generics),
                identifier: ident,
                span,
            })
        } else {
            Ok(GenericIdentifier {
                identifier: ident,
                generic: None,
                span,
            })
        }
    }
}
