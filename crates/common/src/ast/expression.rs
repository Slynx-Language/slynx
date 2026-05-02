use crate::{
    ASTStatement,
    ast::{ComponentExpression, GenericIdentifier, Operator, Span},
};

#[derive(Debug)]
///Simply a name that comes before an expression. It represents anything like 'name: expr', '.name:expr' etc
pub struct NamedExpr {
    pub name: String,
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
    StringLiteral(String),
    FloatLiteral(f32),
    Tuple(Vec<ASTExpression>),
    TupleAccess {
        tuple: Box<ASTExpression>,
        index: usize,
    },
    Boolean(bool),
    Binary {
        lhs: Box<ASTExpression>,
        op: Operator,
        rhs: Box<ASTExpression>,
    },
    Identifier(String),
    ObjectExpression {
        name: GenericIdentifier,
        fields: Vec<NamedExpr>,
    },
    FieldAccess {
        parent: Box<ASTExpression>,
        field: String,
    },
    FunctionCall {
        name: GenericIdentifier,
        args: Vec<ASTExpression>,
    },
    If {
        condition: Box<ASTExpression>,
        body: Vec<ASTStatement>,
        else_body: Option<Vec<ASTStatement>>,
    },
}
