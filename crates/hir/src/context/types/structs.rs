use std::ops::Index;

use common::pool::{Pool, PoolId};

use crate::{HirType, StructType, SymbolPointer, TupleType};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct StructDefinition {
    pub(crate) name: SymbolPointer,
    pub(crate) fields: Vec<SymbolPointer>,
}

pub struct StructsPool {
    structs: Pool<StructType>,
    bodies: Pool<StructDefinition>,
    tuples: Pool<TupleType>,
}

impl StructsPool {
    pub fn new() -> Self {
        Self {
            structs: Pool::new(),
            bodies: Pool::new(),
            tuples: Pool::new(),
        }
    }
    pub fn insert(
        &self,
        name: SymbolPointer,
        fields: Vec<(SymbolPointer, PoolId<HirType>)>,
    ) -> PoolId<StructType> {
        let (names, types) = fields.into_iter().unzip();
        let s = StructType { fields: types };
        self.bodies.insert(StructDefinition {
            name,
            fields: names,
        });

        let id = self.structs.insert(s);
        id
    }

    pub fn deffinition_of(&self, ty: PoolId<StructType>) -> &StructDefinition {
        self.bodies.get(unsafe { PoolId::from_raw(ty.inner()) })
    }

    pub fn insert_tuple(&self, fields: Vec<PoolId<HirType>>) -> PoolId<TupleType> {
        self.tuples.insert(TupleType { fields })
    }
}

impl Index<PoolId<StructType>> for StructsPool {
    type Output = StructType;
    fn index(&self, index: PoolId<StructType>) -> &Self::Output {
        self.structs.get(index)
    }
}
impl Index<PoolId<TupleType>> for StructsPool {
    type Output = TupleType;
    fn index(&self, index: PoolId<TupleType>) -> &Self::Output {
        self.tuples.get(index)
    }
}
