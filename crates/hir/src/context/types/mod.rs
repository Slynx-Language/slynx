mod components;
mod structs;
use std::{
    collections::{HashSet, VecDeque},
    ops::Index,
};

use common::{
    Span,
    pool::{DedupPool, DedupPoolId},
};
use dashmap::{DashMap, DashSet};

use crate::{
    ComponentType, DeclarationId, FunctionType, HIRError, HirFunctionDeclaration, HirType, Result,
    StructType, StyleType, SymbolPointer, TupleType, VariableId,
};
pub use components::ComponentDefinition;
use components::*;
pub use structs::StructDefinition;
use structs::*;

#[derive(Debug)]
/// Manages all types in the HIR, including built-ins, user-defined types, and variables.
pub struct TypesContext {
    ///Maps a variable to it's type
    pub variables: DashMap<VariableId, DedupPoolId<HirType>>,
    pub names: DashMap<SymbolPointer, DedupPoolId<HirType>>,
    pub methods: DashMap<
        DedupPoolId<HirType>,
        DashMap<SymbolPointer, DeclarationId<HirFunctionDeclaration>>,
    >,
    /// Maps (parent_type, method_name) -> return_type for external object methods.
    external_methods: DashMap<(DedupPoolId<HirType>, SymbolPointer), DedupPoolId<HirType>>,
    /// Set of DedupPoolId<HirType>s that are external (from JS/interop).
    /// When a type is marked external, all references to it are also external.
    externals: DashSet<DedupPoolId<HirType>>,
    structs: StructsPool,
    components: ComponentsPool,
    functions: DedupPool<FunctionType>,
    styles: DedupPool<StyleType>,
    types: DedupPool<HirType>,
}
impl TypesContext {
    /// Creates a new [`TypesContext`] with built-in types pre-registered under the given symbol names.
    pub fn new() -> Self {
        Self {
            styles: DedupPool::new(),
            names: DashMap::new(),
            variables: DashMap::new(),
            methods: DashMap::new(),
            external_methods: DashMap::new(),
            externals: DashSet::new(),
            types: DedupPool::new(),
            functions: DedupPool::new(),
            structs: StructsPool::default(),
            components: ComponentsPool::default(),
        }
    }

    ///Inserts a new variable on this Context
    pub fn insert_variable(&self, varid: VariableId, ty: DedupPoolId<HirType>) {
        self.variables.insert(varid, ty);
    }

    pub fn create_function_type(
        &self,
        args: Vec<DedupPoolId<HirType>>,
        ret: DedupPoolId<HirType>,
    ) -> DedupPoolId<HirType> {
        let fid = self.functions.insert(FunctionType {
            args: args.into(),
            ret,
        });
        self.create_type(HirType::Function(fid))
    }

    /// Creates a new tuple type with the given field types and returns its [`TypeId`].
    pub fn create_tuple_type(&self, fields: Vec<DedupPoolId<HirType>>) -> DedupPoolId<HirType> {
        let tuple = self.structs.insert_at_tuples(TupleType { fields });
        self.types.insert(HirType::Tuple(tuple))
    }

    pub fn create_struct_type(
        &self,
        name: SymbolPointer,
        fields: Vec<(SymbolPointer, DedupPoolId<HirType>)>,
        methods: Vec<(SymbolPointer, DeclarationId<HirFunctionDeclaration>)>,
    ) -> DedupPoolId<HirType> {
        let (id, _) = self.structs.insert(name, fields, methods);
        let id = self.create_type(HirType::Struct(id));
        self.names.insert(name, id);
        id
    }

    pub fn create_component_type(
        &self,
        name: SymbolPointer,
        properties: Vec<(SymbolPointer, DedupPoolId<HirType>)>,
        children: Vec<DedupPoolId<ComponentType>>,
    ) -> DedupPoolId<HirType> {
        let (comp_ty, _) = self.components.insert(name, properties, children);
        let id = self.create_type(HirType::Component(comp_ty));
        self.names.insert(name, id);
        id
    }

