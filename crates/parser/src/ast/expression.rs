use crate::{
    ASTStatement, SymbolPointer,
    ast::{ComponentExpression, GenericIdentifier},
};
use common::{Operator, Span, pool::PoolId};

#[derive(Debug)]
///Simply a name that comes before an expression. It represents anything like 'name: expr', '.name:expr' etc
pub struct NamedExpr {
    pub name: SymbolPointer,
    pub expr: ASTExpression,
    pub span: Span,
}

#[derive(Debug)]
pub struct ASTExpression {
    pub kind: ASTExpressionKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum ASTExpressionKind {
    Component(ComponentExpression),
    IntLiteral(i32),
    StringLiteral(SymbolPointer),
    FloatLiteral(f32),
    Tuple(Vec<ASTExpression>),
    TupleAccess {
        tuple: PoolId<ASTExpression>,
        index: usize,
    },
    Boolean(bool),
    Binary {
        lhs: PoolId<ASTExpression>,
        op: Operator,
        rhs: PoolId<ASTExpression>,
    },
    Identifier(SymbolPointer),
    ObjectExpression {
        name: GenericIdentifier,
        fields: Vec<NamedExpr>,
    },
    FieldAccess {
        parent: PoolId<ASTExpression>,
        field: PoolId<ASTExpression>,
    },
    FunctionCall {
        name: GenericIdentifier,
        args: Vec<ASTExpression>,
    },
    If {
        condition: PoolId<ASTExpression>,
        body: Vec<ASTStatement>,
        else_body: Option<Vec<ASTStatement>>,
    },
}
