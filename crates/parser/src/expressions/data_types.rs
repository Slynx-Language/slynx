use common::{Spanned, pool::DedupPoolId};
use slynx_lexer::TokenKind;
use smallvec::{SmallVec, smallvec};

use crate::{
    ASTExpression, ComponentExpression, ComponentMemberValue, NamedExpr, Parser, Result,
    SymbolPointer, Type, flags::ParserFlags,
};

impl Parser<'_> {
    ///Parses an component expression but, starting from the LBrace, assuming the name of the component is the provided `name`
    pub fn parse_component_expr_with_name(
        &mut self,
        name: Spanned<DedupPoolId<Type>>,
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<ComponentExpression>> {
        let mut span = name.span;
        self.expect(&TokenKind::LBrace)?;
        let mut values = Vec::new();
        loop {
            if let Ok(curr) = self.peek()
                && curr.kind == TokenKind::RBrace
            {
                span.end = curr.span.end;
                break;
            };

            match self.peek_at(1)?.kind {
                TokenKind::Colon => {
                    let ident = self.expect_identifier()?;
                    self.expect(&TokenKind::Colon)?;
                    let val = self.parse_expression(type_params, flags)?;
                    values.push(ComponentMemberValue::Assign {
                        prop_name: ident.data,
                        rhs: val,
                    });
                    if self.peek()?.kind == TokenKind::Comma {
                        self.eat()?;
                    }
                }
                _ => {
                    let val = self.parse_component_expr(type_params, flags)?;
                    values.push(ComponentMemberValue::Child(val.data));
                }
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Spanned::new(ComponentExpression { name, values }, span))
    }
    pub fn parse_component_expr(
        &mut self,
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<ComponentExpression>> {
        let ty = self.parse_type(type_params)?;
        self.parse_component_expr_with_name(ty, type_params, flags)
    }

    ///From the current token parses a `NamedExpr`. It starts from the current token supposing it's a identifier,
    ///and parses expecting ':' and then another expression
    pub fn parse_named_expr(
        &mut self,
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<NamedExpr>> {
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::Colon)?;
        let expr = self.parse_expression(type_params, flags)?;
        let span = name.span.merge_with(expr.span);
        Ok(Spanned::new(
            NamedExpr {
                name: name.data,
                expr,
            },
            span,
        ))
    }
    ///Parses a tuple expression, which follows the rule (expr, expr, expr) or ()
    pub fn parse_tuple_with_first(
        &mut self,
        start: Spanned<DedupPoolId<ASTExpression>>,
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let start_span = start.span;

        if self.peek()?.kind == TokenKind::RParen {
            let _ = self.eat()?;
            return Ok(start);
        }
        let mut vec = smallvec![start];
        while self.peek()?.kind != TokenKind::RParen {
            vec.push(self.parse_expression(type_params, flags)?);
            if self.peek()?.kind == TokenKind::Comma {
                self.eat()?;
            }
        }
        let end = self.expect(&TokenKind::RParen)?.span;
        let span = start_span.merge_with(end);
        let id = self.intern_expression(ASTExpression::Tuple(vec));
        Ok(Spanned { data: id, span })
    }
    ///Parses an object expression, which follows the rule Object(field: expr, field: value)
    pub fn parse_object_expression_with_name(
        &mut self,
        name: Spanned<DedupPoolId<Type>>,
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        self.expect(&TokenKind::LParen)?;
        let fields: SmallVec<[Spanned<NamedExpr>; 4]> = self
            .parse_separated(TokenKind::RParen, TokenKind::Comma, true, |parser| {
                parser.parse_named_expr(type_params, flags)
            })?
            .into();
        let end = self.expect(&TokenKind::RParen)?.span;
        let span = name.span.merge_with(end);
        let id = self.intern_expression(ASTExpression::ObjectExpression { name, fields });
        Ok(Spanned::new(id, span))
    }

    pub fn parse_object_expression(
        &mut self,
        type_params: &[SymbolPointer],
        flags: ParserFlags,
    ) -> Result<Spanned<DedupPoolId<ASTExpression>>> {
        let name = self.parse_type(type_params)?;
        self.parse_object_expression_with_name(name, type_params, flags)
    }
}
