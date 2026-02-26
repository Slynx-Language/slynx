mod tys;
pub use tys::*;

use crate::hir::{TypeId, VariableId, symbols::SymbolPointer};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BuiltinTypes {
    int: TypeId,
    int_ty: HirType,

    float: TypeId,
    float_ty: HirType,

    str: TypeId,
    str_ty: HirType,

    void: TypeId,
    void_ty: HirType,

    ///Won't appear on final IR, just so the type checker knows what must be inferred
    infer: TypeId,
    infer_ty: HirType,

    generic_component: TypeId,
    generic_component_ty: HirType,

    bool: TypeId,
    bool_ty: HirType,
}

impl Default for BuiltinTypes {
    fn default() -> Self {
        Self::new()
    }
}

impl BuiltinTypes {
    pub fn new() -> Self {
        Self {
            int: TypeId::new(),
            int_ty: HirType::Int,
            float: TypeId::new(),
            float_ty: HirType::Float,
            str: TypeId::new(),
            str_ty: HirType::Str,
            void: TypeId::new(),
            void_ty: HirType::Void,
            infer: TypeId::new(),
            infer_ty: HirType::Infer,
            generic_component: TypeId::new(),
            generic_component_ty: HirType::GenericComponent,
            bool: TypeId::new(),
            bool_ty: HirType::Bool,
        }
    }
}

#[derive(Debug, Default)]
pub struct TypesModule {
    ///A hashmap that maps a name of a global name to its type. This is not for variables, but only for global types, such as structs, functions and components
    type_names: HashMap<SymbolPointer, TypeId>,
    ///Maps a variable to it's type
    pub variables: HashMap<VariableId, TypeId>,

    types: Vec<HirType>,
    builtins: BuiltinTypes,
}
impl TypesModule {
    pub fn new() -> Self {
        let builtins = BuiltinTypes::new();
        Self {
            types: vec![builtins.int_ty.clone(), builtins.float_ty.clone(), builtins.str_ty.clone(), builtins.void_ty.clone(), builtins.infer_ty.clone(), builtins.generic_component_ty.clone(), builtins.bool_ty.clone()],
            builtins,
            type_names: HashMap::new(),
            variables: HashMap::new(),
        }
    }

    pub fn int_id(&self) -> TypeId {
        self.builtins.int
    }
    pub fn float_id(&self) -> TypeId {
        self.builtins.float
    }
    pub fn str_id(&self) -> TypeId {
        self.builtins.str
    }
    pub fn void_id(&self) -> TypeId {
        self.builtins.void
    }
    pub fn infer_id(&self) -> TypeId {
        self.builtins.infer
    }
    pub fn generic_component_id(&self) -> TypeId {
        self.builtins.generic_component
    }
    pub fn bool_id(&self) -> TypeId {
        self.builtins.bool
    }
    ///Inserts a new variable on this module
    pub fn insert_variable(&mut self, varid: VariableId, ty: TypeId) {
        self.variables.insert(varid, ty);
    }

    ///Inserts the provided `ty` to have the provided `name`
    pub fn insert_type(&mut self, name: SymbolPointer, ty: HirType) -> TypeId {
        let raw = self.types.len() as u64;
        let v = TypeId::from_raw(raw);
        self.type_names.insert(name, v);
        self.types.push(ty);
        v
    }

    ///Simply inserts the provided `ty` inside this module. Doesn't map it to anything
    pub fn insert_unnamed_type(&mut self, ty: HirType) -> TypeId {
        let id = TypeId::from_raw(self.types.len() as u64);
        self.types.push(ty);
        id
    }

    ///Retrieves the TypeId of the provided `name` on the current module
    pub fn get_id(&self, name: &SymbolPointer) -> Option<&TypeId> {
        self.type_names.get(name)
    }
    pub fn get_type(&self, id: &TypeId) -> &HirType {
        &self.types[id.as_raw() as usize]
    }
    pub fn get_variable(&self, id: &VariableId) -> Option<&TypeId> {
        self.variables.get(id)
    }
    pub fn get_type_from_name(&self, name: &SymbolPointer) -> Option<&HirType> {
        self.type_names.get(name).map(|id| self.get_type(id))
    }

    pub fn get_type_mut(&mut self, id: &TypeId) -> &mut HirType {
        &mut self.types[id.as_raw() as usize]
    }
    pub fn get_type_from_name_mut(&mut self, name: &SymbolPointer) -> Option<&mut HirType> {
        self.type_names
            .get(name)
            .map(|id| &mut self.types[id.as_raw() as usize])
    }
    pub fn get_type_from_ref(&self, id: &TypeId) -> &HirType {
        if let HirType::Reference { rf, .. } = self.get_type(&id) {
            self.get_type(rf)
        } else {
            unreachable!("The provided ref_ty should be of type Reference");
        }
    }
}
