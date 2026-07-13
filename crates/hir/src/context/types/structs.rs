use std::fmt::Debug;

use common::{
    dedup_pooled,
    pool::{DedupPool, DedupPoolId},
};

use crate::{
    DeclarationId, HirFunctionDeclaration, HirType, StructType, SymbolPointer, TupleType,
    helpers::Visible,
};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct StructDefinition {
    pub(crate) name: SymbolPointer,
    pub(crate) fields: Vec<Visible<SymbolPointer>>,
    pub(crate) methods: Vec<Visible<(SymbolPointer, DeclarationId<HirFunctionDeclaration>)>>,
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
        fields: Vec<Visible<(SymbolPointer, DedupPoolId<HirType>)>>,
        methods: Vec<Visible<(SymbolPointer, DeclarationId<HirFunctionDeclaration>)>>,
    ) -> (DedupPoolId<StructType>, DedupPoolId<StructDefinition>) {
        let (names, types) = fields
            .into_iter()
            .map(|v| (Visible::new(v.visibility, v.data.0), v.data.1))
            .collect();
        let def_id = self.bodies.insert(StructDefinition {
            name,
            fields: names,
            methods,
        });
        let s = StructType {
            fields: types,
            metadata: def_id,
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
