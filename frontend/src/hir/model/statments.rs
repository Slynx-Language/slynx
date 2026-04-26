use common::Span;

use crate::hir::{VariableId, model::HirExpression};

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
