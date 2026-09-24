use crate::{
    SlynxHir, SymbolPointer,
    model::{HirExpression, HirExpressionKind},
    term::{Term, TermId},
};
use common::{Operator, Spanned, pool::PoolId};

impl<'a> SlynxHir<'a> {
    /// Creates a string literal expression.
    pub(crate) fn create_strliteral_expression(&self, s: SymbolPointer) -> HirExpression {
        HirExpression {
            ty: self.types.create_type(Term::string_type()),
            kind: HirExpressionKind::StringLiteral(s),
        }
    }

    /// Creates an int expression that must be inferred.
    pub(crate) fn create_int_expression(&self, i: i32, bitlen: u8) -> HirExpression {
        HirExpression {
            kind: HirExpressionKind::Int(i),
            ty: self.types.create_type(Term::signed_integer_type(bitlen)),
        }
    }

    /// Creates a float expression.
    pub(crate) fn create_float_expression(&self, float: f32) -> HirExpression {
        HirExpression {
            kind: HirExpressionKind::Float(float.into()),
            ty: self.types.create_type(Term::float32_type()),
        }
    }
    /// Creates a binary expression.
    pub(crate) fn create_binary_expression(
        &self,
        left: Spanned<PoolId<HirExpression>>,
        right: Spanned<PoolId<HirExpression>>,
        operator: Operator,
        ty: TermId,
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
