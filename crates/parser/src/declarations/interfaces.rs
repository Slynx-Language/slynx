use common::{Span, Spanned, VisibilityModifier};
use slynx_lexer::TokenKind;

use crate::{
    ASTAttribute, ExtendDeclaration, GenericsMetadata, InterfaceDeclaration, Parser,
    ParsingContext, Result, SymbolPointer, flags::ParserFlags,
};

impl Parser<'_> {
    ///Parses an `interface Name<T>: A, B where T: C { func f() -> T; ... }`
    ///declaration. Every method of an interface is a signature, so
    ///[`ParserFlags::ONLY_SIGNATURES`] is always requested regardless of the
    ///context the interface itself was declared in.
    pub fn parse_interface(&mut self, context: ParsingContext) -> Result<InterfaceDeclaration> {
        //interface Name<T>: A, B where T: C { ... }
        let (name, type_args) = self.parse_generic_name()?;
        let interfaces = self.parse_interface_implementations(&type_args)?;
        let clauses = self.parse_clauses(&type_args)?;
        self.expect(&TokenKind::LBrace)?;
        let mut methods = Vec::new();

        while self.peek()?.kind != TokenKind::RBrace {
            let attributes = self.parse_attributes()?;
            let span = self.peek()?.span;
            self.expect(&TokenKind::Func)?;
            methods.push(self.parse_func(span, attributes, ParserFlags::ONLY_SIGNATURES)?);
        }
        let end = self.expect(&TokenKind::RBrace)?.span;

        Ok(InterfaceDeclaration {
            name,
            generics: GenericsMetadata {
                type_params: type_args,
                interface_implementations: interfaces,
                clauses,
            },
            methods,
            // The language has no `requires` keyword, so an interface never
            // names another one as a super-interface here. The field stays so
            // that adding the keyword back only needs parser work.
            super_interfaces: Vec::new(),
            span: context.span.merge_with(end),
            attributes: context.attributes,
            visibility: VisibilityModifier::default(),
        })
    }

    pub fn parse_extension_generics(&mut self) -> Result<Vec<SymbolPointer>> {
        if self.peek()?.kind != TokenKind::Lt {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        self.expect(&TokenKind::Lt)?;
        while self.peek()?.kind != TokenKind::Gt {
            out.push(self.expect_identifier()?.data);
            if self.peek()?.kind == TokenKind::Gt {
                break;
            }
            self.expect(&TokenKind::Comma)?;
        }
        self.expect(&TokenKind::Gt)?;
        Ok(out)
    }

    pub fn parse_extend(
        &mut self,
        span: Span,
        attributes: Vec<Spanned<ASTAttribute>>,
    ) -> Result<ExtendDeclaration> {
        let generic_inputs = self.parse_extension_generics()?;

        let target = self.parse_type(&generic_inputs)?;
        let target_interfaces = self.parse_interface_implementations(&generic_inputs)?;
        let clauses = self.parse_clauses(&generic_inputs)?;

        self.expect(&TokenKind::LBrace)?;

        let methods = {
            let mut methods = Vec::new();
            while self.peek()?.kind != TokenKind::RBrace {
                let attributes = self.parse_attributes()?;
                let span = self.peek()?.span;
                self.expect(&TokenKind::Func)?;
                methods.push(self.parse_func(span, attributes, ParserFlags::empty())?);
            }
            methods
        };
        let end = self.expect(&TokenKind::RBrace)?.span;
        Ok(ExtendDeclaration {
            target,
            generics: GenericsMetadata {
                type_params: generic_inputs,
                interface_implementations: target_interfaces,
                clauses,
            },

            methods,
            attributes,
            span: span.merge_with(end),
        })
    }
}
