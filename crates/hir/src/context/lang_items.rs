use common::Span;
use dashmap::DashMap;

use crate::{
    DeclarationId, HIRError, HirComponentDeclaration, HirEnumDeclaration, HirExtendDeclaration,
    HirFunctionDeclaration, HirObjectDeclaration, HirStaticDeclaration, LanguageItem, Result,
    SymbolPointer,
};

#[derive(Debug)]
pub struct LangMap<T: std::fmt::Debug>(DashMap<SymbolPointer, DeclarationId<T>>);

#[derive(Debug, Default)]
///A struct to map intrinsic functions, objects, etc
pub struct LangItems {
    pub objects: LangMap<HirObjectDeclaration>,
    pub functions: LangMap<HirFunctionDeclaration>,
    pub enums: LangMap<HirEnumDeclaration>,
    pub components: LangMap<HirComponentDeclaration>,
    pub statics: LangMap<HirStaticDeclaration>,
    pub extensions: LangMap<HirExtendDeclaration>,
}

impl<T: std::fmt::Debug> std::default::Default for LangMap<T> {
    fn default() -> Self {
        Self(DashMap::new())
    }
}

impl<T> LangMap<T>
where
    T: LanguageItem,
{
    pub fn register(
        &self,
        name: SymbolPointer,
        content: DeclarationId<T>,
        span: Span,
    ) -> Result<()> {
        if self.0.insert(name, content).is_some() {
            Err(HIRError::already_defined(name, span))
        } else {
            Ok(())
        }
    }
}

impl LangItems {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register<T: LanguageItem>(
        &self,
        name: SymbolPointer,
        id: DeclarationId<T>,
        span: common::Span,
    ) -> Result<()> {
        let map = T::map(self);
        map.register(name, id, span)
    }
    pub fn try_get<T: LanguageItem>(&self, name: SymbolPointer) -> Option<DeclarationId<T>> {
        T::map(self).0.get(&name).map(|value| value.clone())
    }
    pub fn get<T: LanguageItem>(
        &self,
        name: SymbolPointer,
        span: common::Span,
    ) -> Result<DeclarationId<T>> {
        self.try_get(name)
            .ok_or_else(|| HIRError::intrinsic_not_registered(name, span))
    }
}
