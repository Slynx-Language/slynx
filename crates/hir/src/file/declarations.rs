use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use crate::{
    HirAliasDeclaration, HirComponentDeclaration, HirFunctionDeclaration, HirObjectDeclaration,
    HirStaticDeclaration, HirStylesheetDeclaration, HirType, SymbolPointer,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
};
use common::{
    pool::{DedupPoolId, Pool, PoolId},
    pooled,
};
use dashmap::DashMap;
use module_loader::FileId;

pooled!(pub DeclarationsPool {
    pub objects: HirObjectDeclaration,
    pub functions: HirFunctionDeclaration,
    pub components: HirComponentDeclaration,
    pub styles: HirStylesheetDeclaration,
    pub alias: HirAliasDeclaration,
    pub statik: HirStaticDeclaration
});

impl Debug for DeclarationsPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeclarationsPool")
            .field("objects", &self.objects)
            .field("functions", &self.functions)
            .field("components", &self.components)
            .field("styles", &self.styles)
            .field("aliases", &self.alias)
            .field("statics", &self.statik)
            .finish()
    }
}

/// A top level Context that keeps track of all the declarations on the Hir.
/// Since declarations are avaible only on the top level this is being implemented by thinking in so
#[derive(Default, Debug)]
pub struct FileDeclarations {
    pub declarations: DeclarationsPool,
    import_aliases: DashMap<SymbolPointer, (AnyDeclarationId, DedupPoolId<HirType>)>,
}

impl FileDeclarations {
    /// Creates a new, empty [`DeclarationsContext`].
    pub fn new() -> Self {
        FileDeclarations {
            declarations: DeclarationsPool::default(),
            import_aliases: DashMap::new(),
        }
    }

    /// Registers an import alias so that the `alias` name resolves to the original
    /// declaration identified by `(original_file, original_local, original_ty)`.
    pub fn register_import_alias(
        &self,
        alias: SymbolPointer,
        original_file: FileId,
        original_local: AnyLocalDeclarationId,
        original_ty: DedupPoolId<HirType>,
    ) {
        self.import_aliases.insert(
            alias,
            (
                AnyDeclarationId::new(original_file, original_local),
                original_ty,
            ),
        );
    }

    /// If `name` is an import alias, returns the original declaration data.
    pub fn get_import_alias(
        &self,
        name: &SymbolPointer,
    ) -> Option<(AnyDeclarationId, DedupPoolId<HirType>)> {
        self.import_aliases.get(name).map(|value| *value.value())
    }
}

impl Deref for FileDeclarations {
    type Target = DeclarationsPool;
    fn deref(&self) -> &Self::Target {
        &self.declarations
    }
}
impl DerefMut for FileDeclarations {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.declarations
    }
}
