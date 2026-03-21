use crate::hir::{DeclarationId, ExpressionId, PropertyId, TypeId, VariableId};

use common::ast::{Operator, Span};

#[derive(Debug)]
pub enum SpecializedComponent {
    Text {
        text: Box<HirExpression>,
    },
    Div {
        children: Vec<ComponentMemberDeclaration>,
    },
}

#[derive(Debug)]
#[repr(C)]
pub struct HirDeclaration {
    pub kind: HirDeclarationKind,
    pub id: DeclarationId, // Changed from HirId
    pub ty: TypeId,
    pub span: Span,
}

#[derive(Debug)]
#[repr(C)]
pub enum HirDeclarationKind {
    Object,
    Function {
        statements: Vec<HirStatement>,
        args: Vec<VariableId>, // Changed from HirId - function arguments are variables
        name: String,
    },
    ComponentDeclaration {
        props: Vec<ComponentMemberDeclaration>,
    },
}

#[derive(Debug)]
#[repr(C)]
pub enum ComponentMemberDeclaration {
    Property {
        id: PropertyId, // Changed from HirId
        /// The index of the property on the component
        index: usize,
        value: Option<HirExpression>,
        span: Span,
    },
    Child {
        name: TypeId, // Still HirId - reference to a component declaration
        values: Vec<ComponentMemberDeclaration>,
        span: Span,
    },
    Specialized(SpecializedComponent),
}

#[derive(Debug)]
#[repr(C)]
pub struct HirStatement {
    pub kind: HirStatementKind,
    pub span: Span,
}

#[derive(Debug)]
#[repr(C)]
pub enum HirStatementKind {
    Assign {
        lhs: HirExpression,
        value: HirExpression,
    },
    Variable {
        name: VariableId,
        value: HirExpression, //the type of the variable is the type of this expression
    },
    Expression {
        expr: HirExpression,
    },
    Return {
        expr: HirExpression,
    },

    While {
    condition: HirExpression,
    body: Vec<HirStatement>,
},
}

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
