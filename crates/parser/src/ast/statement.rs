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
        lhs: Spanned<DedupPoolId<ASTExpression>>,
        rhs: Spanned<DedupPoolId<ASTExpression>>,
    },
    While {
        condition: Spanned<DedupPoolId<ASTExpression>>,
        body: Vec<Spanned<DedupPoolId<ASTStatement>>>,
    },
    Return {
        value: Option<Spanned<DedupPoolId<ASTExpression>>>,
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
