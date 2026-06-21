use common::pool::PoolId;

use crate::{ASTExpression, GenericIdentifier, NamedExpr, Spanned, StyleState, SymbolPointer};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ASTStatement {
    Var {
        name: SymbolPointer,
        ty: Option<Spanned<PoolId<GenericIdentifier>>>,
        rhs: Spanned<PoolId<ASTExpression>>,
    },
    MutableVar {
        name: SymbolPointer,
        ty: Option<Spanned<PoolId<GenericIdentifier>>>,
        rhs: Spanned<PoolId<ASTExpression>>,
    },
    Assign {
        ///The Left hand side of the assign, or, the one that will receive the value of `rhs`
        lhs: Spanned<PoolId<ASTExpression>>,
        rhs: Spanned<PoolId<ASTExpression>>,
    },
    While {
        condition: Spanned<PoolId<ASTExpression>>,
        body: Vec<Spanned<PoolId<ASTStatement>>>,
    },

    Expression(Spanned<PoolId<ASTExpression>>),
}

#[derive(Debug)]
pub struct StyleBlock {
    pub state: StyleState,
    pub properties: Vec<Spanned<NamedExpr>>,
    pub children: Vec<Spanned<StyleBlock>>,
}

#[derive(Debug)]
pub enum StyleSheetStatement {
    Statement(Spanned<PoolId<ASTStatement>>),
    Styles(Vec<Spanned<StyleBlock>>),
}
