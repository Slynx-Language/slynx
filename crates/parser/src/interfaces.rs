use common::{Span, Spanned, VisibilityModifier, pool::DedupPoolId};
use slynx_lexer::TokenKind;

use crate::{ASTAttribute, InterfaceDeclaration, Parser, Result, SymbolPointer, Type};

impl Parser<'_> {
    fn parse_interface_requirements(
        &mut self,
        type_args: &[SymbolPointer],
    ) -> Result<Vec<Spanned<DedupPoolId<Type>>>> {
        if let TokenKind::Requires = self.peek()?.kind {
            self.expect(&TokenKind::Requires)?;

            let mut out = Vec::new();
            while self.peek()?.kind != TokenKind::RBrace {
                out.push(self.parse_type(type_args)?);
                if self.peek()?.kind == TokenKind::RBrace {
                    break;
                }
                self.expect(&TokenKind::Comma)?;
            }
            Ok(out)
        } else {
            Ok(vec![])
        }
    }

    pub fn parse_interface(
        &mut self,
        span: Span,
        attributes: Vec<Spanned<ASTAttribute>>,
    ) -> Result<InterfaceDeclaration> {
        let (name, type_args) = self.parse_generic_name()?;
        let requirements = self.parse_interface_requirements(&type_args)?;
        let methods = {
            self.expect(&TokenKind::LBrace)?;
            let mut out = Vec::new();
            while self.peek()?.kind != TokenKind::RBrace {
                self.flags
                    .set_flag(crate::flags::ParserFlag::OnlySignatures);
                let attributes = self.parse_attributes()?;
                let method = self.parse_func(span, attributes)?;
                out.push(method);
            }
            out
        };
        let end = self.expect(&TokenKind::RBrace)?.span;

        self.flags
            .remove_flag(crate::flags::ParserFlag::OnlySignatures);
        Ok(InterfaceDeclaration {
            name,
            type_args,
            methods,
            super_interfaces: requirements,
            attributes,
            visibility: VisibilityModifier::default(),
            span: span.merge_with(end),
        })
    }
}
