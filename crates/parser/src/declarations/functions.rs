use crate::flags::ParserFlags;
use crate::{ASTAttribute, SymbolPointer};
use crate::{FuncDeclaration, Parser, Result};
use slynx_lexer::tokens::TokenKind;

use crate::ast::{ASTStatement, TypedName};
use common::{Span, Spanned};
impl Parser<'_> {
    ///Parses the arguments of a function. It parses until the `)` of the function args.
    pub fn parse_args(&mut self, type_params: &[SymbolPointer]) -> Result<Vec<Spanned<TypedName>>> {
        self.parse_separated(TokenKind::RParen, TokenKind::Comma, true, |parser| {
            parser.parse_typedname(type_params)
        })
    }

    ///Parses a function. The provided `span` is the initial span for the 'func' keyword.
    ///Parses both `func main(arg1:T): Q {...}` and `func main(arg1:T): Q -> ...`
    pub fn parse_func(
        &mut self,
        span: Span,
        attributes: Vec<Spanned<ASTAttribute>>,
        flags: ParserFlags,
    ) -> Result<FuncDeclaration> {
        let (name, generics) = self.parse_generic_name()?;
        self.parse_func_rest(span, name, generics, attributes, flags)
    }

    ///Parses everything that comes after the function name: the arguments, the
    ///return type and the body. The type parameters are kept in scope while this
    ///runs, so `T` in argument/return types resolves to [`Type::Generic`].
    fn parse_func_rest(
        &mut self,
        span: Span,
        name: SymbolPointer,
        type_params: Vec<SymbolPointer>,
        attributes: Vec<Spanned<ASTAttribute>>,
        flags: ParserFlags,
    ) -> Result<FuncDeclaration> {
        self.expect(&TokenKind::LParen)?;
        let args = self.parse_args(&type_params)?;
        self.expect(&TokenKind::RParen)?;
        // The return type may follow either ':' or '->'. Object, interface and
        // `extend` methods conventionally write `func f(&self) -> str`, while
        // standalone functions write `func f(): void`.
        if matches!(self.peek()?.kind, TokenKind::Colon | TokenKind::Arrow) {
            self.eat()?;
        } else {
            self.expect(&TokenKind::Colon)?;
        }
        let return_type = self.parse_type(&type_params)?;

        if flags.contains(ParserFlags::ONLY_SIGNATURES) {
            // Interface signatures are written without a trailing ';' in the
            // corpus and docs, but tolerate it if present.
            if self.peek()?.kind == TokenKind::SemiColon {
                self.eat()?;
            }
            return Ok(FuncDeclaration {
                attributes,
                visibility: Default::default(),
                span: span.merge_with(return_type.span),
                external: false,
                name,
                type_params,
                args,
                return_type,
                body: vec![],
            });
        }
        let current = self.eat()?;

        //func main(arg:T):Q ->/{}
        match current.kind {
            TokenKind::Arrow => {
                let expr = self.parse_expression(&type_params, flags)?;
                let end = expr
                    .span
                    .merge_with(self.expect(&TokenKind::SemiColon)?.span);
                let body = vec![Spanned::new(
                    self.intern_statement(ASTStatement::Expression(expr)),
                    end,
                )];
                Ok(FuncDeclaration {
                    attributes,
                    visibility: Default::default(),
                    span: span.merge_with(end),
                    name,
                    type_params,
                    args,
                    return_type,
                    body,
                    external: false,
                })
            }
            TokenKind::LBrace => {
                let mut body = vec![];
                while !matches!(self.peek()?.kind, TokenKind::RBrace) {
                    let stmt = self.parse_statement(&type_params)?;
                    body.push(stmt);

                    if self.peek()?.kind == TokenKind::RBrace {
                        continue;
                    }
                    self.finish_current_parse(ParserFlags::REQUIRE_SEMICOLON)?;
                }
                let end = self.expect(&TokenKind::RBrace)?.span;
                Ok(FuncDeclaration {
                    attributes,
                    visibility: Default::default(),
                    external: false,
                    span: span.merge_with(end),
                    name,
                    type_params,
                    args,
                    return_type,
                    body,
                })
            }
            _ => self.unexpected_with(
                format!(
                    "Instead was expecting function body, which initializes with '->' or '{{' {:?}",
                    flags
                ),
                current,
            ),
        }
    }
}
