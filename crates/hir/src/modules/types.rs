use common::SymbolPointer;

use crate::{TypeId, VariableId, model::HirType};
use std::collections::HashMap;

const INT_IDX: usize = 0;
const FLOAT_IDX: usize = 1;
const STR_IDX: usize = 2;
const VOID_IDX: usize = 3;
const INFER_IDX: usize = 4;
const GENERIC_COMPONENT_IDX: usize = 5;
const BOOL_IDX: usize = 6;
const BUILTIN_TYPES_SIZE: usize = 7;

/// The set of built-in primitive types pre-registered in every [`TypesModule`].
pub const BUILTIN_TYPES: [HirType; BUILTIN_TYPES_SIZE] = [
    HirType::Int,
    HirType::Float,
    HirType::Str,
    HirType::Void,
    HirType::Infer,
    HirType::GenericComponent,
    HirType::Bool,
];
/// The source-level names corresponding to each entry in [`BUILTIN_TYPES`].
pub const BUILTIN_NAMES: [&str; BUILTIN_TYPES_SIZE] = [
    "int",
    "float",
    "str",
    "void",
    "infer",
    "GenericComponent",
    "bool",
];

/// Holds the pre-allocated [`TypeId`]s for each built-in primitive type.
#[derive(Debug, Clone)]
pub struct BuiltinTypes {
    int: TypeId,

    float: TypeId,

    str: TypeId,

    void: TypeId,

    ///Won't appear on final IR, just so the type checker knows what must be inferred
    infer: TypeId,

    generic_component: TypeId,

    bool: TypeId,
}

impl Default for BuiltinTypes {
    fn default() -> Self {
        Self::new()
    }
}

impl BuiltinTypes {
    /// Creates a new [`BuiltinTypes`] with IDs matching the fixed indices in [`BUILTIN_TYPES`].
    pub fn new() -> Self {
        Self {
            int: TypeId::from_raw(INT_IDX as u64),
            float: TypeId::from_raw(FLOAT_IDX as u64),
            str: TypeId::from_raw(STR_IDX as u64),
            void: TypeId::from_raw(VOID_IDX as u64),
            infer: TypeId::from_raw(INFER_IDX as u64),
            generic_component: TypeId::from_raw(GENERIC_COMPONENT_IDX as u64),
            bool: TypeId::from_raw(BOOL_IDX as u64),
        }
    }
}

/// Manages all types in the HIR, including built-ins, user-defined types, and variables.
#[derive(Debug, Default)]
pub struct TypesModule {
    ///A hashmap that maps a name of a global name to its type. This is not for variables, but only for global types, such as structs, functions and components
    type_names: HashMap<SymbolPointer, TypeId>,
    ///Maps the type ids to its their name forms
    name_of_types: HashMap<TypeId, SymbolPointer>,
    ///Maps a variable to it's type
    pub variables: HashMap<VariableId, TypeId>,

    types: Vec<HirType>,
    builtins: BuiltinTypes,
}
impl TypesModule {
    /// Creates a new [`TypesModule`] with built-in types pre-registered under the given symbol names.
    pub fn new(builtin_names: &[SymbolPointer; BUILTIN_TYPES_SIZE]) -> Self {
        let mut types = Vec::with_capacity(BUILTIN_TYPES_SIZE);
        let mut type_names = HashMap::new();
        let mut name_of_types = HashMap::new();
        for ty in BUILTIN_TYPES.iter() {
            types.push(ty.clone());
        }
        for (idx, name_symbol) in builtin_names.iter().enumerate() {
            let id = TypeId::from_raw(idx as u64);
            type_names.insert(*name_symbol, id);
            name_of_types.insert(id, *name_symbol);
        }
        Self {
            type_names,
            name_of_types,
            variables: HashMap::new(),
            types,
            builtins: BuiltinTypes::new(),
        }
    }

