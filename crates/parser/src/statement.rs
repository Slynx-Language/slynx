use crate::{Parser, Result, TypeParamScope, flags::ParserFlag};
use slynx_lexer::tokens::TokenKind;

use crate::ast::ASTStatement;
use common::{Span, Spanned, pool::DedupPoolId};

impl Parser<'_> {
    ///Parses a let Statement. Until now it's only for variable declaration, so, this only parses 'let name: t = value;' or 'let name = value;', same for mut variants
    ///Maybe, in the future, more things will be parsed.
    ///Obs: this function should initialize right after 'let' token, and the `letstan` is the span of the 'let' token
    pub fn parse_let_statement(
        &mut self,
        letspan: Span,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTStatement>>> {
        self.add_flag(ParserFlag::RequireSemicolon);
        let mutable = if let TokenKind::Mut = self.peek()?.kind {
            self.eat()?;
            true
        } else {
            false
        };

        let name = self.expect_identifier()?;

        let vartype = match self.peek()?.kind {
            TokenKind::Colon => {
                self.eat()?;
                Some(self.parse_type(type_params)?)
            }
            _ => None,
        };
        self.expect(&TokenKind::Eq)?; //eat '='
        let rhs = self.parse_expression(type_params)?;
        let span = letspan.merge_with(rhs.span);
        let id = self.intern_statment(if mutable {
            ASTStatement::MutableVar {
                name: name.data,
                ty: vartype,
                rhs,
            }
        } else {
            ASTStatement::Var {
                name: name.data,
                ty: vartype,
                rhs,
            }
        });
        Ok(Spanned::new(id, span))
    }

    pub fn parse_while_statement(
        &mut self,
        span: Span,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTStatement>>> {
        self.reset_flags();

        let condition = self.parse_expression(type_params)?;

        let (body, block_span) = self.parse_block(type_params)?;

        let id = self.intern_statment(ASTStatement::While { condition, body });
        Ok(Spanned::new(id, span.merge_with(block_span)))
    }

    pub fn parse_return_statement(
        &mut self,
        span: Span,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTStatement>>> {
        self.add_flag(ParserFlag::RequireSemicolon);
        let value = if matches!(self.peek()?.kind, TokenKind::SemiColon) {
            None
        } else {
            Some(self.parse_expression(type_params)?)
        };
        let end_span = value.as_ref().map(|v| v.span).unwrap_or(span);
        let id = self.intern_statment(ASTStatement::Return { value });
        Ok(Spanned::new(id, span.merge_with(end_span)))
    }

    pub fn parse_statement(
        &mut self,
        type_params: TypeParamScope,
    ) -> Result<Spanned<DedupPoolId<ASTStatement>>> {
        match self.peek()?.kind {
            TokenKind::Let => {
                let span = self.eat()?.span;
                self.parse_let_statement(span, type_params)
            }

            TokenKind::While => {
                let span = self.eat()?.span; //Consume "While"
                self.parse_while_statement(span, type_params)
            }

            TokenKind::Return => {
                let span = self.eat()?.span;
                self.parse_return_statement(span, type_params)
            }

            _ => {
                let expr = self.parse_expression(type_params)?;
                self.add_flag(ParserFlag::RequireSemicolon);
                if matches!(self.peek()?.kind, TokenKind::Eq)
                    && self.expressions.get(expr.data).is_assignable()
                {
                    self.eat()?;
                    let rhs = self.parse_expression(type_params)?;
                    let span = expr.span.merge_with(rhs.span);
                    let id = self.intern_statment(ASTStatement::Assign { lhs: expr, rhs });
                    Ok(Spanned::new(id, span))
                } else {
                    let span = expr.span;
                    let id = self.intern_statment(ASTStatement::Expression(expr));
                    Ok(Spanned::new(id, span))
                }
            }
        }
    }
}
