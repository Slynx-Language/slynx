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

impl HirStatement {
    ///Creates a new return statment
    pub fn new_return(expr: HirExpression) -> Self {
        Self {
            span: expr.span,
            kind: HirStatementKind::Return { expr },
        }
    }
    pub fn new_variable(name: VariableId, value: HirExpression, span: Span) -> Self {
        Self {
            kind: HirStatementKind::Variable { name, value },
            span,
        }
    }

    pub fn new_while(condition: HirExpression, body: Vec<HirStatement>, span: Span) -> Self {
        Self {
            span,
            kind: HirStatementKind::While { condition, body },
        }
    }
    pub fn new_expression(expr: HirExpression) -> Self {
        Self {
            span: expr.span,
            kind: HirStatementKind::Expression { expr },
        }
    }
}
