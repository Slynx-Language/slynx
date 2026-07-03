//! Declaration Definitions
//!
//! This module defines the [`HirDeclaration`] structure and related types that
//! represent top-level declarations in the HIR.
//!
//! # Overview
//!
//! Declarations are the building blocks of a Slynx program. They include:
//! - Functions
//! - Components
//! - Objects (data structures)
//! - Type aliases
//!
//! Each declaration has:
//! - A unique [`DeclarationId`] for identification
//! - A [`TypeId`] representing its type
//! - A [`HirDeclarationKind`] describing what kind of declaration it is
//! - A source [`Span`] for error reporting
//!
//! # Key Types
//!
//! - [`HirDeclaration`] — The main declaration structure
//! - [`HirDeclarationKind`] — Enum of declaration kinds
//! - [`ComponentMemberDeclaration`] — Members of a component declaration

use common::{
    Span, Spanned, VisibilityModifier,
    pool::{DedupPoolId, PoolId},
};
use smallvec::SmallVec;

use crate::{
    DeclarationId, HirType, SymbolPointer, VariableId,
    model::{
        HirComponentExpression, HirExpression, HirStatement, HirStyleStatement, PropertyExpression,
    },
};

#[derive(Debug)]
///A style usage. This contains an ID to another stylesheet, and the parameters used to generate before the actual style.
pub struct HirStyleUsage {
    /// The id of the style to use
    pub style: DeclarationId<HirStylesheetDeclaration>,
    ///The parameters to it
    pub params: Vec<Spanned<PoolId<HirExpression>>>,
}
#[derive(Debug)]
pub struct HirFunctionDeclaration {
    pub name: SymbolPointer,
    pub args: SmallVec<[VariableId; 2]>,
    pub statements: Vec<Spanned<PoolId<HirStatement>>>,
    pub ty: DedupPoolId<HirType>,
    pub visibility: VisibilityModifier,
    pub external: bool,
}

#[derive(Debug)]
pub struct HirObjectDeclaration {
    pub name: SymbolPointer,
    pub ty: DedupPoolId<HirType>,
    pub visibility: VisibilityModifier,
    pub external: bool,
}

#[derive(Debug)]
pub struct HirStaticDeclaration {
    pub name: SymbolPointer,
    pub ty: DedupPoolId<HirType>,
    pub visibility: VisibilityModifier,
    pub external: bool,
}

#[derive(Debug)]
pub struct HirAliasDeclaration {
    pub name: SymbolPointer,
    pub ty: DedupPoolId<HirType>,
    pub visibility: VisibilityModifier,
}

#[derive(Debug)]
pub struct HirComponentDeclaration {
    pub name: SymbolPointer,
    pub props: Vec<ComponentMemberDeclaration>,
    pub ty: DedupPoolId<HirType>,
    pub visibility: VisibilityModifier,
}

#[derive(Debug)]
pub struct HirStylesheetDeclaration {
    pub usages: Vec<HirStyleUsage>,
    pub name: SymbolPointer,
    pub args: SmallVec<[VariableId; 2]>,
    pub statements: Vec<HirStyleStatement>,
    pub ty: DedupPoolId<HirType>,
    pub visibility: VisibilityModifier,
    pub external: bool,
}

/// A member of a component declaration.
///
/// Component members can be either properties (with optional default values)
/// or child components.
///
/// # Variants
///
/// ## `Property`
///
/// A named property of the component with an optional default value.
///
/// ```slynx
/// prop label: str = "Hello"
/// ```
///
/// ## `Child`
///
/// A child component that can contain other members.
///
/// ```slynx
/// child Container {
///     prop items: list<str>
/// }
/// ```
///
/// ## `Specialized`
///
/// A specialized component like `Text` or `Div` with predefined behavior.
#[derive(Debug)]
#[repr(C)]
pub enum ComponentMemberDeclaration {
    /// A property declaration with an optional default value.
    ///
    /// # Fields
    ///
    /// - `id` — A unique ID for this property
    /// - `index` — The property's position in the component's property list
    /// - `value` — The optional default value expression
    /// - `span` — Source location for error reporting
    Property {
        /// The index of this property in the component's property list.
        ///
        /// Used for efficient property access at runtime.
        index: usize,

        /// The property's default value, if any.
        ///
        /// If `None`, the property must be provided when the component is used.
        value: Option<HirExpression>,

        /// The source location of this property declaration.
        span: Span,
    },

    /// A child component declaration.
    ///
    /// # Fields
    ///
    /// - `name` — The child component's type
    /// - `values` — The child's property values
    /// - `span` — Source location for error reporting
    Child(HirComponentExpression),
}

impl ComponentMemberDeclaration {
    /// Creates a new property declaration.
    ///
    /// # Arguments
    ///
    /// * `index` — The property's position in the component's property list
    /// * `value` — The optional default value expression
    /// * `span` — The source location of the property
    ///
    /// # Returns
    ///
    /// A new [`ComponentMemberDeclaration::Property`] instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use slynx_frontend::hir::model::*;
    /// # use common::Span;
    /// # let span = Span::default();
    /// # let value = None;
    /// let prop = ComponentMemberDeclaration::new_property(
    ///     0,      // index
    ///     value,  // default value
    ///     span,
    /// );
    /// ```
    pub fn new_property(index: usize, value: Option<HirExpression>, span: Span) -> Self {
        Self::Property { index, value, span }
    }

    /// Creates a new child component declaration.
    ///
    /// # Arguments
    ///
    /// * `name` — The child component's type ID
    /// * `values` — The child's property values
    /// * `span` — The source location of the child declaration
    ///
    /// # Returns
    ///
    /// A new [`ComponentMemberDeclaration::Child`] instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use slynx_frontend::hir::model::*;
    /// # use common::Span;
    /// # let span = Span::default();
    /// # let name = TypeId::from_raw(0);
    /// # let values = vec![];
    /// let child = ComponentMemberDeclaration::new_child(name, values, span);
    /// ```
    pub fn new_child(
        name: DedupPoolId<HirType>,
        properties: Vec<PropertyExpression>,
        children: Vec<Spanned<PoolId<HirComponentExpression>>>,
    ) -> Self {
        Self::Child(HirComponentExpression {
            name,
            properties,
            children,
        })
    }
}
