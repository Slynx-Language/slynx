mod components;
mod structs;
use std::{
    collections::{HashSet, VecDeque},
    ops::Index,
};

use common::{
    Span,
    pool::{Pool, PoolId},
};
use dashmap::{DashMap, DashSet};

use components::*;
use structs::*;

use crate::{
    ComponentType, DeclarationId, HIRError, HirType, Result, StructType, SymbolPointer, TupleType,
    VariableId,
};

/// Manages all types in the HIR, including built-ins, user-defined types, and variables.
pub struct TypesContext {
    ///Maps a variable to it's type
    pub variables: DashMap<VariableId, PoolId<HirType>>,
    pub names: DashMap<SymbolPointer, PoolId<HirType>>,
    pub methods: DashMap<PoolId<HirType>, DashMap<SymbolPointer, DeclarationId>>,
    /// Maps (parent_type, method_name) -> return_type for external object methods.
    external_methods: DashMap<(PoolId<HirType>, SymbolPointer), PoolId<HirType>>,

    /// Set of PoolId<HirType>s that are external (from JS/interop).
    /// When a type is marked external, all references to it are also external.
    externals: DashSet<PoolId<HirType>>,
    structs: StructsPool,
    components: ComponentsPool,
    types: Pool<HirType>,
}
impl TypesContext {
    /// Creates a new [`TypesContext`] with built-in types pre-registered under the given symbol names.
    pub fn new() -> Self {
        Self {
            names: DashMap::new(),
            variables: DashMap::new(),
            methods: DashMap::new(),
            external_methods: DashMap::new(),
            externals: DashSet::new(),
            types: Pool::new(),
            structs: StructsPool::new(),
            components: ComponentsPool::new(),
        }
    }

    ///Inserts a new variable on this Context
    pub fn insert_variable(&self, varid: VariableId, ty: PoolId<HirType>) {
        self.variables.insert(varid, ty);
    }
    /// Creates a new tuple type with the given field types and returns its [`TypeId`].
    pub fn create_tuple_type(&self, fields: Vec<PoolId<HirType>>) -> PoolId<HirType> {
        let tuple = self.structs.insert_tuple(fields);
        self.types.insert(HirType::Tuple(tuple))
    }
    pub fn create_struct(
        &self,
        name: SymbolPointer,
        fields: Vec<(SymbolPointer, PoolId<HirType>)>,
    ) -> PoolId<HirType> {
        let id = self.structs.insert(name, fields);
        let id = self.create_type(HirType::Struct(id));
        self.names.insert(name, id);
        id
    }
    pub fn create_alias(&self, name: SymbolPointer, ty: HirType) -> PoolId<HirType> {
        let id = self.create_type(ty);
        self.names.insert(name, id);
        id
    }
    ///Inserts the provided `ty` to have the provided `name`
    pub fn create_type(&self, ty: HirType) -> PoolId<HirType> {
        self.types.insert(ty)
    }

    ///Returns the inner object from the provided `ty`, returns None if the type is not a object
    pub fn get_object(&self, ty: PoolId<HirType>) -> Option<PoolId<HirType>> {
        let mut visited = HashSet::new();
        let mut current = ty;
        loop {
            if !visited.insert(current) {
                return None;
            }

            match self[current] {
                HirType::Struct { .. } => return Some(current),
                HirType::Reference { rf, .. } => current = *rf,
                _ => return None,
            }
        }
    }

    ///Returns the inner component from the provided `ty`, returns None if the type is not a object
    pub fn get_component(&self, ty: &PoolId<HirType>) -> Option<PoolId<HirType>> {
        let mut visited = HashSet::new();
        let mut current = *ty;
        loop {
            if !visited.insert(current) {
                return None;
            }

            match self[current] {
                HirType::Component { .. } => return Some(current),
                HirType::Reference { rf, .. } => current = *rf,
                _ => return None,
            }
        }
    }

    ///Registers a method for the given `ty` on the current declaration context with the given `name` that points to the given `id`. It should be asserted by the HIR to be a function ID
    pub fn create_method(&self, ty: PoolId<HirType>, name: SymbolPointer, id: DeclarationId) {
        if !self.methods.contains_key(&ty) {
            self.methods.insert(ty, DashMap::new());
        }
        self.methods.get(&ty).unwrap().insert(name, id);
    }

    /// Register an external method's return type without creating a declaration entry.
    pub fn register_external_method(
        &self,
        parent_ty: PoolId<HirType>,
        name: SymbolPointer,
        return_type: PoolId<HirType>,
    ) {
        self.external_methods.insert((parent_ty, name), return_type);
    }

    /// Returns the return type of an external method on `parent_ty` with the given `name`.
    pub fn get_method_return_type(
        &self,
        parent_ty: &PoolId<HirType>,
        name: SymbolPointer,
    ) -> Option<PoolId<HirType>> {
        self.external_methods
            .get(&(*parent_ty, name))
            .map(|ret| *ret.value())
    }

