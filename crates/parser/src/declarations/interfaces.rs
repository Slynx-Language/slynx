use common::{Span, Spanned, VisibilityModifier, pool::DedupPoolId};
use slynx_lexer::TokenKind;

use crate::{
    ASTAttribute, BasicParsingContext, ExtendDeclaration, GenericsMetadata, InterfaceDeclaration,
    Parser, ParsingContext, Result, SymbolPointer, Type, flags::ParserFlags,
};

impl Parser<'_> {
    fn parse_interface_requirements(
        &mut self,
        basic: &BasicParsingContext<'_>,
    ) -> Result<Vec<Spanned<DedupPoolId<Type>>>> {
        if let TokenKind::Requires = self.peek()?.kind {
            self.expect(&TokenKind::Requires)?;

            let mut out = Vec::new();
            while self.peek()?.kind != TokenKind::RBrace {
                out.push(self.parse_type(basic.type_params)?);
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

    ///Parses an `interface Name<T> requires A, B { func f() -> T; ... }`
    ///declaration. Every method of an interface is a signature, so
    ///[`ParserFlags::ONLY_SIGNATURES`] is always requested regardless of the
    ///context the interface itself was declared in.
    pub fn parse_interface(&mut self, context: ParsingContext) -> Result<InterfaceDeclaration> {
        let (name, type_args) = self.parse_generic_name()?;
        let (interfaces, clauses) = self.parse_interface_implementations(&type_args)?;
        let requirements = self.parse_interface_requirements(&context.basic)?;
        let methods = {
            self.expect(&TokenKind::LBrace)?;
            let mut out = Vec::new();

            while self.peek()?.kind != TokenKind::RBrace {
                let attributes = self.parse_attributes()?;
                let span = self.peek()?.span;
                self.expect(&TokenKind::Func)?;
                let method = self.parse_func(span, attributes, ParserFlags::ONLY_SIGNATURES)?;
                out.push(method);
            }
            out
        };
        let end = self.expect(&TokenKind::RBrace)?.span;

        Ok(InterfaceDeclaration {
            name,
            generics: GenericsMetadata {
                type_params: type_args,
                interface_implementations: interfaces,
                clauses,
            },
            methods,
            super_interfaces: requirements,
            span: context.span.merge_with(end),
            attributes: context.attributes,
            visibility: VisibilityModifier::default(),
        })
    }

    pub fn parse_extension_generics(&mut self) -> Result<Vec<SymbolPointer>> {
        if self.peek()?.kind != TokenKind::Lt {
            return Ok(Vec::new());
        } else {
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
    }

    pub fn parse_extend(
        &mut self,
        span: Span,
        attributes: Vec<Spanned<ASTAttribute>>,
    ) -> Result<ExtendDeclaration> {
        //extend a: b,c,d,e where t {}
        let generic_inputs = self.parse_extension_generics()?;
        let target = self.parse_type(&generic_inputs)?;
        self.expect(&TokenKind::Colon)?;
        let (target_interfaces, clauses) = self.parse_interface_implementations(&generic_inputs)?; //already eats the left brace.
        let mut methods = Vec::new();
        while self.peek()?.kind != TokenKind::RBrace {
            let attributes = self.parse_attributes()?;
            let span = self.peek()?.span;
            self.expect(&TokenKind::Func)?;
            methods.push(self.parse_func(span, attributes, ParserFlags::empty())?);
        }
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
