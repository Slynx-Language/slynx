use std::collections::HashMap;

use crate::hir::{
    SlynxHir, TypeId,
    types::{HirType, TypesModule},
};

///A struct that handles all the monomorphization on the code
pub struct Monomorphizer {
    reference_cache: HashMap<TypeId, TypeId>,
}

impl Monomorphizer {
    pub fn resolve(hir: &mut SlynxHir, types_module: &mut TypesModule) {
        let mut this = Self {
            reference_cache: HashMap::new(),
        };
        for decl in hir.declarations.iter() {
            this.resolve_reference(decl.ty, types_module)
        }
        for (key, value) in this.reference_cache {
            let HirType::Reference { rf, .. } = types_module.get_type_mut(&key) else {
                continue;
            };
            println!("{value:?}");
            *rf = value;
        }
    }
    /// Resolves a reference. If the provided `id` is a reference to a concrete type, doesnt do anything, otherwise(thus, a reference)
    /// to another reference) it resolves it to make the reference point to the concrete type. This only caches it for later mutability
    pub fn resolve_reference(&mut self, id: TypeId, types_module: &TypesModule) {
        let mut current = id;
        while let HirType::Reference { rf, .. } = types_module.get_type(&current) {
            // olha o próximo tipo
            match types_module.get_type(rf) {
                HirType::Reference { .. } => {
                    current = *rf; // ainda é referência, continua
                }
                _ => {
                    // próximo NÃO é referência → paramos aqui
                    break;
                }
            }
        }
        if current != id {
            self.reference_cache.insert(id, current);
        }
    }
}
