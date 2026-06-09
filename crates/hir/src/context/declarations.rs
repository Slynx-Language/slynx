use crate::{
    DeclarationId, HirDeclaration, SymbolPointer, TypeId, id::LocalDeclId, module_loader::FileId,
};
use common::VisibilityModifier;
use std::collections::HashMap;

/// A top level Context that keeps track of all the declarations on the Hir.
/// Since declarations are avaible only on the top level this is being implemented by thinking in so
#[derive(Debug, Default)]
pub struct DeclarationsContext {
    next_id: u32,
    decls: HashMap<LocalDeclId, SymbolPointer>,
    declaration_types: Vec<TypeId>,
    visibilities: Vec<VisibilityModifier>,
    pub declarations: Vec<HirDeclaration>,
    import_aliases: HashMap<SymbolPointer, (DeclarationId, TypeId)>,
}

impl DeclarationsContext {
    /// Creates a new, empty [`DeclarationsContext`].
    pub fn new() -> Self {
        DeclarationsContext {
            next_id: 0,
            decls: HashMap::new(),
            declaration_types: Vec::new(),
            visibilities: Vec::new(),
            declarations: Vec::new(),
            import_aliases: HashMap::new(),
        }
    }

    pub(crate) fn reserve_id(&mut self) -> LocalDeclId {
        let out = LocalDeclId(self.next_id);
        self.next_id += 1;
        out
    }

    pub fn register_object(
        &mut self,
        name: SymbolPointer,
        ty: TypeId,
        _fields: Vec<SymbolPointer>,
    ) -> LocalDeclId {
        let id = self.reserve_id();
        self.decls.insert(id, name);
        self.declaration_types.push(ty);
        self.visibilities.push(VisibilityModifier::Private);
        id
    }

    pub fn register_declaration_metadata(
        &mut self,
        name: SymbolPointer,
        ty: TypeId,
    ) -> LocalDeclId {
        let id = self.reserve_id();
        self.decls.insert(id, name);
        self.declaration_types.push(ty);
        self.visibilities.push(VisibilityModifier::Private);
        id
    }

    /// Sets the visibility of the declaration with the given id.
    pub fn set_visibility(&mut self, id: LocalDeclId, visibility: VisibilityModifier) {
        self.visibilities[id.as_raw() as usize] = visibility;
    }

    /// Returns the visibility of the declaration with the given id.
    pub fn get_visibility(&self, id: LocalDeclId) -> VisibilityModifier {
        self.visibilities[id.as_raw() as usize]
    }

    ///Returns the informations of a declaration with the provided `symbol`. The informations are its ID and its type. Returns none if it doesn't exist
    pub fn get_declaration_data_by_name(
        &self,
        symbol: &SymbolPointer,
    ) -> Option<(LocalDeclId, TypeId)> {
        self.decls
            .iter()
            .find(|v| v.1 == symbol)
            .map(|(decl, _)| (*decl, self.declaration_types[decl.as_raw() as usize]))
    }

    /// Returns the [`TypeId`] of the declaration with the given [`DeclarationId`].
    ///
    /// # Panics
    ///
    /// Panics if `id` does not correspond to a registered declaration.
    pub fn get_declaration_type(&self, id: LocalDeclId) -> TypeId {
        self.declaration_types[id.as_raw() as usize]
    }

    /// Registers an import alias so that the `alias` name resolves to the original
    /// declaration identified by `(original_file, original_local, original_ty)`.
    pub fn register_import_alias(
        &mut self,
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
        self.import_aliases.get(name).copied()
    }
}
