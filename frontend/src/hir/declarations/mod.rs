use crate::hir::{DeclarationId, TypeId, symbols::SymbolPointer};
use std::collections::HashMap;

/// A top level module that keeps track of all the declarations on the Hir.
/// Since declarations are avaible only on the top level this is being implemented by thinking in so
#[derive(Debug, Default)]
pub struct DeclarationsModule {
    decls: HashMap<SymbolPointer, DeclarationId>,
    ///The types of the declarations. Use a vec because we can access the type based on the inner value of the ID
    declaration_types: Vec<TypeId>,
    pub objects: HashMap<TypeId, Vec<SymbolPointer>>,
}

impl DeclarationsModule {
    pub fn new() -> Self {
        DeclarationsModule {
            decls: HashMap::new(),
            objects: HashMap::new(),
            declaration_types: Vec::new(),
        }
    }
    pub fn create_declaration(&mut self, name: SymbolPointer, ty: TypeId) -> DeclarationId {
        let id = DeclarationId::from_raw(self.declaration_types.len() as u64);
        self.decls.insert(name, id);
        self.declaration_types.push(ty);
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
        self.decls.insert(name, id);
        self.declaration_types.push(ty);
        self.objects.insert(ty, fields);
        id
    }

    ///Returns the informations of a declaration with the provided `symbol`. The informations are its ID and its type. Returns none if it doesn't exist
    pub fn retrieve_declaration_data_by_name(
        &self,
        symbol: &SymbolPointer,
    ) -> Option<(DeclarationId, TypeId)> {
        self.decls
            .get(symbol)
            .map(|decl| (*decl, self.declaration_types[decl.as_raw() as usize]))
    }

    pub fn retrieve_declaration_type(&self, id: DeclarationId) -> TypeId {
        self.declaration_types[id.as_raw() as usize]
    }

    ///Retrieves the body of the object with provided `id`
    pub fn retrieve_object_body(&self, id: TypeId) -> Option<&[SymbolPointer]> {
        self.objects.get(&id).map(|v| &**v)
    }
}
