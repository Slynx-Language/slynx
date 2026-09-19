mod components;
mod enums;
mod methods;
mod registry;
mod storage;
mod structs;
mod styles;
use std::ops::Index;

use common::pool::DedupPoolId;
use dashmap::{DashMap, DashSet};

use crate::{
    ComponentType, DeclarationId, EnumType, EnumVariantType, FunctionType, HirFunctionDeclaration,
    HirType, Result, StructType, StyleType, SymbolPointer, TupleType, VariableId, helpers::Visible,
};

pub use components::ComponentDefinition;
pub use methods::MethodTable;
pub use registry::TypeRegistry;
pub use storage::TypeStorage;
pub use structs::StructDefinition;
pub use styles::StyleMetadata;

#[derive(Debug)]
/// Manages all types in the HIR, including built-ins, user-defined types, and variables.
///
/// This is a thin facade over three single-purpose collaborators:
/// - [`TypeStorage`] — the deduplicated pools holding type shapes,
/// - [`TypeRegistry`] — the name→type-id mapping,
/// - [`MethodTable`] — methods attached to any type.
///
/// It additionally tracks variables and externally-marked types.
pub struct TypesContext {
    ///Maps a variable to it's type
    pub variables: DashMap<VariableId, DedupPoolId<HirType>>,
    /// Deduplicated pools holding every type shape.
    pub storage: TypeStorage,
    /// Maps names to the type ids they refer to.
    pub registry: TypeRegistry,
    /// Methods attached to types. Methods may be registered on any type id,
    /// so any value can carry methods.
    pub methods: MethodTable,
    /// Set of DedupPoolId<HirType>s that are external (from JS/interop).
    /// When a type is marked external, all references to it are also external.
    externals: DashSet<DedupPoolId<HirType>>,
}
impl Default for TypesContext {
    fn default() -> Self {
        Self::new()
    }
}
impl TypesContext {
    /// Creates a new [`TypesContext`] with built-in types pre-registered under the given symbol names.
    pub fn new() -> Self {
        Self {
            variables: DashMap::new(),
            storage: TypeStorage::default(),
            registry: TypeRegistry::new(),
            methods: MethodTable::new(),
            externals: DashSet::new(),
        }
    }

    /// Number of distinct types in the type pool.
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    ///Inserts a new variable on this Context
    pub fn insert_variable(&self, varid: VariableId, ty: DedupPoolId<HirType>) {
        self.variables.insert(varid, ty);
    }

    /// Returns the [`DedupPoolId<HirType>`] of the given variable, if it has been registered.
    pub fn get_variable(&self, id: &VariableId) -> Option<DedupPoolId<HirType>> {
        self.variables.get(id).map(|v| *v.value())
    }

    pub fn create_function_type(
        &self,
        args: Vec<DedupPoolId<HirType>>,
        ret: DedupPoolId<HirType>,
    ) -> DedupPoolId<HirType> {
        let fid = self.storage.functions.insert(FunctionType {
            args: args.into(),
            ret,
        });
        self.storage.insert_type(HirType::Function(fid))
    }

    /// Creates a new tuple type with the given field types and returns its [`TypeId`].
    pub fn create_tuple_type(&self, fields: Vec<DedupPoolId<HirType>>) -> DedupPoolId<HirType> {
        let tuple = self.storage.structs.insert_at_tuples(TupleType { fields });
        self.storage.insert_type(HirType::Tuple(tuple))
    }

    pub fn create_struct_type(
        &self,
        name: SymbolPointer,
        fields: Vec<Visible<(SymbolPointer, DedupPoolId<HirType>)>>,
        methods: Vec<Visible<(SymbolPointer, DeclarationId<HirFunctionDeclaration>)>>,
    ) -> DedupPoolId<HirType> {
        let (id, _) = self.storage.structs.insert(name, fields, methods);
        let id = self.storage.insert_type(HirType::Struct(id));
        self.registry.register(name, id);
        id
    }

    ///Creates a new enum type with the given `name` and `variants`, registering
    ///its name on the type namespace, and returns its type id.
    ///
    ///Idempotent by name: if an enum with this `name` already exists, its
    ///existing type id is returned and the `names` namespace is left untouched.
    ///This prevents specialization/substitution from registering a duplicate
    ///enum under the same name and silently re-pointing the namespace at a
    ///sibling type while older references still hold the original id.
    pub fn create_enum_type(
        &self,
        name: SymbolPointer,
        variants: Vec<EnumVariantType>,
    ) -> DedupPoolId<HirType> {
        if let Some(existing) = self.storage.enums.find_by_name(name) {
            return self.storage.insert_type(HirType::Enum(existing));
        }
        let id = self.storage.enums.insert(name, variants);
        let id = self.storage.insert_type(HirType::Enum(id));
        self.registry.register(name, id);
        id
    }

    ///Returns the name of the enum associated with the given `id`.
    pub fn get_enum_name(&self, id: DedupPoolId<EnumType>) -> SymbolPointer {
        self.storage.get_enum_name(id)
    }

    ///Returns the variants of the enum associated with the given `id`, in
    ///declaration order.
    pub fn get_enum_variants(&self, id: DedupPoolId<EnumType>) -> &[EnumVariantType] {
        self.storage.get_enum_variants(id)
    }

    ///Finds the variant with the given `name` on the enum associated with `id`.
    pub fn find_enum_variant(
        &self,
        id: DedupPoolId<EnumType>,
        name: SymbolPointer,
    ) -> Option<usize> {
        self.storage.find_enum_variant(id, name)
    }