    /// Creates a new tuple type with the given field types and returns its [`TypeId`].
    pub fn add_tuple_type(&mut self, fields: Vec<TypeId>) -> TypeId {
        self.insert_unnamed_type(HirType::Tuple { fields })
    }
    /// Returns the [`TypeId`] of the built-in `int` type.
    pub fn int_id(&self) -> TypeId {
        self.builtins.int
    }
    /// Returns the [`TypeId`] of the built-in `float` type.
    pub fn float_id(&self) -> TypeId {
        self.builtins.float
    }
    /// Returns the [`TypeId`] of the built-in `str` type.
    pub fn str_id(&self) -> TypeId {
        self.builtins.str
    }
    /// Returns the [`TypeId`] of the built-in `void` type.
    pub fn void_id(&self) -> TypeId {
        self.builtins.void
    }
    /// Returns the [`TypeId`] of the special `infer` type used during type inference.
    pub fn infer_id(&self) -> TypeId {
        self.builtins.infer
    }
    /// Returns the [`TypeId`] of the built-in `GenericComponent` type.
    pub fn generic_component_id(&self) -> TypeId {
        self.builtins.generic_component
    }
    /// Returns the [`TypeId`] of the built-in `bool` type.
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
        self.name_of_types.insert(v, name);
        self.types.push(ty);
        v
    }

    ///Returns the inner object from the provided `ty`, returns None if the type is not a object
    pub fn get_object(&self, ty: &TypeId) -> Option<&HirType> {
        match self.get_type(ty) {
            v @ HirType::Struct { .. } => Some(v),
            HirType::Reference { rf, .. } => self.get_object(rf),
            _ => None,
        }
    }

    ///Returns the inner component from the provided `ty`, returns None if the type is not a object
    pub fn get_component(&self, ty: &TypeId) -> Option<&HirType> {
        match self.get_type(ty) {
            v @ HirType::Component { .. } => Some(v),
            HirType::Reference { rf, .. } => self.get_object(rf),
            _ => None,
        }
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
    /// Returns the [`HirType`] for the given [`TypeId`].
    ///
    /// # Panics
    ///
    /// Panics if `id` does not correspond to a registered type.
    pub fn get_type(&self, id: &TypeId) -> &HirType {
        &self.types[id.as_raw() as usize]
    }
    /// Returns the name symbol associated with the given [`TypeId`], if any.
    pub fn get_type_name(&self, id: &TypeId) -> Option<&SymbolPointer> {
        self.name_of_types.get(id)
    }
    /// Returns the [`TypeId`] of the given variable, if it has been registered.
    pub fn get_variable(&self, id: &VariableId) -> Option<&TypeId> {
        self.variables.get(id)
    }
    /// Returns the [`HirType`] associated with the given name symbol, if it exists.
    pub fn get_type_from_name(&self, name: &SymbolPointer) -> Option<&HirType> {
        self.type_names.get(name).map(|id| self.get_type(id))
    }

    /// Returns a mutable reference to the [`HirType`] for the given [`TypeId`].
    ///
    /// # Panics
    ///
    /// Panics if `id` does not correspond to a registered type.
    pub fn get_type_mut(&mut self, id: &TypeId) -> &mut HirType {
        &mut self.types[id.as_raw() as usize]
    }
    /// Returns a mutable reference to the [`HirType`] associated with the given name symbol, if it exists.
    pub fn get_type_from_name_mut(&mut self, name: &SymbolPointer) -> Option<&mut HirType> {
        self.type_names
            .get(name)
            .map(|id| &mut self.types[id.as_raw() as usize])
    }
    /// Follows a [`HirType::Reference`] and returns the inner type it points to.
    ///
    /// # Panics
    ///
    /// Panics if the type at `id` is not a [`HirType::Reference`].
    pub fn get_type_from_ref(&self, id: &TypeId) -> &HirType {
        if let HirType::Reference { rf, .. } = self.get_type(id) {
            self.get_type(rf)
        } else {
            unreachable!("The provided ref_ty should be of type Reference");
        }
    }
}
