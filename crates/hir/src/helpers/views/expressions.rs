use std::collections::HashMap;

use common::pool::PoolId;

use crate::{
    HirExpression, HirExpressionKind, Owned, VariableId, builders::VariableInfo,
    helpers::HirViewer, term::TermId,
};

impl HirViewer<'_, PoolId<HirExpression>> {
    pub fn raw(&self) -> &HirExpression {
        &self.hir[self.data]
    }
    pub fn ty(&self) -> Owned<TermId> {
        self.hir[self.data].ty
    }
    pub fn ty_viewer(&self) -> HirViewer<'_, TermId> {
        self.new_with(self.ty().term)
    }

    pub fn as_variable(&self) -> Option<VariableId> {
        match &self.hir[self.data].kind {
            HirExpressionKind::Identifier(ident) => Some(*ident),
            _ => None,
        }
    }
}
