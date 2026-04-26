use common::{Operator, Span};

use crate::hir::{
    DeclarationId, ExpressionId, TypeId, VariableId,
    model::{ComponentMemberDeclaration, HirStatement, SpecializedComponent},
};

#[derive(Debug)]
#[repr(C)]
pub struct HirExpression {
    pub id: ExpressionId,
    pub ty: TypeId,
    pub kind: HirExpressionKind,
    pub span: Span,
}

#[derive(Debug)]
#[repr(C)]
pub enum HirExpressionKind {
    Int(i32),
    StringLiteral(String),
    Float(f32),
    Bool(bool),
    Tuple(Vec<HirExpression>),
    Binary {
        lhs: Box<HirExpression>,
        op: Operator,
        rhs: Box<HirExpression>,
    },
    Identifier(VariableId),
    Specialized(SpecializedComponent),
    Component {
        name: TypeId,
        values: Vec<ComponentMemberDeclaration>,
    },
    Object {
        name: TypeId,
        fields: Vec<HirExpression>,
    },
    FieldAccess {
        expr: Box<HirExpression>,
        field_index: usize,
    },
    FunctionCall {
        name: DeclarationId,
        args: Vec<HirExpression>,
    },
    If {
        condition: Box<HirExpression>,
        then_branch: Vec<HirStatement>,
        else_branch: Option<Vec<HirStatement>>,
    },
}
