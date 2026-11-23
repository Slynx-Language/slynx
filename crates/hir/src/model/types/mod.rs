//! Type System
//!
//! This module defines the type system used in the HIR. The [`HirType`] enum
//! represents all possible types in the Slynx language, from primitive types
//! like integers and strings to complex types like functions and components.
//!
//! # Overview
//!
//! The HIR type system includes:
//!
//! - **Primitive types**: `int`, `float`, `str`, `bool`, `void`
//! - **Composite types**: `struct`, `tuple`, `function`, `component`
//! - **Reference types**: References to named types with optional generics
//! - **Special types**: `infer` for type inference, `GenericComponent`
//!
//! # Type Representation
//!
//! Types are represented by the [`HirType`] enum, and type IDs ([`TypeId`])
//! are used throughout the HIR to reference types efficiently.
//!
//! # Examples
//!
//! ```text
//! // Primitive types
//! let int_type = HirType::Int;
//! let bool_type = HirType::Bool;
//!
//! // Struct type
//! let struct_type = HirType::Struct(struct_type_id);
//!
//! // Function type
//! let func_type = HirType::Function(function_type_id);
//!
//! // Reference to a named type with generics
//! let ref_type = HirType::new_generic_ref(option_id, vec![int_type_id]);
//! ```
//!
//! # Related Types
//!
//! - [`HirType`] — The main type enum
//! - [`crate::hir::TypeId`] — Type identifiers
//! - [`crate::hir::modules::TypesModule`] — Type management
pub mod arrays;
pub mod generic_component;
pub mod interface;
pub mod term;
#[cfg(test)]
mod term_tests;
pub mod vector;
use std::hash::Hash;

use crate::{SymbolPointer, Visible, term::TermId};

use common::{VisibilityModifier, pool::DedupPoolId};
use module_loader::FileId;
use smallvec::SmallVec;

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct TupleType {
    pub(crate) fields: Vec<TermId>,
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct StructField {
    pub ty: TermId,
    pub name: SymbolPointer,
}

///The identity of a struct type.
///
///A struct is identified by its name and fields alone; the methods it declares
///are *not* part of that identity. They are not stored here at all — a struct
///lives in a `DedupPool`, which requires `Hash + Eq + Clone` and offers no
///interior mutability, so a lazily grown method list could neither be written
///nor be excluded from the hashed value. Methods are resolved on demand and
///live in the type-keyed `MethodTable` instead, which covers enums and
///components as well.
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct StructType {
    pub(crate) name: SymbolPointer,
    ///The file this struct was declared in. Part of the type's identity for the
    ///same reason as [`ComponentType::owner`]: two same-shaped structs in
    ///different files are not visible in the same places.
    pub(crate) owner: FileId,
    pub(crate) fields: Vec<Visible<StructField>>,
}

/// A single variant of an enum type.
///
/// Raw and raw-valued variants carry no payload; associated and struct
/// variants carry an ordered list of payload types (struct field names are
/// irrelevant to the runtime representation, so they are stored in
/// declaration order). The `discriminant` is the compile-time tag used to
/// distinguish variants at runtime.
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct EnumVariantType {
    /// The name of the variant.
    pub name: SymbolPointer,
    /// The ordered payload types, empty for raw/raw-valued variants.
    pub payload: Vec<TermId>,
    /// The compile-time discriminant (tag) of this variant.
    pub discriminant: i32,
}

/// A user-defined enum type.
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct EnumType {
    /// The name of the enum.
    pub name: SymbolPointer,
    /// The variants of this enum, in declaration order.
    pub variants: Vec<EnumVariantType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentChild {
    pub name: Option<SymbolPointer>,
    ///The type of the child. Its an termid because it can be a generic instantiation for example
    pub ty: TermId,
}
impl Hash for ComponentChild {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ty.hash(state);
    }
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub enum ComponentPropertyKind {
    Property,
    View,
    Input,
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct ComponentProperty {
    pub ty: TermId,
    pub name: SymbolPointer,
    pub property_kind: ComponentPropertyKind,
    pub required: bool,
}

impl ComponentProperty {
    pub fn with_ty(&self, ty: TermId) -> Self {
        Self {
            ty,
            name: self.name,
            property_kind: self.property_kind.clone(),
            required: self.required,
        }
    }
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct ComponentType {
    pub(crate) name: SymbolPointer,
    /// The file this component was declared in.
    ///
    /// Part of the type's identity on purpose: two structurally identical
    /// components declared in different files are *not* the same type, because
    /// they are not visible in the same places. Without this, dedup would merge
    /// them and a private component would leak into the file that reused its
    /// shape.
    pub(crate) owner: FileId,
    pub(crate) properties: Vec<Visible<ComponentProperty>>,
    pub(crate) children: Vec<ComponentChild>,
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct FunctionType {
    pub(crate) args: SmallVec<[TermId; 2]>,
    pub(crate) ret: TermId,
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct InterfaceType {
    pub(crate) name: SymbolPointer,
    pub(crate) super_interfaces: Vec<TermId>,
    ///The signature of the methods
    pub methods: Vec<(SymbolPointer, TermId)>,
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub enum DescriptorId {
    Struct(DedupPoolId<StructType>),
    Enum(DedupPoolId<EnumType>),
    Component(DedupPoolId<ComponentType>),
}
