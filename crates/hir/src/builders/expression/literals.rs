use std::ops::Deref;

use common::{
    Span, Spanned,
    pool::{DedupPoolId, PoolId},
};
use either::Either;
use slynx_parser::{ASTExpression, TypeContext};

use crate::{
    HIRError, HirExpression, HirExpressionKind, HirType, Result, SymbolPointer,
    builders::HirQueueBuilder, error::NotMutableReason,
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
    pub expected: Option<DedupPoolId<HirType>>,
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
        let expr_ty = match queue.hir.view(expr_ty).raw() {
            HirType::ImutableRef(inner) | HirType::MutableRef(inner) => *inner,
            _ => {
                return Err(HIRError::invalid_deref(descriptor.target.span));
            }
        };
        Ok(HirExpression {
            ty: expr_ty,
            kind: HirExpressionKind::Deref(build_expr),
        })
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
            let ty = if descriptor.mutable {
                HirType::MutableRef(ty)
            } else {
                HirType::ImutableRef(ty)
            };
            queue.hir.create_type(ty)
        };
        let able_to_mutate = expression_viewer.is_able_to_mutability(&self.variables);
        let out = HirExpression {
            ty: final_type,
            kind: HirExpressionKind::Reference(hir_expression),
        };
        match (descriptor.mutable, able_to_mutate) {
            (true, false) if let Some(var) = expression_viewer.as_variable() => {
                let name = self
                    .variable_name(var)
                    .expect("Variable should contain a name");
                Err(HIRError::expression_not_mutable(
                    NotMutableReason::ImmutableVariable(name),
                    hir_expression.span,
                ))
            }
            (true, false) => Err(HIRError::expression_not_mutable(
                NotMutableReason::ExpressionNotAssignable,
                hir_expression.span,
            )),
            _ => {
                let out = queue.hir.insert_expression(out);
                Ok(hir_expression.span.make_spanned(out))
            }
        }
    }

    pub(super) fn build_null(
        &self,
        queue: &HirQueueBuilder,
        span: Span,
        expected: Option<DedupPoolId<HirType>>,
    ) -> Result<HirExpression> {
        let ty = match expected {
            None => {
                return Err(HIRError::couldnt_infer(span));
            }
            Some(ty) if let HirType::Nullable(_) = queue.hir.deref()[ty] => ty,
            Some(ty) => {
                return Err(HIRError::unexpected_type(
                    ty,
                    queue.hir.create_type(HirType::Nullable(ty)),
                    span,
                ));
            }
        };
        Ok(HirExpression {
            ty,
            kind: HirExpressionKind::Null,
        })
    }

    pub(super) fn build_bool(&self, queue: &HirQueueBuilder, value: bool) -> HirExpression {
        HirExpression {
            ty: queue.hir.create_type(HirType::Bool),
            kind: if value {
                HirExpressionKind::True
            } else {
                HirExpressionKind::False
            },
        }
    }

    pub(super) fn build_int_literal(&self, queue: &HirQueueBuilder, value: i32) -> HirExpression {
        queue.hir.create_int_expression(value, 0)
    }

    pub(super) fn build_float_literal(&self, queue: &HirQueueBuilder, value: f32) -> HirExpression {
        queue.hir.create_float_expression(value)
    }

    pub(super) fn build_str_literal(
        &self,
        queue: &HirQueueBuilder,
        value: SymbolPointer,
    ) -> HirExpression {
        queue.hir.create_strliteral_expression(value)
    }
}
