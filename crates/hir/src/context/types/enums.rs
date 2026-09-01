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
}

impl Debug for EnumsPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnumsPool")
            .field("enums", &self.enums)
            .finish()
    }
}
