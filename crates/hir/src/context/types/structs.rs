use std::fmt::Debug;

use common::{
    dedup_pooled,
    pool::{DedupPool, DedupPoolId},
};

use crate::{DeclarationId, HirFunctionDeclaration, HirType, StructType, SymbolPointer, TupleType};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct StructDefinition {
    pub(crate) name: SymbolPointer,
    pub(crate) fields: Vec<SymbolPointer>,
}

dedup_pooled!(pub StructsPool {
    structs: StructType,
    bodies: StructDefinition,
    tuples: TupleType,
});

impl StructsPool {
    pub fn insert(
        &self,
        name: SymbolPointer,
        fields: Vec<(SymbolPointer, DedupPoolId<HirType>)>,
        methods: Vec<(SymbolPointer, DeclarationId<HirFunctionDeclaration>)>,
    ) -> (DedupPoolId<StructType>, DedupPoolId<StructDefinition>) {
        let (names, types) = fields.into_iter().unzip();
        let def_id = self.bodies.insert(StructDefinition {
            name,
            fields: names,
        });
        let s = StructType {
            fields: types,
            metadata: def_id,
            methods,
        };

        let id = self.structs.insert(s);
        (id, def_id)
    }
}

impl Debug for StructsPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StructsPool")
            .field("structs", &self.structs)
            .field("bodies", &self.bodies)
            .field("tuples", &self.tuples)
            .finish()
    }
}
