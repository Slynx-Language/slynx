mod ast;
mod component;
pub mod conditionals;
mod declarations;
pub mod error;
mod flags;
mod import;
mod program;
mod queries;
use common::{FrontendSymbol, SymbolsModule, pool::DedupPool};
pub use error::*;
mod expr;
mod functions;
pub mod objects;
mod statement;
mod styles;
mod types;
pub use ast::*;
pub use program::*;

use slynx_lexer::{TokenKind, TokenStream};

use crate::flags::{ParserFlag, ParserFlags};

pub type Result<T> = std::result::Result<T, ParseError>;
pub type SymbolPointer = common::SymbolPointer<common::FrontendSymbol>;
pub struct Parser<'a> {
    symbols: &'a SymbolsModule<FrontendSymbol>,
    expressions: &'a DedupPool<ASTExpression>,
    statements: &'a DedupPool<ASTStatement>,
    types: &'a DedupPool<GenericIdentifier>,
    flags: ParserFlags,
    stream: TokenStream,
}

impl<'a> Parser<'a> {
    ///Creates a new parser instance from the given `stream`
    pub fn new(
        stream: TokenStream,
        symbols: &'a SymbolsModule<FrontendSymbol>,
        expressions: &'a DedupPool<ASTExpression>,
        statements: &'a DedupPool<ASTStatement>,
        types: &'a DedupPool<GenericIdentifier>,
    ) -> Self {
        Parser {
            types,
            expressions,
            statements,
            symbols,
            stream,
            flags: ParserFlags::new(),
        }
    }

    pub fn parse_without_component_expr<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        let should_readd = self.flags.has_flag(ParserFlag::ComponentExpr);
        self.flags.remove_flag(ParserFlag::ComponentExpr);

        let result = parse(self);
        if should_readd {
            self.add_flag(ParserFlag::ComponentExpr);
        }
        result
    }

    pub fn finish_current_parse(&mut self) -> Result<()> {
        if self.flags.has_flag(ParserFlag::RequireSemicolon) {
            self.expect(&TokenKind::SemiColon)?;
        }
        self.reset_flags();
        Ok(())
    }
}
