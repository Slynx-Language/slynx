use common::{
    Operator, Span, Spanned,
    pool::{DedupPoolId, PoolId},
};
use either::Either;
use slynx_parser::{ASTExpression, TypeContext};

use crate::{
    HIRError, HirExpression, HirExpressionKind, Owned, Result, SymbolPointer,
    builders::HirQueueBuilder,
    error::NotMutableReason,
    helpers::IntegerTypeInfo,
    term::{PrimitiveType, Term, TermId, TermNode},
};

use super::{ExpressionBuilder, ExpressionDescriptor};

pub struct ReferenceExpressionDescriptor<'a> {
    ///The target we will build the reference for.
    pub target: Either<Spanned<DedupPoolId<ASTExpression>>, Spanned<PoolId<HirExpression>>>,
    ///If the reference is mutable or not
    pub mutable: bool,
    ///The context of the types to this expression
    pub context: &'a TypeContext<'a>,
}

pub struct DereferenceExpressionDescriptor<'a> {
    ///The target expression to dereference.
    pub target: Spanned<DedupPoolId<ASTExpression>>,
    ///The context of the types to this expression
    pub context: &'a TypeContext<'a>,
    ///The expected type of the dereference, if any. If some, and the generated type does not match, an error will be returned.
    pub expected: Option<TermId>,
}

pub struct NegateExpressionDescriptor<'a> {
    ///The expression to negate
    pub target: Spanned<DedupPoolId<ASTExpression>>,
    ///The span of the whole negation expression
    pub span: Span,
    ///The context of the types to this expression
    pub context: &'a TypeContext<'a>,
    ///The expected type of the negation
    pub expected: Option<TermId>,
}

impl ExpressionBuilder {
    pub(super) fn build_deref(
        &mut self,
        queue: &HirQueueBuilder,
        descriptor: DereferenceExpressionDescriptor<'_>,
    ) -> Result<HirExpression> {
        let build_expr = self.build_expression(
            queue,
            ExpressionDescriptor {
                target: descriptor.target,
                expected: descriptor.expected,
                context: descriptor.context,
            },
        )?;
        let expr_ty = queue.hir.view(build_expr.data).ty();
        let inner = match queue.hir.view(expr_ty.term).raw().node() {
            TermNode::Ref { target, .. } => *target,
            _ => {
                return Err(HIRError::invalid_deref(self.file(), descriptor.target.span));
            }
        };
        Ok(HirExpression {
            ty: Owned::new(expr_ty.owner, inner),
            kind: HirExpressionKind::Deref(build_expr),
        })
    }

    pub(super) fn build_negate(
        &mut self,
        queue: &HirQueueBuilder,
        descriptor: NegateExpressionDescriptor<'_>,
    ) -> Result<HirExpression> {
        if let Some(expected) = descriptor.expected
            && let Some(integer_info) = queue.hir.view(expected).is_integer()
            && !integer_info.signed
        {
            return Err(HIRError::invalid_unary_expression(
                self.file(),
                expected,
                descriptor.span,
            ));
        }
        let build_expr = self.build_expression(
            queue,
            ExpressionDescriptor {
                target: descriptor.target,
                expected: descriptor.expected,
                context: descriptor.context,
            },
        )?;
        let expr_ty = queue.hir.view(build_expr.data).ty();
        match &queue.hir.view(build_expr.data).raw().kind {
            HirExpressionKind::Int(value) => {
                return Ok(HirExpression {
                    ty: expr_ty,
                    kind: HirExpressionKind::Int(value.wrapping_neg()),
                });
            }
            HirExpressionKind::Float(value) => {
                return Ok(HirExpression {
                    ty: expr_ty,
                    kind: HirExpressionKind::Float(-*value),
                });
            }
            _ => {}
        }
        let zero = match queue.hir.view(expr_ty.term).raw().node() {
            TermNode::Primitive(PrimitiveType::Signed { .. }) => HirExpressionKind::Int(0),
            TermNode::Primitive(PrimitiveType::Float32 | PrimitiveType::Float64) => {
                HirExpressionKind::Float(0.0.into())
            }
            _ => {
                return Err(HIRError::invalid_unary_expression(
                    self.file(),
                    expr_ty.term,
                    descriptor.span,
                ));
            }
        };
        let zero = queue.hir.store.insert_expression(HirExpression {
            ty: expr_ty,
            kind: zero,
        });
        Ok(queue.hir.create_binary_expression(
            self.file(),
            descriptor.span.make_spanned(zero),
            build_expr,
            Operator::Sub,
            expr_ty.term,
        ))
    }

