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
    DeclarationId, HirDeclaration, LanguageItem, Owned, SymbolPointer, TypeDeclaration, VariableId,
    interface::InterfaceTerm,
    model::{HirComponentExpression, HirExpression, HirStatement},
    term::TermId,
};

/// A processed attribute on an HIR declaration.
#[derive(Debug, Clone)]
pub struct HirAttribute {
    pub kind: HirAttributeKind,
    pub span: Span,
}

/// The kind of an attribute.
#[derive(Debug, Clone)]
pub enum HirAttributeKind {
    /// `@builtin("name")` — registers the declaration as a lang item.
    Builtin { name: SymbolPointer },
    /// `@capabilities("fs", "io", "net")` — declares effect-system capabilities.
    Capabilities(Vec<SymbolPointer>),
    /// An unrecognized attribute, stored but not processed further.
    Unknown {
        name: SymbolPointer,
        args: Vec<SymbolPointer>,
    },
}

#[derive(Debug)]
pub struct HirExtendDeclaration {
    ///The type being extended.
    pub target: TermId,
    pub generics: Vec<GenericParameter>,
    ///The interface being implemented.
    pub interfaces: Vec<TermId>,
    ///Methods provided by this implementation, paired with their source names.
    pub methods: Vec<(SymbolPointer, DeclarationId<HirFunctionDeclaration>)>,
    pub attributes: Vec<HirAttribute>,
}

///A generic parameter defined for something. The given `name` is the name of the parameter and bounds are the interfaces that the parameter must implement.
#[derive(Debug, Clone)]
pub struct GenericParameter {
    pub name: SymbolPointer,
    pub bounds: Vec<InterfaceTerm>,
}

#[derive(Debug)]
pub struct HirFunctionDeclaration {
    pub name: SymbolPointer,
    ///The type this declaration is an inherent method of, or `None` when it is
    ///not a method of a single type.
    ///
    ///The HIR does not mangle. Two types declared in the same file may each
    ///declare a method called `run`, so a method is identified by the pair
    ///`(self_type, name)` and never by name alone; the pair is resolved
    ///through the type-keyed [`MethodTable`](crate::context::types::MethodTable),
    ///which is the only table methods are registered in. Free functions keep
    ///their `SymbolRegistry` entry, where a symbol maps to exactly one
    ///declaration.
    ///
    ///`Some` means the name is *not* unique on its own and must be widened with
    ///the owner's name before it reaches the IR — codegen reads this to mangle.
    ///`None` means the name is already final: free functions, and the interface
    ///signature declarations and forwarding thunks, which are synthesized with
    ///an already-unique name.
    pub self_type: Option<TermId>,
    pub generics: Vec<GenericParameter>,
    pub args: SmallVec<[VariableId; 2]>,
    pub statements: Vec<Spanned<PoolId<HirStatement>>>,
    pub ty: TermId,
    pub visibility: VisibilityModifier,
    pub external: bool,
    pub attributes: Vec<HirAttribute>,
    pub span: Span,
}

#[derive(Debug)]
pub struct HirObjectDeclaration {
    pub name: SymbolPointer,
    pub span: Span,
    pub generics: Vec<GenericParameter>,
    pub ty: TermId,
    pub visibility: VisibilityModifier,
    pub external: bool,
    pub attributes: Vec<HirAttribute>,
}

#[derive(Debug)]
pub struct HirStaticDeclaration {
    pub name: SymbolPointer,
    pub span: Span,
    pub ty: Owned<TermId>,
    pub visibility: VisibilityModifier,
    pub external: bool,
    pub attributes: Vec<HirAttribute>,
}

#[derive(Debug)]
pub struct HirAliasDeclaration {
    pub name: SymbolPointer,
    pub attributes: Vec<HirAttribute>,
    pub generics: Vec<GenericParameter>,
    pub ty: TermId,
    pub visibility: VisibilityModifier,
}

#[derive(Debug)]
pub struct HirComponentDeclaration {
    pub name: SymbolPointer,
    pub span: Span,
    pub generics: Vec<GenericParameter>,
    pub props: Vec<ComponentMemberDeclaration>,
    pub ty: TermId,
    pub visibility: VisibilityModifier,
    pub attributes: Vec<HirAttribute>,
}

#[derive(Debug, Clone)]
pub enum HirEnumVariantKind {
    Raw,
    Valued(Spanned<DedupPoolId<HirExpression>>),
    Associated(SmallVec<[TermId; 2]>), //Struct is associated but the names get turned into numeric fields, so it becomes ordered, an enum with variant A {thing: int, b: float} is the same as A(int,float)
}

#[derive(Debug, Clone)]
pub struct HirEnumVariant {
    pub attributes: Vec<HirAttribute>,
    pub name: SymbolPointer,
}

#[derive(Debug)]
pub struct HirEnumDeclaration {
    pub name: SymbolPointer,
    pub span: Span,
    pub generics: Vec<GenericParameter>,
    pub variants: Vec<Spanned<HirEnumVariant>>,
    pub visibility: VisibilityModifier,
    pub attributes: Vec<HirAttribute>,
    pub ty: TermId,
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
#[derive(Debug, Clone)]
pub enum ComponentMemberDeclaration {
    /// A property declaration with an optional default value.
    ///
    /// # Fields
    ///
    /// - `name` — The property's name
    /// - `modifier` — The visibility modifier (`pub` or private)
    /// - `index` — The property's position in the component's property list
    /// - `value` — The optional default value expression
    /// - `span` — Source location for error reporting
    Property {
        /// The property's name.
        name: SymbolPointer,

        /// The visibility modifier (`pub` or private).
        modifier: VisibilityModifier,

        /// The index of this property in the component's property list.
        ///
        /// Used for efficient property access at runtime.
        index: usize,

        /// The property's default value, if any.
        ///
        /// If `None`, the property must be provided when the component is used.
        value: Option<Spanned<PoolId<HirExpression>>>,

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
    Child(Spanned<PoolId<HirComponentExpression>>),
}

macro_rules! impl_hir_decl {
    ($($t:ty => $name:ident),*$(,)?) => {
        $(
            impl HirDeclaration for $t {
                fn attributes(&self) -> &[HirAttribute] {
                    &self.attributes
                }
                fn attributes_mut(&mut self) -> &mut Vec<HirAttribute> {
                    &mut self.attributes
                }
            }
            impl LanguageItem for $t {
                fn map(items: &crate::context::LangItems) -> &crate::context::LangMap<Self> {
                    &items.$name
                }
            }
        )*
    };
}

use crate::id::AnyDeclarationId;
macro_rules! impl_type_decl {
    ($($t:ty),*$(,)?) => {
        $(
            impl TypeDeclaration for $t {
                fn hir_type(&self) -> TermId {
                    self.ty
                }
                fn generics(&self) -> &[GenericParameter] {
                    &self.generics
                }
                fn name(&self) -> SymbolPointer {
                    self.name
                }
                fn as_any_id(id: DeclarationId<Self>) -> AnyDeclarationId {
                    AnyDeclarationId::from(id)
                }
            }

        )*
    };
}

impl_type_decl!(
    HirFunctionDeclaration,
    HirObjectDeclaration,
    HirComponentDeclaration,
    HirEnumDeclaration,
    HirAliasDeclaration,
);

impl_hir_decl!(
    HirFunctionDeclaration => functions,
    HirObjectDeclaration => objects,
    HirComponentDeclaration => components,
    HirEnumDeclaration => enums,
    HirExtendDeclaration => extensions,
    HirAliasDeclaration => aliases,
    HirStaticDeclaration => statics,
);
