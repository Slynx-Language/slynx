use crate::{
    Owned, SlynxHir, SymbolPointer,
    model::{HirExpression, HirExpressionKind},
    term::{Term, TermId},
};
use common::{Operator, Spanned, pool::PoolId};
use module_loader::FileId;

impl<'a> SlynxHir<'a> {
    /// Creates a string literal expression owned by `owner`.
    pub(crate) fn create_strliteral_expression(
        &self,
        owner: FileId,
        s: SymbolPointer,
    ) -> HirExpression {
        HirExpression {
            ty: Owned::new(owner, self.types.create_type(Term::string_type())),
            kind: HirExpressionKind::StringLiteral(s),
        }
    }

    /// Creates an int expression that must be inferred, owned by `owner`.
    pub(crate) fn create_int_expression(&self, owner: FileId, i: i32, bitlen: u8) -> HirExpression {
        HirExpression {
            kind: HirExpressionKind::Int(i),
            ty: Owned::new(
                owner,
                self.types.create_type(Term::signed_integer_type(bitlen)),
            ),
        }
    }

    /// Creates a float expression owned by `owner`.
    pub(crate) fn create_float_expression(&self, owner: FileId, float: f32) -> HirExpression {
        HirExpression {
            kind: HirExpressionKind::Float(float.into()),
            ty: Owned::new(owner, self.types.create_type(Term::float32_type())),
        }
    }
    /// Creates a binary expression owned by `owner`.
    pub(crate) fn create_binary_expression(
        &self,
        owner: FileId,
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
            ty: Owned::new(owner, ty),
        }
    }
}
