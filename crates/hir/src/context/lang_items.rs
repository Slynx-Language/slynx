use dashmap::DashMap;

use crate::{HIRError, SymbolPointer, id::AnyDeclarationId};

#[derive(Debug, Default)]
///A struct to map intrinsic functions, objects, etc
pub struct LangItems {
    declarations: DashMap<SymbolPointer, AnyDeclarationId>,
}

impl LangItems {
    pub fn new() -> Self {
        Self {
            declarations: DashMap::new(),
        }
    }
    pub fn register(
        &self,
        name: SymbolPointer,
        id: AnyDeclarationId,
        span: common::Span,
    ) -> Result<(), HIRError> {
        if self.declarations.insert(name, id).is_some() {
            Err(HIRError::already_defined(name, span))
        } else {
            Ok(())
        }
    }
    pub fn try_get(&self, name: SymbolPointer) -> Option<AnyDeclarationId> {
        self.declarations.get(&name).map(|v| *v)
    }
    pub fn get(
        &self,
        name: SymbolPointer,
        span: common::Span,
    ) -> Result<AnyDeclarationId, HIRError> {
        self.try_get(name)
            .ok_or_else(|| HIRError::intrinsic_not_registered(name, span))
    }
}
