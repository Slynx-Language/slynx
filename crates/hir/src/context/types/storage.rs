use std::{
    collections::{HashSet, VecDeque},
    ops::Index,
};

use common::{
    Span,
    pool::{DedupPool, DedupPoolId},
};

use crate::{
    ComponentType, DescriptorId, EnumType, EnumVariantType, HIRError, Result, StructType,
    SymbolPointer, TupleType,
    helpers::Visible,
    term::{Term, TermId, TermNode},
};

use super::{
    components::{ComponentDefinition, ComponentsPool},
    enums::EnumsPool,
    structs::{StructDefinition, StructsPool},
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
    pub enums: EnumsPool,
    pub terms: DedupPool<Term>,
}
impl Default for TypeStorage {
    fn default() -> Self {
        Self {
            terms: DedupPool::new(),
            structs: StructsPool::default(),
            components: ComponentsPool::default(),
            enums: EnumsPool::default(),
        }
    }
}
impl TypeStorage {
    ///Number of distinct types in the term pool.
    pub fn len(&self) -> usize {
        self.terms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Inserts `ty` into the deduplicated term pool and returns its id.
    ///
    /// `HirType` is an alias for `Term`, so a type id and a term id are one and
    /// the same: both flow through the `terms` pool that every [`HirViewer`]
    /// and `Index` read. The legacy `types` pool is being retired.
    pub fn insert_type(&self, ty: Term) -> TermId {
        self.terms.insert(ty)
    }

    ///Returns the inner object from the provided `ty`, returns None if the type is not a object
    pub fn get_object(&self, ty: TermId) -> Option<TermId> {
        let mut visited = HashSet::new();
        let mut current = ty;
        loop {
            if !visited.insert(current) {
                return None;
            }

            match self[current].node() {
                TermNode::Data(DescriptorId::Struct(_)) => {
                    return Some(current);
                }
                TermNode::Apply { target, .. } => current = *target,
                _ => return None,
            }
        }
    }

    ///Returns the inner component from the provided `ty`, returns None if the type is not a object
    pub fn get_component(&self, ty: &TermId) -> Option<TermId> {
        let mut visited = HashSet::new();
        let mut current = *ty;
        loop {
            if !visited.insert(current) {
                return None;
            }

            match self[current].node() {
                TermNode::Data(DescriptorId::Component(_)) => {
                    return Some(current);
                }
                TermNode::Apply { target, .. } => current = *target,
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

    pub fn get_struct_name(&self, s: DedupPoolId<StructType>) -> SymbolPointer {
        let metadata = self.structs[s].metadata;
        self.structs[metadata].name
    }
    pub fn get_struct_fields(&self, s: DedupPoolId<StructType>) -> &[Visible<SymbolPointer>] {
        let metadata = self.structs[s].metadata;
        &self.structs[metadata].fields
    }

    pub fn get_struct_field_types(&self, s: DedupPoolId<StructType>) -> &[TermId] {
        &self.structs[s].fields
    }

    pub fn get_struct_signature(
        &self,
        s: DedupPoolId<StructType>,
    ) -> Vec<(&Visible<SymbolPointer>, &TermId)> {
        self.get_struct_fields(s)
            .iter()
            .zip(&self.structs[s].fields)
            .collect()
    }

    ///Retrieves the type of something by asserting the provided `ref_ty` is a reference type to it
    pub fn get_type_from_ref(&self, ref_ty: TermId, span: &Span) -> Result<TermId> {
        let mut visited = HashSet::new();
        let mut current = ref_ty;
        loop {
            match self[current].node() {
                TermNode::Apply { target, .. } => {
                    if !visited.insert(current) {
                        return Err(HIRError::recursive(current, *span));
                    }
                    current = *target;
                }
                _ => return Ok(current),
            }
        }
    }

    pub fn is_cyclic(&self, ty: TermId) -> bool {
        let mut set = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(ty);
        while let Some(ty) = queue.pop_front() {
            if !set.insert(ty) {
                return true;
            }

            match self[ty].node() {
                TermNode::Apply { target, .. } => {
                    queue.push_back(*target);
                }
                TermNode::Tuple { fields } => {
                    for field in fields {
                        queue.push_back(*field);
                    }
                }
                TermNode::Data(DescriptorId::Struct(descriptor)) => {
                    for field in &self[*descriptor].fields {
                        queue.push_back(*field);
                    }
                }
                TermNode::Data(DescriptorId::Enum(descriptor)) => {
                    for variant in &self[*descriptor].variants {
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
    Term => |this, idx| this.terms.get(idx),
    StructType => |this, idx| &this.structs[idx],
    StructDefinition => |this, idx| &this.structs[idx],
    TupleType => |this, idx| &this.structs[idx],
    EnumType => |this, idx| &this.enums[idx],
    ComponentType => |this, idx| &this.components[idx],
    ComponentDefinition => |this, idx| &this.components[idx],
);
