//! Module idealized for parsing general things related to declarations, such as visibility qualifiers, and attributes

mod component;
mod enums;
mod functions;
mod import;
mod interfaces;
mod objects;
mod statics;
mod styles;
use common::{Spanned, VisibilityModifier};
use slynx_lexer::{Token, TokenKind};

use crate::{
    ASTAttribute, BasicParsingContext, Parser, ParsingContext, Result, flags::ParserFlags,
    program::Program,
};

impl<'a> Parser<'a> {
    ///Parses a list of attributes. This makes the parsing of @name(arg0,arg1,arg2,arg3, ...). If the current token is not an `At` token(in code, '@'), an empty list is returned.
    pub fn parse_attributes(&mut self) -> Result<Vec<Spanned<ASTAttribute>>> {
        let mut out = Vec::new();
        if self.peek()?.kind != TokenKind::At {
            return Ok(out);
        }

        while let TokenKind::At = self.peek()?.kind {
            let start = self.expect(&TokenKind::At)?.span;
            let name = self.expect_identifier()?;
            self.expect(&TokenKind::LParen)?;
            let args =
                self.parse_separated(TokenKind::RParen, TokenKind::Comma, true, |parser| {
                    let (arg, _) = parser.expect_string()?;
                    Ok(arg)
                })?;
            let end = self.expect(&TokenKind::RParen)?.span;
            let attrib = start.merge_with(end).make_spanned(ASTAttribute {
                name: name.data,
                args,
            });
            out.push(attrib);
        }
        Ok(out)
    }

    ///Parses a single declaration
    fn parse_declaration(
        &mut self,
        program: &mut Program,
        external: bool,
        flags: ParserFlags,
    ) -> Result<()> {
        let attributes = self.parse_attributes()?;
        let token = self.peek()?;
        let visibility = if matches!(token.kind, TokenKind::Pub) {
            self.eat()?;
            VisibilityModifier::Public
        } else {
            VisibilityModifier::Private
        };
        match &self.peek()?.kind {
            TokenKind::Extend => {
                let Token { span, .. } = self.eat()?;
                let extend = self.parse_extend(span, attributes)?;
                program.append_extensions(extend);
            }
            TokenKind::Interface => {
                let Token { span, .. } = self.eat()?;
                let mut interface = self.parse_interface(ParsingContext {
                    basic: BasicParsingContext {
                        type_params: &[],
                        _flags: flags,
                        span,
                    },
                    attributes,
                })?;
                interface.visibility = visibility;
                program.append_interfaces(interface);
            }
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
                let mut object = self.parse_object(span, attributes, flags)?;
                object.external = external;
                object.visibility = visibility;
                program.append_object(object)
            }
            TokenKind::Component => {
                let Token { span, .. } = self.eat()?;
                let mut component = self.parse_component_declaration(span, attributes)?;
                component.visibility = visibility;
                program.append_component(component);
            }
            TokenKind::Func => {
                let Token { span, .. } = self.eat()?;
                let mut func = self.parse_func(span, attributes, flags)?;
                func.external = external;
                func.visibility = visibility;
                program.append_func(func);
            }
            TokenKind::StyleSheet => {
                let Token { span, .. } = self.eat()?;
                let mut style = self.parse_stylesheet(span, attributes)?;
                style.visibility = visibility;
                program.append_style(style);
            }
            TokenKind::Static => {
                let Token { span, .. } = self.eat()?;
                let mut static_decl = self.parse_static(span, flags)?;
                static_decl.external = external;
                static_decl.visibility = visibility;
                program.append_statics(static_decl);
            }
            TokenKind::Enum => {
                let span = self.eat()?.span;
                let mut enum_decl = self.parse_enum(span, attributes)?;
                enum_decl.visibility = visibility;
                program.append_enums(enum_decl);
            }
            _ => {
                return self.unexpected(
                    "Unknown declaration that starts with it. Expected some valid declaration",
                );
            }
        };
        Ok(())
    }

    ///Parse extern declarations and insert them on the given `declarations`. Returns the amount of declarations parsed. The main reason for this to not return a new Vec<> is to simply not allocate on a separated vector
    /// and then need to copy/move all the data to the correct vector
    fn parse_externs(&mut self, program: &mut Program) -> Result<usize> {
        self.expect(&TokenKind::LBrace)?;
        let mut amount_parsed = 0;
        loop {
            if self.peek()?.kind == TokenKind::RBrace {
                self.eat()?;
                break Ok(amount_parsed);
            }

            self.parse_declaration(program, true, ParserFlags::ONLY_SIGNATURES)?;
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
            self.parse_declaration(&mut program, false, ParserFlags::empty())?;
        }
        Ok(program)
    }
}
