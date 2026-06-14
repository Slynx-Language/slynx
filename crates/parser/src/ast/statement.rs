use common::Span;

use crate::{ASTExpression, GenericIdentifier, NamedExpr, StyleState};

#[derive(Debug)]
///Some statement on the code, a statement not necessarily have value, in general expressions do.
pub struct ASTStatement {
    pub kind: ASTStatementKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum ASTStatementKind {
    Var {
        name: String,
        ty: Option<GenericIdentifier>,
        rhs: ASTExpression,
    },
    MutableVar {
        name: String,
        ty: Option<GenericIdentifier>,
        rhs: ASTExpression,
    },
    Assign {
        ///The Left hand side of the assign, or, the one that will receive the value of `rhs`
        lhs: ASTExpression,
        rhs: ASTExpression,
    },

    While {
        condition: ASTExpression,
        body: Vec<ASTStatement>,
    },

    Expression(ASTExpression),
}

#[derive(Debug)]
pub struct StyleBlock {
    pub state: StyleState,
    pub properties: Vec<NamedExpr>,
    pub children: Vec<StyleBlock>,
    pub span: Span,
}

#[derive(Debug)]
pub enum StyleSheetStatement {
    Statement(Box<ASTStatement>),
    Styles { styles: Vec<StyleBlock>, span: Span },
}
