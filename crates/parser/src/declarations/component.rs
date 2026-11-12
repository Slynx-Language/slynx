use crate::{
    ASTAttribute, ComponentDeclaration, FuncDeclaration, GenericsMetadata,
    ParseCollectionDescriptor, Result, SymbolPointer,
    ast::{ComponentMember, ComponentMemberKind, VisibilityModifier},
    flags::ParserFlags,
};
use bitflags::Flags;
use common::{Span, Spanned};

use slynx_lexer::tokens::{Token, TokenKind};

use super::Parser;
impl Parser<'_> {
    /// Parses a visibility modifier for a component member. It checks if the next token is 'pub', and if so, it further checks for an optional parenthetical modifier (like 'parent' or 'child') to determine the specific visibility level. If the token is not 'pub', it defaults to `VisibilityModifier::Private`. The function returns the parsed `VisibilityModifier` or an error if an unexpected token is encountered.
    fn parse_modifier(&mut self) -> Result<VisibilityModifier> {
        Ok(match self.peek()?.kind {
            TokenKind::Pub => {
                self.eat()?;
                VisibilityModifier::Public
            }
            _ => VisibilityModifier::Private,
        })
    }

    fn parse_callbacks(&mut self) -> Result<Vec<FuncDeclaration>> {
        self.parse_collection(ParseCollectionDescriptor {
            stop_tokens: &[TokenKind::RBrace],
            separator_token: None,
            parse_item: |parser| {
                let start = parser.expect(&TokenKind::Func)?;
                let callback = parser.parse_func(
                    start.span,
                    vec![],
                    VisibilityModifier::Private,
                    ParserFlags::empty(),
                )?;
                if parser.peek()?.kind == TokenKind::SemiColon {
                    parser.eat()?;
                }
                Ok(callback)
            },
        })
    }

    fn parse_optional_callbacks(&mut self) -> Result<Vec<FuncDeclaration>> {
        if self.peek()?.kind != TokenKind::LBrace {
            return Ok(Vec::new());
        }
        self.eat()?;
        let callbacks = self.parse_callbacks()?;
        self.expect(&TokenKind::RBrace)?;
        if self.peek()?.kind == TokenKind::SemiColon {
            self.eat()?;
        }
        Ok(callbacks)
    }
    ///Parses a component property. The given `span` is the span of the token `prop`
    fn parse_component_property(
        &mut self,
        mut span: Span,
        type_params: &[SymbolPointer],
        modifier: VisibilityModifier,
    ) -> Result<ComponentMember> {
        let ident = self.expect_identifier()?;
        match self.peek()?.kind {
            TokenKind::Colon => {
                self.eat()?;
                let ty = self.parse_type(type_params)?;
                let curr = self.eat()?;
                let (rhs, callbacks) = match curr.kind {
                    TokenKind::LBrace => {
                        // `prop x: T { func ... }`
                        let callbacks = self.parse_callbacks()?;
                        span = span.merge_with(self.expect(&TokenKind::RBrace)?.span);
                        if self.peek()?.kind == TokenKind::SemiColon {
                            span = span.merge_with(self.eat()?.span);
                        }
                        (None, callbacks)
                    }
                    TokenKind::SemiColon => {
                        span.end = curr.span.end;
                        (None, Vec::new())
                    }
                    TokenKind::Eq => {
                        let expr = self.parse_expression(type_params, ParserFlags::empty())?;
                        if self.peek()?.kind == TokenKind::LBrace {
                            // `prop x: T = expr { func ... }`
                            self.eat()?;
                            let callbacks = self.parse_callbacks()?;
                            span.end = self.expect(&TokenKind::RBrace)?.span.end;
                            if self.peek()?.kind == TokenKind::SemiColon {
                                span.end = self.eat()?.span.end;
                            }
                            (Some(expr), callbacks)
                        } else {
                            span.end = self.expect(&TokenKind::SemiColon)?.span.end;
                            (Some(expr), Vec::new())
                        }
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
                        callbacks,
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
                let callbacks = self.parse_optional_callbacks()?;
                if callbacks.is_empty() {
                    span.end = self.expect(&TokenKind::SemiColon)?.span.end;
                }
                Ok(ComponentMember {
                    kind: ComponentMemberKind::Property {
                        callbacks,
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

    /// Parses a component member, which can be either a child component or a property. The visibility `modifier` has already been consumed by the caller. It determines if the member is a child component (identified by an identifier followed by an expression) or a property (identified by the 'prop' keyword followed by an identifier and optional type and default value). The function constructs and returns a `ComponentMember` based on the parsed information, including its kind and span.
    fn parse_component_member(
        &mut self,
        modifier: VisibilityModifier,
        type_params: &[SymbolPointer],
    ) -> Result<ComponentMember> {
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
                let propspan = self.eat()?.span;
                self.parse_component_property(propspan, type_params, modifier)
            }
            TokenKind::View => {
                let propspan = self.eat()?.span;
                let name = self.expect_identifier()?;
                self.expect(&TokenKind::Eq)?;
                let rhs = self.parse_expression(type_params, ParserFlags::empty())?;
                let span = rhs.span.merge_with(propspan);
                Ok(ComponentMember { kind:ComponentMemberKind::View { modifier, name: name.data, rhs } , span })
            }
            TokenKind::In => {
                let propspan = self.eat()?.span;
                let name = self.expect_identifier()?;
                self.expect(&TokenKind::Colon)?;
                let ty = self.parse_type(type_params)?;
                Ok(ComponentMember { kind:ComponentMemberKind::Input { name: name.data, ty }, span: ty.span.merge_with(propspan)})
            }
            _ => self.unexpected(
                "Was expecting some component member. Try defining a property, a method, child, input or view",
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
