use crate::{
    ASTStatement, Spanned, SymbolPointer,
    ast::{ComponentExpression, GenericIdentifier},
};
use common::{Operator, pool::PoolId};
use ordered_float::OrderedFloat;
use smallvec::SmallVec;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
///Simply a name that comes before an expression. It represents anything like 'name: expr', '.name:expr' etc
pub struct NamedExpr {
    pub name: SymbolPointer,
    pub expr: PoolId<ASTExpression>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ASTExpression {
    IntLiteral(i32),
    StringLiteral(SymbolPointer),
    FloatLiteral(OrderedFloat<f32>),
    Identifier(SymbolPointer),
    True,
    False,
    Tuple(SmallVec<[Spanned<PoolId<ASTExpression>>; 2]>),
    TupleAccess {
        tuple: Spanned<PoolId<ASTExpression>>,
        index: u8,
    },
    Component(ComponentExpression),
    Binary {
        lhs: Spanned<PoolId<ASTExpression>>,
        op: Operator,
        rhs: Spanned<PoolId<ASTExpression>>,
    },
    ObjectExpression {
        name: Spanned<PoolId<GenericIdentifier>>,
        fields: SmallVec<[NamedExpr; 4]>,
    },
    FieldAccess {
        parent: Spanned<PoolId<ASTExpression>>,
        field: Spanned<PoolId<ASTExpression>>,
    },
    FunctionCall {
        name: Spanned<PoolId<GenericIdentifier>>,
        args: SmallVec<[Spanned<PoolId<ASTExpression>>; 7]>,
    },
    If {
        condition: Spanned<PoolId<ASTExpression>>,
        body: Vec<Spanned<PoolId<ASTStatement>>>,
        else_body: Vec<Spanned<PoolId<ASTStatement>>>,
    },
}

impl ASTExpression {
    pub fn is_assignable(&self) -> bool {
        matches!(
            self,
            ASTExpression::Identifier(_) | ASTExpression::FieldAccess { .. },
        )
    }
}
