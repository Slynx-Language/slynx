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
pub struct ParsingContext {
    ///Span of the keyword that introduced the declaration.
    pub(crate) span: Span,
    pub(crate) attributes: Vec<Spanned<ASTAttribute>>,
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

pub struct ParseCollectionDescriptor<F, T>
where
    F: FnMut(&mut Parser<'_>) -> Result<T>,
{
    ///Every token that ends the list. The list stops at the first of these it
    ///sees and never consumes it, so whatever follows the list stays for the
    ///caller to read. A list nested inside another list must list every token
    ///that can end it, including the ones its enclosing list stops at.
    pub stop_tokens: &'static [TokenKind],
    ///The token that separates two items. When `None`, items are adjacent. A
    ///separator is only required between two items, never after the last one,
    ///so a trailing separator before a stop token is accepted.
    pub separator_token: Option<TokenKind>,
    pub parse_item: F,
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

    ///Whether the next token is one of the `stop_tokens` that end the list
    ///currently being parsed. Only the token discriminants are compared, the
    ///same way [`Parser::expect`] does, so a data-carrying variant can never
    ///match a unit one by payload.
    pub fn at_list_end(&self, stop_tokens: &[TokenKind]) -> Result<bool> {
        let next = std::mem::discriminant(&self.peek()?.kind);
        Ok(stop_tokens
            .iter()
            .any(|stop| std::mem::discriminant(stop) == next))
    }

    ///Parses a list of items separated by `separator_token` and ended by any of
    ///`stop_tokens`, none of which is consumed. Prefer this over a hand written
    ///`while` loop: it keeps the "stop at, do not consume" decision in one
    ///place, and it only ever asks for a separator *between* two items, so the
    ///last item of a list can sit directly against its terminator.
    pub fn parse_collection<F, T>(
        &mut self,
        mut descriptor: ParseCollectionDescriptor<F, T>,
    ) -> Result<Vec<T>>
    where
        F: FnMut(&mut Parser<'_>) -> Result<T>,
    {
        let mut out = Vec::new();

        while !self.at_list_end(descriptor.stop_tokens)? {
            out.push((descriptor.parse_item)(self)?);
            if self.at_list_end(descriptor.stop_tokens)? {
                break;
            }
            if let Some(ref separator) = descriptor.separator_token {
                self.expect(separator)?;
            }
        }
        Ok(out)
    }
}
