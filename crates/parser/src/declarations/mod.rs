//! Module idealized for parsing general things related to declarations, such as visibility qualifiers, and attributes

mod component;
mod enums;
mod functions;
mod import;
mod interfaces;
mod objects;
mod statics;
mod styles;
use common::{Spanned, VisibilityModifier, pool::DedupPoolId};
use slynx_lexer::{Token, TokenKind};

use crate::{
    ASTAttribute, GenericClause, ParseCollectionDescriptor, ParseErrorKind, Parser, ParsingContext,
    Result, SymbolPointer, Type, flags::ParserFlags, program::Program,
};

///Every token that can end the header of a declaration. An interface
///implementation list or a `where` clause list is attached to a declaration, so
///what follows it depends on the declaration it is attached to: a block body
///(`{`), an expression body (`->`), the `;` of a signature, or the end of the
///enclosing block.
const HEADER_END: &[TokenKind] = &[
    TokenKind::LBrace,
    TokenKind::Arrow,
    TokenKind::SemiColon,
    TokenKind::RBrace,
];

///[`HEADER_END`] plus `Where`, which introduces the `where` clause list that may
///follow an interface implementation list.
const INTERFACE_IMPLEMENTATIONS_END: &[TokenKind] = &[
    TokenKind::LBrace,
    TokenKind::Arrow,
    TokenKind::SemiColon,
    TokenKind::RBrace,
    TokenKind::Where,
];

///[`HEADER_END`] plus `Comma`, which separates two clauses. A bound list is
///nested inside a clause list, and the clause list only stops at `HEADER_END`,
///so the bound list has to stop at those same tokens to know that its last bound
///has been read.
const HEADER_END_OR_COMMA: &[TokenKind] = &[
    TokenKind::LBrace,
    TokenKind::Arrow,
    TokenKind::SemiColon,
    TokenKind::RBrace,
    TokenKind::Comma,
];

impl<'a> Parser<'a> {
    pub fn parse_visibility(&mut self) -> Result<VisibilityModifier> {
        if matches!(self.peek()?.kind, TokenKind::Pub) {
            self.eat()?;
            Ok(VisibilityModifier::Public)
        } else {
            Ok(VisibilityModifier::Private)
        }
    }

    ///Parses the bounds of a single generic clause, the `A & B` of
    ///`where T: A & B`. The list stops at the `,` that separates two clauses and
    ///at whatever ends the clause list itself.
    fn parse_clause_bounds(
        &mut self,
        generics: &[SymbolPointer],
    ) -> Result<Vec<Spanned<DedupPoolId<Type>>>> {
        self.parse_collection(ParseCollectionDescriptor {
            stop_tokens: HEADER_END_OR_COMMA,
            separator_token: Some(TokenKind::BitAnd),
            parse_item: |parser| parser.parse_type(generics),
        })
    }

    ///Parses a clause for a generic type parameter, such as
    ///```func f<T,K>() where
    ///     T: MyInterface1 & MyInterface2,
    ///     K: MyInterface3 & MyInterface4 {
    /// }
    /// ```
    pub fn parse_clause(&mut self, generics: &[SymbolPointer]) -> Result<Spanned<GenericClause>> {
        let type_to_check = self.parse_type(generics)?;
        self.expect(&TokenKind::Colon)?;
        let bounds = self.parse_clause_bounds(generics)?;
        if bounds.is_empty() {
            return Err(crate::ParseError::new(ParseErrorKind::ExpectedBounds(
                type_to_check.span,
            )));
        }
        Ok(Spanned {
            span: type_to_check.span.merge_with(bounds.last().unwrap().span),
            data: GenericClause {
                type_to_check,
                bounds,
            },
        })
    }

    ///Parses the `where T: A & B, K: C` clause list of a declaration, if there is
    ///one. Returns an empty list when the declaration has no `where` clause.
    ///The list has no terminator of its own: it ends at whatever ends the
    ///declaration header, and it never consumes that token.
    pub fn parse_clauses(
        &mut self,
        generics: &[SymbolPointer],
    ) -> Result<Vec<Spanned<GenericClause>>> {
        if self.peek()?.kind != TokenKind::Where {
            return Ok(Vec::new());
        }
        self.expect(&TokenKind::Where)?;
        self.parse_collection(ParseCollectionDescriptor {
            stop_tokens: HEADER_END,
            separator_token: Some(TokenKind::Comma),
            parse_item: |parser| parser.parse_clause(generics),
        })
    }

