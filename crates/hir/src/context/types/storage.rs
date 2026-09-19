use std::{
    collections::{HashSet, VecDeque},
    ops::Index,
};

use common::{
    Span,
    pool::{DedupPool, DedupPoolId},
};

use crate::{
    ComponentType, EnumType, EnumVariantType, FunctionType, HIRError, HirType, Result, StructType,
    StyleType, SymbolPointer, TupleType, helpers::Visible,
};

use super::{
    components::{ComponentDefinition, ComponentsPool},
    enums::EnumsPool,
    structs::{StructDefinition, StructsPool},
    styles::StylesPool,
};

#[derive(Debug)]
/// Owns the deduplicated pools that store every HIR type shape — structs,
/// components, styles, enums, functions, tuples and the raw `HirType` tags.
///
/// This is pure storage: it has no concept of names ([`super::registry::TypeRegistry`]
/// maps names to type ids) and no notion of methods ([`super::methods::MethodTable`]).
pub struct TypeStorage {
    pub structs: StructsPool,
    pub components: ComponentsPool,
    pub styles: StylesPool,
    pub enums: EnumsPool,
    pub functions: DedupPool<FunctionType>,
    pub types: DedupPool<HirType>,
}
impl Default for TypeStorage {
    fn default() -> Self {
        Self {
            structs: StructsPool::default(),
            components: ComponentsPool::default(),
            styles: StylesPool::default(),
            enums: EnumsPool::default(),
            functions: DedupPool::new(),
            types: DedupPool::new(),
        }
    }
}
impl TypeStorage {
    /// Number of distinct types in the type pool.
    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Inserts `ty` into the deduplicated type pool and returns its id.
    pub fn insert_type(&self, ty: HirType) -> DedupPoolId<HirType> {
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

    ///Returns the name of the enum associated with the given `id`.
    pub fn get_enum_name(&self, id: DedupPoolId<EnumType>) -> SymbolPointer {
        self.enums[id].name
    }

    ///Returns the variants of the enum associated with the given `id`, in
    ///declaration order.
    pub fn get_enum_variants(&self, id: DedupPoolId<EnumType>) -> &[EnumVariantType] {
        &self.enums[id].variants
    }

    ///Finds the variant with the given `name` on the enum associated with `id`.
    pub fn find_enum_variant(
        &self,
        id: DedupPoolId<EnumType>,
        name: SymbolPointer,
    ) -> Option<usize> {
        self.get_enum_variants(id)
            .iter()
            .position(|variant| variant.name == name)
    }

    pub fn get_component_definition(
        &self,
        comp: DedupPoolId<ComponentType>,
    ) -> &ComponentDefinition {
        let meta = self.components[comp].metadata;
        &self.components[meta]
    }

    pub fn get_style_name(&self, s: DedupPoolId<StyleType>) -> SymbolPointer {
        let metadata = self.styles[s].metadata;
        self.styles.index(metadata).name
    }
    pub fn get_struct_name(&self, s: DedupPoolId<StructType>) -> SymbolPointer {
        let metadata = self.structs[s].metadata;
        self.structs[metadata].name
    }
    pub fn get_struct_fields(&self, s: DedupPoolId<StructType>) -> &[Visible<SymbolPointer>] {
        let metadata = self.structs[s].metadata;
        &self.structs[metadata].fields
    }

    pub fn get_struct_field_types(&self, s: DedupPoolId<StructType>) -> &[DedupPoolId<HirType>] {
        &self.structs[s].fields
    }

    pub fn get_struct_signature(
        &self,
        s: DedupPoolId<StructType>,
    ) -> Vec<(&Visible<SymbolPointer>, &DedupPoolId<HirType>)> {
        self.get_struct_fields(s)
            .iter()
            .zip(&self.structs[s].fields)
            .collect()
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
                HirType::Enum(id) => {
                    for variant in &self[id].variants {
                        for field in &variant.payload {
                            queue.push_back(*field);
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }
}

macro_rules! impl_index {
    ($($ty:ident => |$this:ident, $idx:ident| $body:expr),* $(,)?) => {
        $(
            impl Index<DedupPoolId<$ty>> for TypeStorage {
                type Output = $ty;

                fn index(&self, index: DedupPoolId<$ty>) -> &Self::Output {
                    let $this = self;
                    let $idx = index;
                    $body
                }
            }
        )*
    };
}

impl_index!(
    HirType => |this, idx| this.types.get(idx),
    StructType => |this, idx| &this.structs[idx],
    StructDefinition => |this, idx| &this.structs[idx],
    TupleType => |this, idx| &this.structs[idx],
    EnumType => |this, idx| &this.enums[idx],
    ComponentType => |this, idx| &this.components[idx],
    ComponentDefinition => |this, idx| &this.components[idx],
    FunctionType => |this, idx| &this.functions[idx],
    StyleType => |this, idx| &this.styles[idx]
);