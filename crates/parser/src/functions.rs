use crate::{
    ExpectedContent, FuncDeclaration, Parser, Result, error::ParseError, flags::ParserFlag,
};
use slynx_lexer::tokens::TokenKind;

use crate::ast::{ASTStatement, TypedName};
use common::{Span, Spanned};
impl Parser<'_> {
    ///Parses the arguments of a function. It parses until the `)` of the function args.
    pub fn parse_args(&mut self) -> Result<Vec<Spanned<TypedName>>> {
        let mut names = Vec::new();
        while !matches!(self.peek()?.kind, TokenKind::RParen) {
            names.push(self.parse_typedname()?);
            if matches!(self.peek()?.kind, TokenKind::RParen) {
                break;
            } else {
                self.expect(&TokenKind::Comma)?;
            }
        }

        Ok(names)
    }

    ///Parses a function. The provided `span` is the initial span for the 'func' keyword.
    ///Parses both `func main(arg1:T): Q {...}` and `func main(arg1:T): Q -> ...`
    pub fn parse_func(&mut self, span: Span) -> Result<FuncDeclaration> {
        let name = self.parse_type()?;
        self.expect(&TokenKind::LParen)?;
        let args = self.parse_args()?;
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::Colon)?;
        let return_type = self.parse_type()?;
        if self.flags.has_flag(ParserFlag::OnlySignatures) {
            self.expect(&TokenKind::SemiColon).map_err(|e| {
                let ParseError::UnexpectedToken(tk, _) = e else {
                    unreachable!()
                };
                ParseError::UnexpectedToken(
                    tk,
                    ExpectedContent::ParsingContext(crate::ParserContext::OnlySignatures),
                )
            })?;
            return Ok(FuncDeclaration {
                attributes: vec![],
                visibility: Default::default(),
                span: span.merge_with(return_type.span),
                external: false,
                name,
                args,
                return_type,
                body: vec![],
            });
        }
        let current = self.eat()?;

        //func main(arg:T):Q ->/{}
        match current.kind {
            TokenKind::Arrow => {
                let expr = self.parse_expression()?;
                let end = expr
                    .span
                    .merge_with(self.expect(&TokenKind::SemiColon)?.span);
                let body = vec![Spanned::new(
                    self.intern_statment(ASTStatement::Expression(expr)),
                    end,
                )];
                Ok(FuncDeclaration {
                    attributes: vec![],
                    visibility: Default::default(),
                    span: span.merge_with(end),
                    name,
                    args,
                    return_type,
                    body,
                    external: false,
                })
            }
            TokenKind::LBrace => {
                self.reset_flags();
                let mut body = vec![];
                while !matches!(self.peek()?.kind, TokenKind::RBrace) {
                    let stmt = self.parse_statement()?;
                    body.push(stmt);

                    if self.peek()?.kind == TokenKind::RBrace {
                        continue;
                    }
                    self.finish_current_parse()?;
                }
                let end = self.expect(&TokenKind::RBrace)?.span;
                Ok(FuncDeclaration {
                    attributes: vec![],
                    visibility: Default::default(),
                    external: false,
                    span: span.merge_with(end),
                    name,
                    args,
                    return_type,
                    body,
                })
            }
            _ => Err(ParseError::UnexpectedToken(
                current,
                ExpectedContent::Raw(
                    "Instead was expecting function body, which initializes with '->' or '{'"
                        .to_string(),
                ),
            )),
        }
    }
}
