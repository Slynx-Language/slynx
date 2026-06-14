use crate::{
    DeclarationId, HirDeclaration, SymbolPointer, TypeId, id::LocalDeclId, module_loader::FileId,
};
use common::VisibilityModifier;
use dashmap::DashMap;
use std::sync::atomic::AtomicU32;

/// A top level Context that keeps track of all the declarations on the Hir.
/// Since declarations are avaible only on the top level this is being implemented by thinking in so
#[derive(Debug, Default)]
pub struct DeclarationsContext {
    next_id: AtomicU32,
    decls: DashMap<LocalDeclId, SymbolPointer>,
    declaration_types: boxcar::Vec<TypeId>,
    visibilities: boxcar::Vec<VisibilityModifier>,
    pub declarations: boxcar::Vec<HirDeclaration>,
    import_aliases: DashMap<SymbolPointer, (DeclarationId, TypeId)>,
}

impl DeclarationsContext {
    /// Creates a new, empty [`DeclarationsContext`].
    pub fn new() -> Self {
        DeclarationsContext {
            next_id: AtomicU32::new(0),
            decls: DashMap::new(),
            declaration_types: boxcar::Vec::new(),
            visibilities: boxcar::Vec::new(),
            declarations: boxcar::Vec::new(),
            import_aliases: DashMap::new(),
        }
    }

    pub fn get_declaration(&self, local: LocalDeclId) -> &HirDeclaration {
        &self.declarations[local.as_raw()]
    }

    pub(crate) fn reserve_id(&self) -> LocalDeclId {
        let next = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        LocalDeclId(next)
    }

    pub fn register_object(
        &self,
        name: SymbolPointer,
        ty: TypeId,
        _fields: Vec<SymbolPointer>,
        visibility: VisibilityModifier,
    ) -> LocalDeclId {
        let id = self.reserve_id();
        self.decls.insert(id, name);
        self.declaration_types.push(ty);
        self.visibilities.push(visibility);
        id
    }

    pub fn register_declaration_metadata(
        &self,
        name: SymbolPointer,
        ty: TypeId,
        visibility: VisibilityModifier,
    ) -> LocalDeclId {
        let id = self.reserve_id();
        self.decls.insert(id, name);
        self.declaration_types.push(ty);
        self.visibilities.push(visibility);
        id
    }

    /// Returns the visibility of the declaration with the given id.
    pub fn get_visibility(&self, id: LocalDeclId) -> VisibilityModifier {
        self.visibilities[id.as_raw()]
    }

    ///Returns the informations of a declaration with the provided `symbol`. The informations are its ID and its type. Returns none if it doesn't exist
    pub fn get_declaration_data_by_name(
        &self,
        symbol: &SymbolPointer,
    ) -> Option<(LocalDeclId, TypeId)> {
        if let Some(symbol) = self.decls.iter().find(|v| v.value() == symbol) {
            let key = *symbol.key();
            Some((key, self.declaration_types[key.as_raw()]))
        } else {
            None
        }
    }

    /// Returns the [`TypeId`] of the declaration with the given [`DeclarationId`].
    ///
    /// # Panics
    ///
    /// Panics if `id` does not correspond to a registered declaration.
    pub fn get_declaration_type(&self, id: LocalDeclId) -> TypeId {
        self.declaration_types[id.as_raw()]
    }

    pub fn all_declaration_count(&self) -> usize {
        self.declaration_types.count()
    }

    /// Registers an import alias so that the `alias` name resolves to the original
    /// declaration identified by `(original_file, original_local, original_ty)`.
    pub fn register_import_alias(
        &self,
        alias: SymbolPointer,
        original_file: FileId,
        original_local: LocalDeclId,
        original_ty: TypeId,
    ) {
        self.import_aliases.insert(
            alias,
            (
                DeclarationId::new(original_file, original_local),
                original_ty,
            ),
        );
    }

    /// If `name` is an import alias, returns the original declaration data.
    pub fn get_import_alias(&self, name: &SymbolPointer) -> Option<(DeclarationId, TypeId)> {
        self.import_aliases.get(name).map(|value| *value.value())
    }
}