    pub fn create_component_type(
        &self,
        name: SymbolPointer,
        properties: Vec<(SymbolPointer, DedupPoolId<HirType>)>,
        children: Vec<DedupPoolId<ComponentType>>,
    ) -> DedupPoolId<HirType> {
        let (comp_ty, _) = self.storage.components.insert(name, properties, children);
        let id = self.storage.insert_type(HirType::Component(comp_ty));
        self.registry.register(name, id);
        id
    }

    pub fn create_style_type(
        &self,
        name: SymbolPointer,
        args: Vec<DedupPoolId<HirType>>,
    ) -> DedupPoolId<HirType> {
        let metadata = self
            .storage
            .styles
            .insert_at_metadata(StyleMetadata { name });
        let style_id = self.storage.styles.insert_at_styles(StyleType {
            args: args.into(),
            metadata,
        });
        let id = self.storage.insert_type(HirType::Style(style_id));
        self.registry.register(name, id);
        id
    }

    pub fn get_component_definition(
        &self,
        comp: DedupPoolId<ComponentType>,
    ) -> &ComponentDefinition {
        self.storage.get_component_definition(comp)
    }

    pub fn create_alias_type(&self, name: SymbolPointer, ty: HirType) -> DedupPoolId<HirType> {
        let id = self.storage.insert_type(ty);
        self.registry.register(name, id);
        id
    }

    ///Inserts the provided `ty` to have the provided `name`
    pub fn create_type(&self, ty: HirType) -> DedupPoolId<HirType> {
        self.storage.insert_type(ty)
    }

    ///Returns the inner object from the provided `ty`, returns None if the type is not a object
    pub fn get_object(&self, ty: DedupPoolId<HirType>) -> Option<DedupPoolId<HirType>> {
        self.storage.get_object(ty)
    }

    ///Returns the inner component from the provided `ty`, returns None if the type is not a object
    pub fn get_component(&self, ty: &DedupPoolId<HirType>) -> Option<DedupPoolId<HirType>> {
        self.storage.get_component(ty)
    }

    ///Registers a method for the given `ty` on the current declaration context with the given `name` that points to the given `id`. It should be asserted by the HIR to be a function ID
    pub fn create_method(
        &self,
        ty: DedupPoolId<HirType>,
        name: SymbolPointer,
        id: DeclarationId<HirFunctionDeclaration>,
    ) {
        self.methods.create_method(ty, name, id);
    }

    /// Register an external method's return type without creating a declaration entry.
    pub fn register_external_method(
        &self,
        parent_ty: DedupPoolId<HirType>,
        name: SymbolPointer,
        return_type: DedupPoolId<HirType>,
    ) {
        self.methods
            .register_external_method(parent_ty, name, return_type);
    }

    /// Returns the return type of an external method on `parent_ty` with the given `name`.
    pub fn get_method_return_type(
        &self,
        parent_ty: &DedupPoolId<HirType>,
        name: SymbolPointer,
    ) -> Option<DedupPoolId<HirType>> {
        self.methods.get_method_return_type(parent_ty, name)
    }

    ///Registers a method for the given `ty` on the current declaration context with the given `name` that points to the given `id`. It should be asserted by the HIR to be a function ID
    pub fn get_methods_of(
        &self,
        ty: DedupPoolId<HirType>,
    ) -> Vec<(SymbolPointer, DeclarationId<HirFunctionDeclaration>)> {
        self.methods.get_methods_of(ty)
    }

    pub fn get_style_name(&self, s: DedupPoolId<StyleType>) -> SymbolPointer {
        self.storage.get_style_name(s)
    }

    ///Retrieves the DedupPoolId<HirType> of the provided `name` on the currentContext
    pub fn get_id_of_name(&self, name: &SymbolPointer) -> Option<DedupPoolId<HirType>> {
        self.registry.get_id_of_name(name)
    }
    pub fn get_struct_name(&self, s: DedupPoolId<StructType>) -> SymbolPointer {
        self.storage.get_struct_name(s)
    }
    pub fn get_struct_fields(&self, s: DedupPoolId<StructType>) -> &[Visible<SymbolPointer>] {
        self.storage.get_struct_fields(s)
    }

    pub fn get_struct_field_types(&self, s: DedupPoolId<StructType>) -> &[DedupPoolId<HirType>] {
        self.storage.get_struct_field_types(s)
    }

    pub fn get_struct_signature(
        &self,
        s: DedupPoolId<StructType>,
    ) -> Vec<(&Visible<SymbolPointer>, &DedupPoolId<HirType>)> {
        self.storage.get_struct_signature(s)
    }

    ///Retrieves the type of something by asserting the provided `ref_ty` is a reference type to it
    pub fn get_type_from_ref(
        &self,
        ref_ty: DedupPoolId<HirType>,
        span: &common::Span,
    ) -> Result<DedupPoolId<HirType>> {
        self.storage.get_type_from_ref(ref_ty, span)
    }

    pub fn is_cyclic(&self, ty: DedupPoolId<HirType>) -> bool {
        self.storage.is_cyclic(ty)
    }

    /// Mark a type as external. Also traverses `Reference` wrappers to mark
    /// the inner struct type, so that all layers of indirection are covered.
    pub fn mark_external(&self, ty: DedupPoolId<HirType>) {
        self.externals.insert(ty);
        let mut current = ty;
        while let HirType::Reference { rf, .. } = self[current] {
            self.externals.insert(rf);
            current = rf;
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

macro_rules! impl_index {
    ($($ty:ident),* $(,)?) => {
        $(
            impl Index<DedupPoolId<$ty>> for TypesContext {
                type Output = $ty;

                fn index(&self, index: DedupPoolId<$ty>) -> &Self::Output {
                    self.storage.index(index)
                }
            }
        )*
    };
}

impl_index!(
    HirType,
    StructType,
    StructDefinition,
    TupleType,
    EnumType,
    ComponentType,
    ComponentDefinition,
    FunctionType,
    StyleType
);
