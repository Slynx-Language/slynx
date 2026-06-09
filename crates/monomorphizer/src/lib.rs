use std::collections::{HashMap, HashSet};

use common::Span;
use slynx_hir::{HIRError, HirType, Result, SlynxHir, TypeId};

///A struct that handles all the monomorphization on the code
pub struct Monomorphizer {
    reference_cache: HashMap<TypeId, TypeId>,
}

impl Monomorphizer {
    pub fn resolve(hir: &mut SlynxHir) -> Result<()> {
        let mut this = Self {
            reference_cache: HashMap::new(),
        };
        for file in &hir.files {
            for decl in file.declarations() {
                this.resolve_reference(hir, decl.ty, decl.span)?;
            }
        }
        for (key, value) in this.reference_cache {
            let HirType::Reference { rf, .. } = hir.get_type_mut(&key) else {
                continue;
            };
            *rf = value;
        }
        Ok(())
    }
    /// Resolves a reference. If the provided `id` is a reference to a concrete type, doesnt do anything, otherwise(thus, a reference)
    /// to another reference) it resolves it to make the reference point to the concrete type. This only caches it for later mutability
    pub fn resolve_reference(&mut self, hir: &SlynxHir, id: TypeId, span: Span) -> Result<()> {
        let mut current = id;
        let mut visited = HashSet::from([id]);
        while let HirType::Reference { rf, .. } = hir.get_type(&current)
            && let HirType::Reference { .. } = hir.get_type(rf)
        {
            if !visited.insert(*rf) {
                let name = hir
                    .get_name_of_type(*rf)
                    .expect("Expected type to have a name");

                return Err(HIRError::recursive(name, span));
            }
            current = *rf;
        }
        if current != id {
            self.reference_cache.insert(id, current);
        }
        Ok(())
    }
}
