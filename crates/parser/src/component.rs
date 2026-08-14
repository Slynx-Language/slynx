use crate::{
    ComponentDeclaration, ExpectedContent, Result, TypeParamScope,
    ast::{ComponentMember, ComponentMemberKind, VisibilityModifier},
};
use common::Span;

use crate::error::ParseError;
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
                        return Err(ParseError::UnexpectedToken(
                            Token {
                                kind: TokenKind::Identifier(self.symbols.get_name(modifier.data).to_string()),
                                span: modifier.span,
                            },
                            ExpectedContent::Raw("Instead was expecting child' or 'parent' to determine who will be able to access it"
                                .to_string()),
                        ));
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

    /// Parses a component member, which can be either a child component or a property. It first checks for any visibility modifiers (like 'pub'), then determines if the member is a child component (identified by an identifier followed by an expression) or a property (identified by the 'prop' keyword followed by an identifier and optional type and default value). The function constructs and returns a `ComponentMember` based on the parsed information, including its kind and span.
    fn parse_component_member(&mut self, type_params: TypeParamScope) -> Result<ComponentMember> {
        let mut span = self.peek()?.span;
        let modifier = self.parse_modifier()?;
        let curr = self.peek()?;
        match curr.kind {
            TokenKind::Identifier(_) => {
                let span = curr.span;
                let expr = self.parse_component_expr(type_params)?;
                Ok(ComponentMember {
                    kind: ComponentMemberKind::Child(expr),
                    span,
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
                                let expr = self.parse_expression(type_params)?;

                                span.end = self.expect(&TokenKind::SemiColon)?.span.end;
                                Some(expr)
                            }
                            _ => {
                                return Err(ParseError::UnexpectedToken(
                                    curr,
                                    ExpectedContent::Raw("Was expecing ';' to determine this property initialization is required by it's parent or '=' to give it a default value".to_string()),
                                ));
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
                        let expr = self.parse_expression(type_params)?;
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
                        Err(ParseError::UnexpectedToken(
                            self.eat()?,
                            ExpectedContent::Raw("Was expecting '=' or ':' to define the type of the property or a ';' to keep it to be initialized by it's parent".to_string()),
                        ))
                    }
                }
            }
            _ => Err(ParseError::UnexpectedToken(
                self.eat()?,
                ExpectedContent::Raw("Was expecting some component member. Try defining a property, a method or child".to_string()),
            )),
        }
    }
    ///Parses a component declaration. This initializes on the 'component' keyword
    pub(crate) fn parse_component(&mut self, mut span: Span) -> Result<ComponentDeclaration> {
        let (name, generics) = self.parse_generic_name()?;

        self.expect(&TokenKind::LBrace)?;
        let mut defs = Vec::new();
        let mut type_params = Vec::new();
        self.push_type_params(&generics, &mut type_params);
        while self.peek()?.kind != TokenKind::RBrace {
            defs.push(self.parse_component_member(&type_params)?);
        }
        let Token { span: end, .. } = self.expect(&TokenKind::RBrace)?;
        span.end = end.end;

        Ok(ComponentDeclaration {
            type_params: generics,
            attributes: Vec::new(),
            visibility: Default::default(),
            name,
            members: defs,
            span,
        })
    }
}
