use std::{collections::HashMap, ops::Deref};

use common::{Span, pool::DedupPoolId};
use slynx_hir::{HIRError, HirType, Result, SlynxHir};

///A struct that handles all the monomorphization on the code
pub struct Monomorphizer {
    reference_cache: HashMap<DedupPoolId<HirType>, DedupPoolId<HirType>>,
}

impl Monomorphizer {
    pub fn resolve(_: &mut SlynxHir) -> Result<()> {
        let _ = Self {
            reference_cache: HashMap::new(),
        };

        Ok(())
    }
    /// Resolves a reference. If the provided `id` is a reference to a concrete type, doesnt do anything, otherwise(thus, a reference)
    /// to another reference) it resolves it to make the reference point to the concrete type. This only caches it for later mutability
    pub fn resolve_reference(
        &mut self,
        hir: &SlynxHir,
        id: DedupPoolId<HirType>,
        span: Span,
    ) -> Result<()> {
        let current = id;

        let cyclic = hir.types_module.is_cyclic(current);
        if cyclic {
            return Err(HIRError::recursive(id, span));
        }

        Ok(())
    }
}
