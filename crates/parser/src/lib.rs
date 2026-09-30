mod ast;
mod declarations;

pub mod error;

mod flags;
mod program;
mod queries;
use common::{FrontendSymbol, Span, Spanned, SymbolsModule, pool::DedupPool};
pub use error::*;

mod expressions;

mod statement;

mod types;
pub use ast::*;
pub use program::*;

#[cfg(test)]
mod tests;

use slynx_lexer::{TokenKind, TokenStream};

use crate::flags::ParserFlags;

pub type Result<T> = std::result::Result<T, ParseError>;
pub type SymbolPointer = common::SymbolPointer<common::FrontendSymbol>;

///The information about the declaration currently being parsed that is not
///consumed directly by the declaration itself.
pub struct BasicParsingContext<'a> {
    pub(crate) type_params: &'a [SymbolPointer],
    ///Span of the keyword that introduced the declaration.
    pub(crate) span: Span,
}

pub struct ParsingContext<'a> {
    pub(crate) basic: BasicParsingContext<'a>,
    pub(crate) attributes: Vec<Spanned<ASTAttribute>>,
}

impl<'a> std::ops::Deref for ParsingContext<'a> {
    type Target = BasicParsingContext<'a>;
    fn deref(&self) -> &Self::Target {
        &self.basic
    }
}

///The type parameters of the generic function currently being parsed. Each
///entry maps a parameter's name to its index, so that `T` inside
///`func identity<T>(x: T): T` resolves to `Type::Generic(0)`.
pub struct Parser<'a> {
    symbols: &'a SymbolsModule<FrontendSymbol>,
    expressions: &'a DedupPool<ASTExpression>,
    statements: &'a DedupPool<ASTStatement>,
    types: &'a DedupPool<Type>,
    stream: TokenStream,
}

impl<'a> Parser<'a> {
    ///Creates a new parser instance from the given `stream`
    pub fn new(
        stream: TokenStream,
        symbols: &'a SymbolsModule<FrontendSymbol>,
        expressions: &'a DedupPool<ASTExpression>,
        statements: &'a DedupPool<ASTStatement>,
        types: &'a DedupPool<Type>,
    ) -> Self {
        Parser {
            types,
            expressions,
            statements,
            symbols,
            stream,
        }
    }

    ///Terminates the statement that was just parsed. `flags` says whether the
    ///statement is expected to end with a `;`; block bodies always pass
    ///[`ParserFlags::REQUIRE_SEMICOLON`] explicitly so that the check does not
    ///depend on whatever context the block itself was parsed in.
    pub fn finish_current_parse(&mut self, flags: ParserFlags) -> Result<()> {
        if flags.contains(ParserFlags::REQUIRE_SEMICOLON) {
            self.expect(&TokenKind::SemiColon)?;
        }

        Ok(())
    }
}