    pub fn get_component_definition(
        &self,
        comp: DedupPoolId<ComponentType>,
    ) -> &ComponentDefinition {
        let meta = self.components[comp].metadata;
        &self.components[meta]
    }

    pub fn create_alias_type(&self, name: SymbolPointer, ty: HirType) -> DedupPoolId<HirType> {
        let id = self.create_type(ty);
        self.names.insert(name, id);
        id
    }

    ///Inserts the provided `ty` to have the provided `name`
    pub fn create_type(&self, ty: HirType) -> DedupPoolId<HirType> {
        self.types.insert(ty)
    }

    ///Returns the inner object from the provided `ty`, returns None if the type is not a object
    pub fn get_object(&self, ty: DedupPoolId<HirType>) -> Option<DedupPoolId<HirType>> {
        let mut visited = HashSet::new();
        let mut current = ty;
        loop {
            if !visited.insert(current) {
                return None;
            }

            match self[current] {
                HirType::Struct { .. } => return Some(current),
                HirType::Reference { rf, .. } => current = rf,
                _ => return None,
            }
        }
    }

    ///Returns the inner component from the provided `ty`, returns None if the type is not a object
    pub fn get_component(&self, ty: &DedupPoolId<HirType>) -> Option<DedupPoolId<HirType>> {
        let mut visited = HashSet::new();
        let mut current = *ty;
        loop {
            if !visited.insert(current) {
                return None;
            }

            match self[current] {
                HirType::Component { .. } => return Some(current),
                HirType::Reference { rf, .. } => current = rf,
                _ => return None,
            }
        }
    }

    ///Registers a method for the given `ty` on the current declaration context with the given `name` that points to the given `id`. It should be asserted by the HIR to be a function ID
    pub fn create_method(
        &self,
        ty: DedupPoolId<HirType>,
        name: SymbolPointer,
        id: DeclarationId<HirFunctionDeclaration>,
    ) {
        if !self.methods.contains_key(&ty) {
            self.methods.insert(ty, DashMap::new());
        }
        self.methods.get(&ty).unwrap().insert(name, id);
    }

    /// Register an external method's return type without creating a declaration entry.
    pub fn register_external_method(
        &self,
        parent_ty: DedupPoolId<HirType>,
        name: SymbolPointer,
        return_type: DedupPoolId<HirType>,
    ) {
        self.external_methods.insert((parent_ty, name), return_type);
    }

    /// Returns the return type of an external method on `parent_ty` with the given `name`.
    pub fn get_method_return_type(
        &self,
        parent_ty: &DedupPoolId<HirType>,
        name: SymbolPointer,
    ) -> Option<DedupPoolId<HirType>> {
        self.external_methods
            .get(&(*parent_ty, name))
            .map(|ret| *ret.value())
    }

    ///Registers a method for the given `ty` on the current declaration context with the given `name` that points to the given `id`. It should be asserted by the HIR to be a function ID
    pub fn get_methods_of(
        &self,
        ty: DedupPoolId<HirType>,
    ) -> Vec<(SymbolPointer, DeclarationId<HirFunctionDeclaration>)> {
        if let Some(methos_map) = self.methods.get(&ty) {
            let mut out = Vec::with_capacity(methos_map.len());
            for entry in methos_map.iter() {
                let (key, value) = entry.pair();
                out.push((*key, value.clone()));
            }
            out
        } else {
            Vec::new()
        }
    }

    ///Retrieves the DedupPoolId<HirType> of the provided `name` on the currentContext
    pub fn get_id_of_name(&self, name: &SymbolPointer) -> Option<DedupPoolId<HirType>> {
        self.names.get(name).map(|v| v.value().clone())
    }
    pub fn get_struct_name(&self, s: DedupPoolId<StructType>) -> SymbolPointer {
        let metadata = self.structs[s].metadata;
        self.structs[metadata].name
    }
    pub fn get_struct_fields(&self, s: DedupPoolId<StructType>) -> &[SymbolPointer] {
        let metadata = self.structs[s].metadata;
        &self.structs[metadata].fields
    }

