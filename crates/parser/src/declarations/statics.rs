use common::{Span, VisibilityModifier};
use slynx_lexer::TokenKind;

use crate::{Parser, Result, StaticDeclaration, flags::ParserFlags};

impl<'a> Parser<'a> {
    pub fn parse_static(
        &mut self,
        span: Span,
        external: bool,
        visibility: VisibilityModifier,
        flags: ParserFlags,
    ) -> Result<StaticDeclaration> {
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type(&[])?;
        let expr = if flags.contains(ParserFlags::ONLY_SIGNATURES) {
            None
        } else {
            Some(self.parse_expression(&[], flags)?)
        };
        self.expect(&TokenKind::SemiColon)?;
        Ok(StaticDeclaration {
            attributes: vec![],
            external,
            span: span.merge_with(expr.as_ref().map(|expr| expr.span).unwrap_or(ty.span)),
            visibility,
            name: name.data,
            ty,
            value: expr,
        })
    }
}
