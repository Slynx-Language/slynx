//! Module idealized for parsing general things related to declarations, such as visibility qualifiers, and attributes

use common::{Span, Spanned, VisibilityModifier};
use slynx_lexer::{Token, TokenKind};

use crate::{
    ASTAttribute, ExpectedContent, ParseError, Parser, Result, StaticDeclaration,
    flags::ParserFlag, program::Program,
};

impl<'a> Parser<'a> {
    pub fn parse_static(&mut self, span: Span) -> Result<StaticDeclaration> {
        let (name, _) = self.expect_identifier()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let expr = if self.flags.has_flag(ParserFlag::OnlySignatures) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect(&TokenKind::SemiColon)?;
        Ok(StaticDeclaration {
            attributes: vec![],
            external: false,
            span: span.merge_with(expr.as_ref().map(|expr| expr.span).unwrap_or(ty.span)),
            visibility: VisibilityModifier::Private,
            name,
            ty,
            value: expr,
        })
    }

    pub fn parse_attributes(&mut self) -> Result<Vec<Spanned<ASTAttribute>>> {
        let mut out = Vec::new();
        if self.peek()?.kind != TokenKind::At {
            return Ok(out);
        }

        while let TokenKind::At = self.peek()?.kind {
            let start = self.expect(&TokenKind::At)?.span;
            let (name, _) = self.expect_identifier()?;
            self.expect(&TokenKind::LParen)?;
            let args = {
                let mut args = Vec::new();
                loop {
                    if self.peek()?.kind == TokenKind::RParen {
                        break args;
                    }
                    let (arg, _) = self.expect_string()?;
                    args.push(arg);
                    if self.peek()?.kind == TokenKind::Comma {
                        self.eat()?;
                    }
                }
            };
            let end = self.expect(&TokenKind::RParen)?.span;
            let attrib = start
                .merge_with(end)
                .make_spanned(ASTAttribute { name, args });
            out.push(attrib);
        }
        Ok(out)
    }

    ///Parses a single declaration
    fn parse_declaration(&mut self, program: &mut Program, external: bool) -> Result<()> {
        let mut attributes = self.parse_attributes()?;
        let token = self.peek()?;
        let visibility = if matches!(token.kind, TokenKind::Pub) {
            self.eat()?;
            VisibilityModifier::Public
        } else {
            VisibilityModifier::Private
        };
        match &self.peek()?.kind {
            TokenKind::Import => {
                let span = self.eat()?.span;
                let import = self.parse_import(span)?;
                program.append_imports(import);
            }
            TokenKind::Alias => {
                let Token { span, .. } = self.eat()?;
                let mut alias = self.parse_alias(span)?;
                alias.visibility = visibility;
                program.append_alias(alias);
            }
            TokenKind::Object => {
                let Token { span, .. } = self.eat()?;
                let mut object = self.parse_object(span)?;
                object.attributes.append(&mut attributes);
                object.external = external;
                object.visibility = visibility;
                program.append_object(object)
            }
            TokenKind::Component => {
                let Token { span, .. } = self.eat()?;
                let mut component = self.parse_component(span)?;
                component.attributes.append(&mut attributes);
                component.visibility = visibility;
                program.append_component(component);
            }
            TokenKind::Func => {
                let Token { span, .. } = self.eat()?;
                let mut func = self.parse_func(span)?;
                func.attributes.append(&mut attributes);
                func.external = external;
                func.visibility = visibility;
                program.append_func(func);
            }
            TokenKind::StyleSheet => {
                let Token { span, .. } = self.eat()?;
                let mut style = self.parse_stylesheet(span)?;
                style.attributes.append(&mut attributes);
                style.visibility = visibility;
                program.append_style(style);
            }
            TokenKind::Static => {
                let Token { span, .. } = self.eat()?;
                let mut static_decl = self.parse_static(span)?;
                static_decl.attributes.append(&mut attributes);
                static_decl.external = external;
                static_decl.visibility = visibility;
                program.append_statics(static_decl);
            }
            _ => {
                return Err(ParseError::UnexpectedToken(
                    self.eat()?,
                    ExpectedContent::Raw(
                        "Unknown declaration that starts with it. Expected some valid declaration"
                            .to_owned(),
                    ),
                ));
            }
        };
        Ok(())
    }

    ///Parse extern declarations and insert them on the given `declarations`. Returns the amount of declarations parsed. The main reason for this to not return a new Vec<> is to simply not allocate on a separated vector
    /// and then need to copy/move all the data to the correct vector
    fn parse_externs(&mut self, program: &mut Program) -> Result<usize> {
        self.expect(&TokenKind::LBrace)?;
        self.add_flag(ParserFlag::OnlySignatures);
        let mut amount_parsed = 0;
        loop {
            if self.peek()?.kind == TokenKind::RBrace {
                self.eat()?;
                self.remove_flag(ParserFlag::OnlySignatures);
                break Ok(amount_parsed);
            }
            self.parse_declaration(program, true)?;
            amount_parsed += 1;
        }
    }

    /// Parses the declarations in the source code and returns them as a vector of `ASTDeclaration`s.
    /// The parser will continue parsing until it reaches the end of the input stream.
    /// If it encounters an unexpected token, it will return an error indicating the expected token type.
    pub fn parse_declarations(&mut self) -> Result<Program> {
        let mut program = Program::new();
        while let Ok(token) = self.peek() {
            if matches!(token.kind, TokenKind::Extern) {
                self.eat()?;
                self.parse_externs(&mut program)?;
                continue;
            }
            self.parse_declaration(&mut program, false)?;
        }
        Ok(program)
    }
}
