use crate::flags::ParserFlags;
use crate::{ASTAttribute, GenericsMetadata, SymbolPointer};
use crate::{FuncDeclaration, Parser, Result};
use slynx_lexer::tokens::TokenKind;

use crate::ast::{ASTStatement, TypedName};
use common::{Span, Spanned, VisibilityModifier};
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
        visibility: VisibilityModifier,
        flags: ParserFlags,
    ) -> Result<FuncDeclaration> {
        let (name, generics) = self.parse_generic_name()?;
        self.expect(&TokenKind::LParen)?;
        let args = self.parse_args(&generics)?;
        self.expect(&TokenKind::RParen)?;
        // The return type may follow either ':' or '->'. Object, interface and
        // `extend` methods conventionally write `func f(&self) -> str`, while
        // standalone functions write `func f(): void`.
        if matches!(self.peek()?.kind, TokenKind::Colon | TokenKind::Arrow) {
            self.eat()?;
        } else {
            self.expect(&TokenKind::Colon)?;
        }
        let return_type = self.parse_type(&generics)?;

        if flags.contains(ParserFlags::ONLY_SIGNATURES) {
            let clauses = self.parse_clauses(&generics)?;
            // Interface signatures are written without a trailing ';' in the
            // corpus and docs, but tolerate it if present.
            if self.peek()?.kind == TokenKind::SemiColon {
                self.eat()?;
            }
            return Ok(FuncDeclaration {
                attributes,
                visibility,
                span: span.merge_with(return_type.span),
                external: false,
                name,
                generics: GenericsMetadata {
                    clauses,
                    type_params: generics,
                    interface_implementations: Vec::new(),
                },
                args,
                return_type,
                body: vec![],
            });
        }
        // The `where` clause list has to be read before the body is taken off the
        // stream: `parse_clauses` looks for a leading `where` and would
        // otherwise be handed the body token instead.
        let clauses = self.parse_clauses(&generics)?;
        let current = self.eat()?;
        //func main(arg:T):Q ->/{}
        match current.kind {
            TokenKind::Arrow => {
                let expr = self.parse_expression(&generics, flags)?;
                let end = expr
                    .span
                    .merge_with(self.expect(&TokenKind::SemiColon)?.span);
                let body = vec![Spanned::new(
                    self.intern_statement(ASTStatement::Expression(expr)),
                    end,
                )];
                Ok(FuncDeclaration {
                    attributes,
                    visibility,
                    span: span.merge_with(end),
                    name,
                    generics: GenericsMetadata {
                        type_params: generics,
                        interface_implementations: Vec::new(),
                        clauses,
                    },
                    args,
                    return_type,
                    body,
                    external: false,
                })
            }
            TokenKind::LBrace => {
                let mut body = vec![];
                // A function body is a block: component literals are allowed and
                // every statement is ';'-terminated. Both are requested here
                // instead of being inherited from `flags`, which also carries
                // `ONLY_SIGNATURES` for declarations inside an `extern` block.
                while !matches!(self.peek()?.kind, TokenKind::RBrace) {
                    let stmt = self.parse_statement(&generics, ParserFlags::COMPONENT_EXPR)?;
                    body.push(stmt);

                    if self.peek()?.kind == TokenKind::RBrace {
                        continue;
                    }
                    self.finish_current_parse(ParserFlags::REQUIRE_SEMICOLON)?;
                }
                let end = self.expect(&TokenKind::RBrace)?.span;
                Ok(FuncDeclaration {
                    attributes,
                    visibility,
                    external: false,
                    span: span.merge_with(end),
                    name,
                    generics: GenericsMetadata {
                        type_params: generics,
                        interface_implementations: Vec::new(),
                        clauses,
                    },
                    args,
                    return_type,
                    body,
                })
            }
            _ => self.unexpected_with(
                "Instead was expecting function body, which initializes with '->' or '{'",
                current,
            ),
        }
    }
}
