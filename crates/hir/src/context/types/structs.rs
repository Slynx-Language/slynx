use std::fmt::Debug;

use common::{
    dedup_pooled,
    pool::{DedupPool, DedupPoolId},
};

use crate::{
    DeclarationId, HirFunctionDeclaration, StructField, StructMethod, StructType, SymbolPointer,
    TupleType, helpers::Visible, term::TermId,
};

use common::DedupPoolStorage;
dedup_pooled!(pub StructsPool {
    structs: StructType,
    tuples: TupleType,
});

impl StructsPool {
    pub fn insert_struct(
        &self,
        name: SymbolPointer,
        fields: Vec<Visible<(SymbolPointer, TermId)>>,
        methods: Vec<Visible<(SymbolPointer, DeclarationId<HirFunctionDeclaration>)>>,
    ) -> DedupPoolId<StructType> {
        let fields = fields
            .into_iter()
            .map(|field| {
                Visible::new(
                    field.visibility,
                    StructField {
                        name: field.data.0,
                        ty: field.data.1,
                    },
                )
            })
            .collect();
        let methods = methods
            .into_iter()
            .map(|field| {
                Visible::new(
                    field.visibility,
                    StructMethod {
                        name: field.data.0,
                        target: field.data.1,
                    },
                )
            })
            .collect();

        let s = StructType {
            name,
            fields,
            methods,
        };

        self.structs.insert(s)
    }
}

impl Debug for StructsPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StructsPool")
            .field("structs", &self.structs)
            .field("tuples", &self.tuples)
            .finish()
    }
}