    pub(super) fn build_reference_expression(
        &mut self,
        queue: &HirQueueBuilder,
        descriptor: ReferenceExpressionDescriptor,
    ) -> Result<Spanned<PoolId<HirExpression>>> {
        let hir_expression = match descriptor.target {
            Either::Left(ast_expression) => self.build_expression(
                queue,
                ExpressionDescriptor {
                    target: ast_expression,
                    expected: None,
                    context: descriptor.context,
                },
            )?,
            Either::Right(hir_expression) => hir_expression,
        };
        let expression_viewer = queue.hir.view(hir_expression.data);
        let final_type = {
            let ty = expression_viewer.ty();
            let reference = if descriptor.mutable {
                self.is_expression_able_to_write(queue, hir_expression)?;
                Term::mutable_reference(ty.term)
            } else {
                Term::reference(ty.term)
            };
            Owned::new(ty.owner, queue.hir.types.create_type(reference))
        };

        let out = HirExpression {
            ty: final_type,
            kind: HirExpressionKind::Reference(hir_expression),
        };

        let out = queue.hir.store.insert_expression(out);
        Ok(hir_expression.span.make_spanned(out))
    }

    pub(super) fn build_bool(&self, queue: &HirQueueBuilder, value: bool) -> HirExpression {
        let ty = Owned::new(
            self.file(),
            queue.hir.types.create_type(Term::boolean_type()),
        );
        HirExpression {
            ty,
            kind: if value {
                HirExpressionKind::True
            } else {
                HirExpressionKind::False
            },
        }
    }

    pub(super) fn build_int_literal(
        &self,
        queue: &HirQueueBuilder,
        value: i32,
        expected: Option<TermId>,
        span: Span,
    ) -> Result<HirExpression> {
        let numeric_info = if let Some(expected) = expected {
            if queue.hir.view(expected).is_generic().is_some() {
                IntegerTypeInfo::default()
            } else if let Some(integer_info) = queue.hir.view(expected).is_integer() {
                integer_info
            } else {
                return Err(HIRError::expected_integer_type(self.file(), expected, span));
            }
        } else {
            IntegerTypeInfo::default()
        };

        let (min, max) = {
            let min = if numeric_info.signed {
                -2i128.pow(numeric_info.bit_length as u32 - 1)
            } else {
                0
            };
            let max = if numeric_info.signed {
                2i128.pow(numeric_info.bit_length as u32 - 1)
            } else {
                2i128.pow(numeric_info.bit_length as u32)
            } - 1;
            (min, max)
        };
        let hir_type = queue.hir.types.create_type(if numeric_info.signed {
            Term::signed_integer_type(numeric_info.bit_length)
        } else {
            Term::unsigned_integer_type(numeric_info.bit_length)
        });
        {
            let value128 = value as i128;
            if value128 < min || value128 > max {
                return Err(HIRError::integer_out_of_bounds(
                    self.file(),
                    value as i64,
                    hir_type,
                    min,
                    max,
                    span,
                ));
            }
        }

        Ok(queue
            .hir
            .create_int_expression(self.file(), value, hir_type))
    }

    pub(super) fn build_str_literal(
        &self,
        queue: &HirQueueBuilder,
        value: SymbolPointer,
    ) -> HirExpression {
        queue.hir.create_strliteral_expression(self.file(), value)
    }
}
