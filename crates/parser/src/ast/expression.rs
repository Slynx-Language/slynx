use crate::{
    ASTStatement, SymbolPointer,
    ast::{ComponentExpression, Type},
};
use common::{Operator, Spanned, pool::DedupPoolId};
use ordered_float::OrderedFloat;
use smallvec::SmallVec;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
///Simply a name that comes before an expression. It represents anything like 'name: expr', '.name:expr' etc
pub struct NamedExpr {
    pub name: SymbolPointer,
    pub expr: Spanned<DedupPoolId<ASTExpression>>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ASTExpression {
    IntLiteral(i32),
    StringLiteral(SymbolPointer),
    FloatLiteral(OrderedFloat<f32>),
    Identifier(SymbolPointer),
    True,
    False,
    Tuple(SmallVec<[Spanned<DedupPoolId<ASTExpression>>; 2]>),
    TupleAccess {
        tuple: Spanned<DedupPoolId<ASTExpression>>,
        index: u8,
    },
    Component(ComponentExpression),
    Binary {
        lhs: Spanned<DedupPoolId<ASTExpression>>,
        op: Operator,
        rhs: Spanned<DedupPoolId<ASTExpression>>,
    },
    ObjectExpression {
        name: Spanned<DedupPoolId<Type>>,
        fields: SmallVec<[Spanned<NamedExpr>; 4]>,
    },
    FieldAccess {
        parent: Spanned<DedupPoolId<ASTExpression>>,
        field: Spanned<DedupPoolId<ASTExpression>>,
    },
    FunctionCall {
        name: Spanned<DedupPoolId<Type>>,
        args: SmallVec<[Spanned<DedupPoolId<ASTExpression>>; 7]>,
    },
    If {
        condition: Spanned<DedupPoolId<ASTExpression>>,
        body: Vec<Spanned<DedupPoolId<ASTStatement>>>,
        else_body: Vec<Spanned<DedupPoolId<ASTStatement>>>,
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
