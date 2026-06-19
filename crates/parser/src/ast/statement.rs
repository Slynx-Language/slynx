use common::{Span, pool::PoolId};

use crate::{ASTExpression, GenericIdentifier, NamedExpr, StyleState, SymbolPointer};

#[derive(Debug)]
///Some statement on the code, a statement not necessarily have value, in general expressions do.
pub struct ASTStatement {
    pub kind: ASTStatementKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum ASTStatementKind {
    Var {
        name: SymbolPointer,
        ty: Option<GenericIdentifier>,
        rhs: PoolId<ASTExpression>,
    },
    MutableVar {
        name: SymbolPointer,
        ty: Option<GenericIdentifier>,
        rhs: PoolId<ASTExpression>,
    },
    Assign {
        ///The Left hand side of the assign, or, the one that will receive the value of `rhs`
        lhs: PoolId<ASTExpression>,
        rhs: PoolId<ASTExpression>,
    },

    While {
        condition: PoolId<ASTExpression>,
        body: Vec<ASTStatement>,
    },

    Expression(PoolId<ASTExpression>),
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