    pub fn get_struct_field_types(&self, s: DedupPoolId<StructType>) -> &[DedupPoolId<HirType>] {
        &self.structs[s].fields
    }

    pub fn get_struct_signature(
        &self,
        s: DedupPoolId<StructType>,
    ) -> Vec<(&SymbolPointer, &DedupPoolId<HirType>)> {
        self.get_struct_fields(s)
            .iter()
            .zip(&self.structs[s].fields)
            .collect()
    }

    /// Returns the [`DedupPoolId<HirType>`] of the given variable, if it has been registered.
    pub fn get_variable(&self, id: &VariableId) -> Option<DedupPoolId<HirType>> {
        self.variables.get(id).map(|v| *v.value())
    }

    ///Retrieves the type of something by asserting the provided `ref_ty` is a reference type to it
    pub fn get_type_from_ref(
        &self,
        ref_ty: DedupPoolId<HirType>,
        span: &Span,
    ) -> Result<DedupPoolId<HirType>> {
        let mut visited = HashSet::new();
        let mut current = ref_ty;
        loop {
            match self[current] {
                HirType::Reference { rf, .. } => {
                    if !visited.insert(current) {
                        return Err(HIRError::recursive(current, *span));
                    }
                    current = rf;
                }
                _ => return Ok(current),
            }
        }
    }

    pub fn is_cyclic(&self, ty: DedupPoolId<HirType>) -> bool {
        let mut set = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(ty);
        while let Some(ty) = queue.pop_front() {
            if !set.insert(ty) {
                return true;
            }

            match self[ty] {
                HirType::Reference { rf, .. } => {
                    queue.push_back(rf);
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
    pub fn mark_external(&self, ty: DedupPoolId<HirType>) {
        self.externals.insert(ty);
        let mut current = ty;
        loop {
            match self[current] {
                HirType::Reference { rf, .. } => {
                    self.externals.insert(rf);
                    current = rf;
                }
                _ => break,
            }
        }
    }

    /// Returns `true` if the given type (or any `Reference` it wraps) has
    /// been marked as external.
    pub fn is_external(&self, ty: &DedupPoolId<HirType>) -> bool {
        if self.externals.contains(ty) {
            return true;
        }

        let mut current = *ty;
        loop {
            match self[current] {
                HirType::Reference { rf, .. } => {
                    if self.externals.contains(&rf) {
                        return true;
                    }
                    current = rf;
                }
                _ => return false,
            }
        }
    }
}

impl Index<DedupPoolId<HirType>> for TypesContext {
    type Output = HirType;
    fn index(&self, index: DedupPoolId<HirType>) -> &Self::Output {
        self.types.get(index)
    }
}

impl Index<DedupPoolId<StructType>> for TypesContext {
    type Output = StructType;
    fn index(&self, index: DedupPoolId<StructType>) -> &Self::Output {
        &self.structs[index]
    }
}

impl Index<DedupPoolId<TupleType>> for TypesContext {
    type Output = TupleType;
    fn index(&self, index: DedupPoolId<TupleType>) -> &Self::Output {
        &self.structs[index]
    }
}

impl Index<DedupPoolId<ComponentType>> for TypesContext {
    type Output = ComponentType;
    fn index(&self, index: DedupPoolId<ComponentType>) -> &Self::Output {
        &self.components[index]
    }
}
impl Index<DedupPoolId<ComponentDefinition>> for TypesContext {
    type Output = ComponentDefinition;
    fn index(&self, index: DedupPoolId<ComponentDefinition>) -> &Self::Output {
        &self.components[index]
    }
}
impl Index<DedupPoolId<FunctionType>> for TypesContext {
    type Output = FunctionType;
    fn index(&self, index: DedupPoolId<FunctionType>) -> &Self::Output {
        &self.functions[index]
    }
}
impl Index<DedupPoolId<StyleType>> for TypesContext {
    type Output = StyleType;
    fn index(&self, index: DedupPoolId<StyleType>) -> &Self::Output {
        &self.styles[index]
    }
}
