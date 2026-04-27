use common::Span;

use crate::hir::{
    ExpressionId, SlynxHir, TypeId,
    model::{HirExpression, HirExpressionKind},
};

impl SlynxHir {
    pub fn create_tuple_expression(
        &self,
        tuple_ty: TypeId,
        values: Vec<HirExpression>,
        span: Span,
    ) -> HirExpression {
        HirExpression {
            id: ExpressionId::new(),
            ty: tuple_ty,
            kind: HirExpressionKind::Tuple(values),
            span,
        }
    }
}
