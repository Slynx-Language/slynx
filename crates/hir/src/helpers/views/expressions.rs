use std::collections::{HashMap, HashSet};

use common::pool::{DedupPoolId, PoolId};

use crate::{HirExpression, HirExpressionKind, HirType, VariableId, helpers::HirViewer};

impl HirViewer<'_, PoolId<HirExpression>> {
    pub fn ty(&self) -> DedupPoolId<HirType> {
        self.hir[self.data].ty
    }
    pub fn ty_viewer(&self) -> HirViewer<'_, DedupPoolId<HirType>> {
        self.new_with(self.ty())
    }
    pub fn is_able_to_mutability(
        &self,
        mutables: &HashSet<VariableId>,
        variable_types: &HashMap<VariableId, DedupPoolId<HirType>>,
    ) -> bool {
        match &self.hir[self.data].kind {
            HirExpressionKind::Identifier(ident) => {
                mutables.contains(ident)
                    || self
                        .new_with(variable_types[ident])
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