    ///Parses the interface implementations of a declaration, such as
    ///`object MyObject: MyInterface1, MyInterface2` or
    ///`extend MyType: MyInterface`. Returns an empty list when the next token is
    ///not `:`, so this can be called unconditionally after the declaration's
    ///name. The list has no terminator of its own: it ends at whatever ends the
    ///declaration header, and it never consumes that token. A `where` clause list
    ///that may follow it is read separately by [`Parser::parse_clauses`].
    pub fn parse_interface_implementations(
        &mut self,
        generics: &[SymbolPointer],
    ) -> Result<Vec<Spanned<DedupPoolId<Type>>>> {
        if self.peek()?.kind != TokenKind::Colon {
            return Ok(Vec::new());
        }
        self.expect(&TokenKind::Colon)?;
        self.parse_collection(ParseCollectionDescriptor {
            stop_tokens: INTERFACE_IMPLEMENTATIONS_END,
            separator_token: Some(TokenKind::Comma),
            parse_item: |parser| parser.parse_type(generics),
        })
    }

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

    ///Parses a single declaration. `flags` is the parsing context the declaration
    ///is being read in, and is forwarded to every declaration kind that cares
    ///about it: `ONLY_SIGNATURES` is set for everything inside an `extern { .. }`
    ///block, and is what makes bodies optional.
    fn parse_declaration(
        &mut self,
        program: &mut Program,
        external: bool,
        flags: ParserFlags,
    ) -> Result<()> {
        let attributes = self.parse_attributes()?;
        let visibility = self.parse_visibility()?;
        match &self.peek()?.kind {
            TokenKind::Extend => {
                let Token { span, .. } = self.eat()?;
                let extend = self.parse_extend(span, attributes)?;
                program.append_extensions(extend);
            }
            TokenKind::Interface => {
                let Token { span, .. } = self.eat()?;
                let mut interface = self.parse_interface(ParsingContext {
                    span,
                    attributes,
                    visibility,
                })?;
                program.append_interfaces(interface);
            }
            TokenKind::Import => {
                let span = self.eat()?.span;
                let import = self.parse_import(span)?;
                program.append_imports(import);
            }
            TokenKind::Alias => {
                let Token { span, .. } = self.eat()?;
                let mut alias = self.parse_alias(span, visibility)?;
                program.append_alias(alias);
            }
            TokenKind::Object => {
                let Token { span, .. } = self.eat()?;
                let mut object =
                    self.parse_object(span, attributes, external, visibility, flags)?;
                program.append_object(object)
            }
            TokenKind::Component => {
                let Token { span, .. } = self.eat()?;
                let mut component =
                    self.parse_component_declaration(span, attributes, visibility)?;
                program.append_component(component);
            }
            TokenKind::Func => {
                let Token { span, .. } = self.eat()?;
                let mut func = self.parse_func(span, attributes, visibility, flags)?;
                func.external = external;
                program.append_func(func);
            }
            TokenKind::StyleSheet => {
                let Token { span, .. } = self.eat()?;
                let mut style = self.parse_stylesheet(span, attributes, visibility)?;
                program.append_style(style);
            }
            TokenKind::Static => {
                let Token { span, .. } = self.eat()?;
                let mut static_decl = self.parse_static(span, external, visibility, flags)?;
                program.append_statics(static_decl);
            }
            TokenKind::Enum => {
                let span = self.eat()?.span;
                let mut enum_decl = self.parse_enum(span, attributes, visibility)?;
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
            // Top level declarations are read in the empty context: no enclosing
            // `extern` block, so `ONLY_SIGNATURES` is absent. Blocks request
            // `COMPONENT_EXPR` themselves when they are parsed.
            self.parse_declaration(&mut program, false, ParserFlags::empty())?;
        }
        Ok(program)
    }
}
