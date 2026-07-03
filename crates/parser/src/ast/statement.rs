use common::{Spanned, pool::DedupPoolId};

use crate::{ASTExpression, GenericIdentifier, NamedExpr, StyleState, SymbolPointer};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ASTStatement {
    Var {
        name: SymbolPointer,
        ty: Option<Spanned<DedupPoolId<GenericIdentifier>>>,
        rhs: Spanned<DedupPoolId<ASTExpression>>,
    },
    MutableVar {
        name: SymbolPointer,
        ty: Option<Spanned<DedupPoolId<GenericIdentifier>>>,
        rhs: Spanned<DedupPoolId<ASTExpression>>,
    },
    Assign {
        ///The Left hand side of the assign, or, the one that will receive the value of `rhs`
        lhs: Spanned<DedupPoolId<ASTExpression>>,
        rhs: Spanned<DedupPoolId<ASTExpression>>,
    },
    While {
        condition: Spanned<DedupPoolId<ASTExpression>>,
        body: Vec<Spanned<DedupPoolId<ASTStatement>>>,
    },

    Expression(Spanned<DedupPoolId<ASTExpression>>),
}

#[derive(Debug)]
pub struct StyleBlock {
    pub state: StyleState,
    pub properties: Vec<Spanned<NamedExpr>>,
    pub children: Vec<Spanned<StyleBlock>>,
}

#[derive(Debug)]
pub enum StyleSheetStatement {
    Statement(Spanned<DedupPoolId<ASTStatement>>),
    Styles(Vec<Spanned<StyleBlock>>),
}