    ///Registers a method for the given `ty` on the current declaration context with the given `name` that points to the given `id`. It should be asserted by the HIR to be a function ID
    pub fn get_methods_of(&self, ty: PoolId<HirType>) -> Vec<(SymbolPointer, DeclarationId)> {
        if let Some(methos_map) = self.methods.get(&ty) {
            methos_map
                .iter()
                .map(|entry| (*entry.key(), *entry.value()))
                .collect()
        } else {
            Vec::new()
        }
    }

    ///Retrieves the PoolId<HirType> of the provided `name` on the currentContext
    pub fn get_id_of_name(&self, name: &SymbolPointer) -> Option<PoolId<HirType>> {
        self.names.get(name).map(|v| v.value().clone())
    }
    pub fn get_struct_deffinition(&self, s: PoolId<StructType>) -> SymbolPointer {
        self.structs.deffinition_of(s).name
    }

    /// Returns the [`PoolId<HirType>`] of the given variable, if it has been registered.
    pub fn get_variable(&self, id: &VariableId) -> Option<PoolId<HirType>> {
        self.variables.get(id).map(|v| *v.value())
    }

    /// Returns a mutable reference to the [`HirType`] for the given [`PoolId<HirType>`].
    ///
    /// # Panics
    ///
    /// Panics if `id` does not correspond to a registered type.
    pub fn get_type_mut(&self, id: PoolId<HirType>) -> &HirType {
        self.types.get(id)
    }
    /// Returns a mutable reference to the [`HirType`] associated with the given name symbol, if it exists.

    ///Retrieves the body of the object with provided `id`
    pub fn get_object_body(&self, id: PoolId<StructType>) -> &[SymbolPointer] {
        &self.structs.deffinition_of(id).fields
    }
    ///Retrieves the type of something by asserting the provided `ref_ty` is a reference type to it
    pub fn get_type_from_ref(
        &self,
        ref_ty: PoolId<HirType>,
        span: &Span,
    ) -> Result<PoolId<HirType>> {
        let mut visited = HashSet::new();
        let mut current = ref_ty;
        loop {
            match self[current] {
                HirType::Reference { rf, .. } => {
                    if !visited.insert(current) {
                        return Err(HIRError::recursive(current, *span));
                    }
                    current = *rf;
                }
                _ => return Ok(current),
            }
        }
    }

    pub fn is_cyclic(&self, ty: PoolId<HirType>) -> bool {
        let mut set = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(ty);
        while let Some(ty) = queue.pop_front() {
            if !set.insert(ty) {
                return true;
            }

            match self[ty] {
                HirType::Reference { rf, .. } => {
                    queue.push_back(*rf);
                }
                HirType::Struct(id) => {
                    for field in &self[id].fields {
                        queue.push_back(*field);
                    }
                }
                HirType::Tuple(id) => {
                    for field in &self[id].fields {
                        queue.push_back(*field);
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Mark a type as external. Also traverses `Reference` wrappers to mark
    /// the inner struct type, so that all layers of indirection are covered.
    pub fn mark_external(&self, ty: PoolId<HirType>) {
        self.externals.insert(ty);
        let mut current = ty;
        loop {
            match self[current] {
                HirType::Reference { rf, .. } => {
                    self.externals.insert(*rf);
                    current = *rf;
                }
                _ => break,
            }
        }
    }

    /// Returns `true` if the given type (or any `Reference` it wraps) has
    /// been marked as external.
    pub fn is_external(&self, ty: &PoolId<HirType>) -> bool {
        if self.externals.contains(ty) {
            return true;
        }

        let mut current = *ty;
        loop {
            match self[current] {
                HirType::Reference { rf, .. } => {
                    if self.externals.contains(rf) {
                        return true;
                    }
                    current = *rf;
                }
                _ => return false,
            }
        }
    }
}

impl Index<PoolId<HirType>> for TypesContext {
    type Output = HirType;
    fn index(&self, index: PoolId<HirType>) -> &Self::Output {
        self.types.get(index)
    }
}

impl Index<PoolId<StructType>> for TypesContext {
    type Output = StructType;
    fn index(&self, index: PoolId<StructType>) -> &Self::Output {
        &self.structs[index]
    }
}

impl Index<PoolId<TupleType>> for TypesContext {
    type Output = TupleType;
    fn index(&self, index: PoolId<TupleType>) -> &Self::Output {
        &self.structs[index]
    }
}

impl Index<PoolId<ComponentType>> for TypesContext {
    type Output = ComponentType;
    fn index(&self, index: PoolId<ComponentType>) -> &Self::Output {
        &self.components[index]
    }
}
