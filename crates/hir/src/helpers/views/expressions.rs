use std::collections::HashMap;

use common::pool::{DedupPoolId, PoolId};

use crate::{
    HirExpression, HirExpressionKind, HirType, VariableId, builders::VariableInfo,
    helpers::HirViewer,
};

impl HirViewer<'_, PoolId<HirExpression>> {
    pub fn raw(&self) -> &HirExpression {
        &self.hir[self.data]
    }
    pub fn ty(&self) -> DedupPoolId<HirType> {
        self.hir[self.data].ty
    }
    pub fn ty_viewer(&self) -> HirViewer<'_, DedupPoolId<HirType>> {
        self.new_with(self.ty())
    }
    pub fn is_able_to_mutability(&self, vars: &HashMap<VariableId, VariableInfo>) -> bool {
        match &self.hir[self.data].kind {
            HirExpressionKind::Identifier(ident) if let Some(var) = vars.get(ident) => {
                var.mutable
                    || self
                        .new_with(vars[ident].type_id)
                        .is_mutable_ref()
                        .is_some()
            }

            _ => true,
        }
    }
    pub fn as_variable(&self) -> Option<VariableId> {
        match &self.hir[self.data].kind {
            HirExpressionKind::Identifier(ident) => Some(*ident),
            _ => None,
        }
    }
}
