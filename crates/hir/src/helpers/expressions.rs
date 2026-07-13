use crate::{
    HirType, SlynxHir, SymbolPointer,
    model::{HirExpression, HirExpressionKind},
};
use common::{
    Operator, Spanned,
    pool::{DedupPoolId, PoolId},
};

impl<'a> SlynxHir<'a> {
    /// Creates a string literal expression.
    pub(crate) fn create_strliteral_expression(&self, s: SymbolPointer) -> HirExpression {
        HirExpression {
            ty: self.create_type(HirType::Str),
            kind: HirExpressionKind::StringLiteral(s),
        }
    }

    /// Creates an int expression that must be inferred.
    pub(crate) fn create_int_expression(&self, i: i32, _bitlen: u8) -> HirExpression {
        HirExpression {
            kind: HirExpressionKind::Int(i),
            ty: self.create_type(HirType::Int),
        }
    }

    /// Creates a float expression.
    pub(crate) fn create_float_expression(&self, float: f32) -> HirExpression {
        HirExpression {
            kind: HirExpressionKind::Float(float.into()),
            ty: self.create_type(HirType::Float),
        }
    }
    /// Creates a binary expression.
    pub(crate) fn create_binary_expression(
        &self,
        left: Spanned<PoolId<HirExpression>>,
        right: Spanned<PoolId<HirExpression>>,
        operator: Operator,
        ty: DedupPoolId<HirType>,
    ) -> HirExpression {
        HirExpression {
            kind: HirExpressionKind::Binary {
                lhs: left,
                op: operator,
                rhs: right,
            },
            ty,
        }
    }
}
