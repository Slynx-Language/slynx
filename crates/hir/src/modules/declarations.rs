use crate::{HirDeclaration, SymbolPointer, TypeId, id::LocalDeclId};
use std::collections::HashMap;

/// A top level module that keeps track of all the declarations on the Hir.
/// Since declarations are avaible only on the top level this is being implemented by thinking in so
#[derive(Debug, Default)]
pub struct DeclarationsModule {
    next_id: u32,
    decls: HashMap<LocalDeclId, SymbolPointer>,
    ///The types of the declarations. Use a vec because we can access the type based on the inner value of the ID
    declaration_types: Vec<TypeId>,
    /// Maps each object [`TypeId`] to its ordered list of field symbol pointers.
    pub objects: HashMap<TypeId, Vec<SymbolPointer>>,
    declarations: Vec<HirDeclaration>,
}

impl DeclarationsModule {
    /// Creates a new, empty [`DeclarationsModule`].
    pub fn new() -> Self {
        DeclarationsModule {
            next_id: 0,
            decls: HashMap::new(),
            objects: HashMap::new(),
            declaration_types: Vec::new(),
            declarations: Vec::new(),
        }
    }

    ///Reserves a new local id for this declarations module. This is mainly usefull when dealing with hoisting
    pub(crate) fn reserve_id(&mut self) -> LocalDeclId {
        let out = LocalDeclId(self.next_id);
        self.next_id += 1;
        out
    }

    /// Registers a new declaration with the given name symbol and type, returning its [`DeclarationId`].
    pub fn register_declaration_metadata(
        &mut self,
        name: SymbolPointer,
        ty: TypeId,
    ) -> LocalDeclId {
        let id = self.reserve_id();
        self.decls.insert(id, name);
        self.declaration_types.push(ty);
        id
    }
    ///Creates an objct with the provided `name`, `ty` and `fields` and returns it's id
    pub fn register_object(
        &mut self,
        name: SymbolPointer,
        ty: TypeId,
        fields: Vec<SymbolPointer>,
    ) -> LocalDeclId {
        let id = self.reserve_id();
        self.decls.insert(id, name);
        self.declaration_types.push(ty);
        self.objects.insert(ty, fields);
        id
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

    ///Retrieves the body of the object with provided `id`
    pub fn get_object_body(&self, id: TypeId) -> Option<&[SymbolPointer]> {
        self.objects.get(&id).map(|v| &**v)
    }
}
