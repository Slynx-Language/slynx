mod tys;
pub use tys::*;

use crate::hir::{TypeId, VariableId, symbols::SymbolPointer};
use std::collections::HashMap;

const INT_IDX: usize = 0;
const FLOAT_IDX: usize = 1;
const STR_IDX: usize = 2;
const VOID_IDX: usize = 3;
const INFER_IDX: usize = 4;
const GENERIC_COMPONENT_IDX: usize = 5;
const BOOL_IDX: usize = 6;

const BUILTIN_TYPES: [HirType; 7] = [
    HirType::Int,
    HirType::Float,
    HirType::Str,
    HirType::Void,
    HirType::Infer,
    HirType::GenericComponent,
    HirType::Bool,
];

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
        // Keep builtin ids deterministic and fail fast if array order drifts.
        debug_assert_eq!(BOOL_IDX + 1, BUILTIN_TYPES.len());
        debug_assert!(matches!(BUILTIN_TYPES[INT_IDX], HirType::Int));
        debug_assert!(matches!(BUILTIN_TYPES[FLOAT_IDX], HirType::Float));
        debug_assert!(matches!(BUILTIN_TYPES[STR_IDX], HirType::Str));
        debug_assert!(matches!(BUILTIN_TYPES[VOID_IDX], HirType::Void));
        debug_assert!(matches!(BUILTIN_TYPES[INFER_IDX], HirType::Infer));
        debug_assert!(matches!(
            BUILTIN_TYPES[GENERIC_COMPONENT_IDX],
            HirType::GenericComponent
        ));
        debug_assert!(matches!(BUILTIN_TYPES[BOOL_IDX], HirType::Bool));

        let builtins = BuiltinTypes::new();
        Self {
            types: BUILTIN_TYPES.to_vec(),
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
        if let HirType::Reference { rf, .. } = self.get_type(id) {
            self.get_type(rf)
        } else {
            unreachable!("The provided ref_ty should be of type Reference");
        }
    }
}
