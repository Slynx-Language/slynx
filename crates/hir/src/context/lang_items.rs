use common::Span;
use dashmap::DashMap;

use crate::{
    DeclarationId, HIRError, HirAliasDeclaration, HirComponentDeclaration, HirEnumDeclaration,
    HirExtendDeclaration, HirFunctionDeclaration, HirObjectDeclaration, HirStaticDeclaration,
    LanguageItem, Result, SymbolPointer,
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
    pub aliases: LangMap<HirAliasDeclaration>,
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
            Err(HIRError::already_defined(content.owner, name, span))
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
        T::map(self).0.get(&name).map(|value| *value)
    }
    /// Looks a language item up, failing when it was never registered.
    ///
    /// `file` is the file the lookup was requested from: an unregistered
    /// intrinsic is reported against the code that used it, not against
    /// whichever file the registry happens to live in.
    pub fn get<T: LanguageItem>(
        &self,
        name: SymbolPointer,
        span: common::Span,
        file: module_loader::FileId,
    ) -> Result<DeclarationId<T>> {
        self.try_get(name)
            .ok_or_else(|| HIRError::intrinsic_not_registered(file, name, span))
    }
}
