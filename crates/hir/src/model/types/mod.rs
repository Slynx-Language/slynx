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
//! - [`ComponentProperty`] — Property of a component type
//! - [`crate::hir::TypeId`] — Type identifiers
//! - [`crate::hir::modules::TypesModule`] — Type management
pub mod arrays;
pub mod generic_component;
pub mod term;
#[cfg(test)]
mod term_tests;
pub mod vector;
use crate::{
    SymbolPointer,
    context::{ComponentDefinition, StructDefinition},
    term::{Term, TermId},
};

use common::{VisibilityModifier, pool::DedupPoolId};
use module_loader::ASTBuiltin;
use smallvec::SmallVec;

/// A property of a component type.
///
/// Component properties define the interface of a component, including
/// the property name, its type, and visibility.
///
/// # Fields
///
/// - `0` — The visibility modifier (`pub` or private)
/// - `1` — The property name
/// - `2` — The property's type ID
///
/// # Example
///
/// ```slynx
/// component Button(props: ButtonProps) {
///     pub label: str = "Click me"  // ComponentProperty(Public, "label", str)
///     private count: int = 0       // ComponentProperty(Private, "count", int)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ComponentProperty(VisibilityModifier, SymbolPointer, DedupPoolId<HirType>);

impl ComponentProperty {
    /// Creates a new component property.
    ///
    /// # Arguments
    ///
    /// * `visibility` — The property's visibility (`pub` or private)
    /// * `name` — The property's name
    /// * `ty` — The property's type ID
    ///
    /// # Returns
    ///
    /// A new [`ComponentProperty`] instance.
    pub fn new(
        visibility: VisibilityModifier,
        name: SymbolPointer,
        ty: DedupPoolId<HirType>,
    ) -> Self {
        Self(visibility, name, ty)
    }

    /// Creates a new public component property.
    ///
    /// # Arguments
    ///
    /// * `name` — The property's name
    /// * `ty` — The property's type ID
    ///
    /// # Returns
    ///
    /// A new [`ComponentProperty`] with public visibility.
    pub fn new_public(name: SymbolPointer, ty: DedupPoolId<HirType>) -> Self {
        Self::new(VisibilityModifier::Public, name, ty)
    }

    /// Creates a new private component property.
    ///
    /// # Arguments
    ///
    /// * `name` — The property's name
    /// * `ty` — The property's type ID
    ///
    /// # Returns
    ///
    /// A new [`ComponentProperty`] with private visibility.
    pub fn new_private(name: SymbolPointer, ty: DedupPoolId<HirType>) -> Self {
        Self::new(VisibilityModifier::Private, name, ty)
    }

    /// Returns the property's visibility modifier.
    pub fn visibility(&self) -> &VisibilityModifier {
        &self.0
    }

    /// Returns the property's name.
    pub fn name(&self) -> SymbolPointer {
        self.1
    }

    /// Returns the property's type ID.
    pub fn prop_type(&self) -> DedupPoolId<HirType> {
        self.2
    }
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct TupleType {
    pub(crate) fields: Vec<DedupPoolId<HirType>>,
}
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct StructType {
    pub(crate) fields: Vec<DedupPoolId<HirType>>,
    pub(crate) metadata: DedupPoolId<StructDefinition>,
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

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct ComponentType {
    pub(crate) properties: Vec<DedupPoolId<HirType>>,
    pub(crate) children: Vec<DedupPoolId<ComponentType>>,
    pub(crate) metadata: DedupPoolId<ComponentDefinition>,
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct FunctionType {
    pub(crate) args: SmallVec<[DedupPoolId<HirType>; 2]>,
    pub(crate) ret: DedupPoolId<HirType>,
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub enum DescriptorId {
    Struct(DedupPoolId<StructType>),
    Enum(DedupPoolId<EnumType>),
    Component(DedupPoolId<ComponentType>),
}

pub type HirType = Term;
