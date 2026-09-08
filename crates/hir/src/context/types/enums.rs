use std::fmt::Debug;

use common::{
    dedup_pooled,
    pool::{DedupPool, DedupPoolId},
};

use crate::{EnumType, EnumVariantType, SymbolPointer};

dedup_pooled!(pub EnumsPool {
    enums: EnumType,
});

impl EnumsPool {
    ///Inserts a new [`EnumType`] into this pool, deduplicating by full equality.
    pub fn insert(
        &self,
        name: SymbolPointer,
        variants: Vec<EnumVariantType>,
    ) -> DedupPoolId<EnumType> {
        self.enums.insert(EnumType { name, variants })
    }

    ///Finds the enum type registered under `name`, if any.
    ///
    ///This is used to make [`TypesContext::create_enum_type`] idempotent by
    ///name: rebuilding an enum that already exists must return the *existing*
    ///type rather than registering a duplicate under the same name.
    pub fn find_by_name(&self, name: SymbolPointer) -> Option<DedupPoolId<EnumType>> {
        self.enums
            .iter()
            .find_map(|(id, ty)| (ty.name == name).then_some(id))
    }
}

impl Debug for EnumsPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnumsPool")
            .field("enums", &self.enums)
            .finish()
    }
}
