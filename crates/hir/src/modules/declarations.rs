use crate::{DeclarationId, SymbolPointer, TypeId};
use std::collections::HashMap;

/// A top level module that keeps track of all the declarations on the Hir.
/// Since declarations are avaible only on the top level this is being implemented by thinking in so
#[derive(Debug, Default)]
pub struct DeclarationsModule {
    decls: HashMap<DeclarationId, SymbolPointer>,
    ///The types of the declarations. Use a vec because we can access the type based on the inner value of the ID
    declaration_types: Vec<TypeId>,
    /// Maps each object [`TypeId`] to its ordered list of field symbol pointers.
    pub objects: HashMap<TypeId, Vec<SymbolPointer>>,
    ///Maps a enum type to its variants
    pub enums: HashMap<TypeId, Vec<SymbolPointer>>,
}

impl DeclarationsModule {
    /// Creates a new, empty [`DeclarationsModule`].
    pub fn new() -> Self {
        DeclarationsModule {
            decls: HashMap::new(),
            objects: HashMap::new(),
            declaration_types: Vec::new(),
            enums: HashMap::new(),
        }
    }
    /// Registers a new declaration with the given name symbol and type, returning its [`DeclarationId`].
    pub fn create_declaration(&mut self, name: SymbolPointer, ty: TypeId) -> DeclarationId {
        let id = DeclarationId::from_raw(self.declaration_types.len() as u64);
        self.decls.insert(id, name);
        self.declaration_types.push(ty);
        id
    }

    pub fn create_enum(
        &mut self,
        name: SymbolPointer,
        variants: Vec<SymbolPointer>,
        ty: TypeId,
    ) -> DeclarationId {
        let id = DeclarationId::from_raw(self.declaration_types.len() as u64);
        self.decls.insert(id, name);
        self.declaration_types.push(ty);
        self.enums.insert(ty, variants);
        id
    }

    ///Creates an objct with the provided `name`, `ty` and `fields` and returns it's id
    pub fn create_object(
        &mut self,
        name: SymbolPointer,
        ty: TypeId,
        fields: Vec<SymbolPointer>,
    ) -> DeclarationId {
        let id = DeclarationId::from_raw(self.declaration_types.len() as u64);
        self.decls.insert(id, name);
        self.declaration_types.push(ty);
        self.objects.insert(ty, fields);
        id
    }

    ///Returns the informations of a declaration with the provided `symbol`. The informations are its ID and its type. Returns none if it doesn't exist
    pub fn get_declaration_data_by_name(
        &self,
        symbol: &SymbolPointer,
    ) -> Option<(DeclarationId, TypeId)> {
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
    pub fn get_declaration_type(&self, id: DeclarationId) -> TypeId {
        self.declaration_types[id.as_raw() as usize]
    }

    ///Retrieves the body of the object with provided `id`
    pub fn get_object_body(&self, id: TypeId) -> Option<&[SymbolPointer]> {
        self.objects.get(&id).map(|v| &**v)
    }
}
