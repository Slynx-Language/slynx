use crate::{
    ASTAttribute, ComponentDeclaration, GenericsMetadata, Result, SymbolPointer,
    ast::{ComponentMember, ComponentMemberKind, VisibilityModifier},
    flags::ParserFlags,
};
use common::{Span, Spanned};

use slynx_lexer::tokens::{Token, TokenKind};

use super::Parser;
impl Parser<'_> {
    /// Parses a visibility modifier for a component member. It checks if the next token is 'pub', and if so, it further checks for an optional parenthetical modifier (like 'parent' or 'child') to determine the specific visibility level. If the token is not 'pub', it defaults to `VisibilityModifier::Private`. The function returns the parsed `VisibilityModifier` or an error if an unexpected token is encountered.
    fn parse_modifier(&mut self) -> Result<VisibilityModifier> {
        Ok(match self.peek()?.kind {
            TokenKind::Pub => {
                self.eat()?;
                if self.peek()?.kind == TokenKind::LParen {
                    self.eat()?;
                    let modifier = self.expect_identifier()?;

                    let modifier = if modifier.data == self.intern("parent") {
                        VisibilityModifier::ParentPublic
                    } else if modifier.data == self.intern("child") {
                        VisibilityModifier::ChildrenPublic
                    } else {
                        return self.unexpected_with(
                            "Instead was expecting child' or 'parent' to determine who will be able to access it",
                            Token {
                                kind: TokenKind::Identifier(self.symbols.get_name(modifier.data).to_string()),
                                span: modifier.span,
                            },
                        );
                    };
                    self.expect(&TokenKind::RParen)?;
                    modifier
                } else {
                    VisibilityModifier::Public
                }
            }
            _ => VisibilityModifier::Private,
        })
    }

    /// Parses a component member, which can be either a child component or a property. The visibility `modifier` has already been consumed by the caller. It determines if the member is a child component (identified by an identifier followed by an expression) or a property (identified by the 'prop' keyword followed by an identifier and optional type and default value). The function constructs and returns a `ComponentMember` based on the parsed information, including its kind and span.
    fn parse_component_member(
        &mut self,
        modifier: VisibilityModifier,
        type_params: &[SymbolPointer],
    ) -> Result<ComponentMember> {
        let mut span = self.peek()?.span;
        let curr = self.peek()?;
        match curr.kind {
            TokenKind::Identifier(_) if self.peek_at(1)?.kind == TokenKind::Eq => {
                //checks for 'name='
                let name = self.expect_identifier()?;
                self.eat()?;
                let expr = self.parse_component_expr(type_params, ParserFlags::empty())?;
                Ok(ComponentMember {
                    span: name.span.merge_with(expr.span),
                    kind: ComponentMemberKind::NamedChild {
                        name: name.data,
                        child: expr,
                    },
                })
            }
            TokenKind::Identifier(_) => {
                let span = curr.span;
                let expr = self.parse_component_expr(type_params, ParserFlags::empty())?;
                Ok(ComponentMember {
                    span: span.merge_with(expr.span),
                    kind: ComponentMemberKind::Child(expr),
                })
            }
            TokenKind::Prop => {
                self.eat()?;
                let ident = self.expect_identifier()?;
                match self.peek()?.kind {
                    TokenKind::SemiColon => {
                        span.end = self.eat()?.span.end;
                        Ok(ComponentMember {
                            kind: ComponentMemberKind::Property {
                                name: ident.data,
                                modifier,
                                ty: None,
                                rhs: None,
                            },
                            span,
                        })
                    }

                    TokenKind::Colon => {
                        self.eat()?;
                        let ty = self.parse_type(type_params)?;

                        let curr = self.eat()?;
                        let rhs = match curr.kind {
                            TokenKind::SemiColon => {
                                span.end = curr.span.end;
                                None
                            }
                            TokenKind::Eq => {
                                let expr = self.parse_expression(type_params, ParserFlags::empty())?;

                                span.end = self.expect(&TokenKind::SemiColon)?.span.end;
                                Some(expr)
                            }
                            _ => {
                                return self.unexpected_with(
                                    "Was expecing ';' to determine this property initialization is required by it's parent or '=' to give it a default value",
                                    curr,
                                );
                            }
                        };
                        Ok(ComponentMember {
                            kind: ComponentMemberKind::Property {
                                name: ident.data,
                                modifier,
                                ty: Some(ty),
                                rhs,
                            },
                            span,
                        })
                    }
                    TokenKind::Eq => {
                        self.eat()?;
                        let expr = self.parse_expression(type_params, ParserFlags::empty())?;
                        span.end = self.expect(&TokenKind::SemiColon)?.span.end;
                        Ok(ComponentMember {
                            kind: ComponentMemberKind::Property {
                                name: ident.data,
                                modifier,
                                ty: None,
                                rhs: Some(expr),
                            },
                            span,
                        })
                    }
                    _ => {
                        self.unexpected(
                            "Was expecting '=' or ':' to define the type of the property or a ';' to keep it to be initialized by it's parent",
                        )
                    }
                }
            }
            _ => self.unexpected(
                "Was expecting some component member. Try defining a property, a method or child",
            ),
        }
    }
    ///Parses a component declaration. This initializes on the 'component' keyword
    pub(crate) fn parse_component_declaration(
        &mut self,
        span: Span,
        attributes: Vec<Spanned<ASTAttribute>>,
        visibility: VisibilityModifier,
    ) -> Result<ComponentDeclaration> {
        let (name, generics) = self.parse_generic_name()?;
        let interface_implementations = self.parse_interface_implementations(&generics)?;
        let clauses = self.parse_clauses(&generics)?;

        self.expect(&TokenKind::LBrace)?;
        let mut defs = Vec::new();
        let mut methods = Vec::new();

        while self.peek()?.kind != TokenKind::RBrace {
            let attributes = self.parse_attributes()?;
            let modifier = self.parse_modifier()?;
            if self.peek()?.kind == TokenKind::Func {
                let start = self.eat()?.span;
                methods.push(self.parse_method(
                    start,
                    attributes,
                    ParserFlags::empty(),
                    modifier,
                )?);
                if self.peek()?.kind == TokenKind::Comma {
                    self.eat()?;
                }
                continue;
            }
            defs.push(self.parse_component_member(modifier, &generics)?);
        }
        let Token { span: end, .. } = self.expect(&TokenKind::RBrace)?;

        Ok(ComponentDeclaration {
            generics: GenericsMetadata {
                type_params: generics,
                interface_implementations,
                clauses,
            },
            attributes,
            visibility,
            name,
            members: defs,
            span: span.merge_with(end),
            methods,
        })
    }
}
