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

use crate::{
    SymbolPointer,
    context::{ComponentDefinition, StructDefinition, StyleMetadata},
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
pub struct StyleType {
    ///The arguments the stylesheet receives
    pub(crate) args: SmallVec<[DedupPoolId<HirType>; 2]>,
    pub(crate) metadata: DedupPoolId<StyleMetadata>,
}

/// The type system for the HIR.
///
/// `HirType` represents all possible types in the Slynx language. Each variant
/// corresponds to a different kind of type, from primitive types to complex
/// user-defined types.
///
/// # Type Categories
///
/// ## Primitive Types
///
/// - [`Bool`](HirType::Bool) — Boolean values (`true` or `false`)
/// - [`Float`](HirType::Float) — 32-bit floating-point numbers
/// - [`Int`](HirType::Int) — 32-bit signed integers
/// - [`Str`](HirType::Str) — String values
/// - [`Void`](HirType::Void) — The absence of a value
///
/// ## Composite Types
///
/// - [`Struct`](HirType::Struct) — User-defined data structures with named fields
/// - [`Tuple`](HirType::Tuple) — Anonymous fixed-length sequences of values
/// - [`Function`](HirType::Function) — Function signatures with argument and return types
/// - [`Component`](HirType::Component) — UI components with properties
///
/// ## Reference Types
///
/// - [`Reference`](HirType::Reference) — References to named types with optional generics
///
/// ## Special Types
///
/// - [`GenericComponent`](HirType::GenericComponent) — A generic component type
///
/// # Examples
///
/// ```text
/// // Primitive types
/// let int_type = HirType::Int;
/// let bool_type = HirType::Bool;
/// let void_type = HirType::Void;
///
/// // Composite types are created through the HIR's type module and
/// // referenced by their `DedupPoolId<HirType>`:
/// let person_type = HirType::Struct(struct_type_id); // Person
/// let tuple_type = HirType::Tuple(tuple_type_id);
/// let func_type = HirType::Function(function_type_id);
///
/// // Reference type with generics: Vec<int>
/// let vec_type = HirType::new_generic_ref(vec_type_id, vec![int_type_id]);
///
/// // Type to be inferred
/// let infer_type = HirType::Infer;
/// ```
///
/// # Type Operations
///
/// The [`HirType`] enum provides several associated functions for creating
/// common type patterns:
///
/// - [`new_struct`](HirType::new_struct) — Create a struct type
/// - [`new_tuple`](HirType::new_tuple) — Create a tuple type
/// - [`new_function`](HirType::new_function) — Create a function type
/// - [`new_component`](HirType::new_component) — Create a component type
/// - [`new_ref`](HirType::new_ref) — Create a reference type
///
/// # See Also
///
/// - [`crate::hir::modules::TypesModule`] — Manages type creation and lookup
/// - [`crate::hir::TypeId`] — Type identifiers
/// - [`crate::hir::model::ComponentProperty`] — Component property definitions
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum HirType {
    Nullable(DedupPoolId<HirType>),
    GenericParam {
        index: u8,
        name: SymbolPointer,
    },

    Array(DedupPoolId<HirType>, usize),
    Vector(DedupPoolId<HirType>),
    /// A struct type with named fields.
    ///
    /// Structs are user-defined data structures with a fixed set of named fields.
    /// Each field has a type, and fields are accessed by name.
    ///
    /// # Example
    ///
    /// ```slynx
    /// object Person {
    ///     name: str,
    ///     age: int,
    /// }
    /// ```
    ///
    /// In HIR, this becomes a reference to a [`StructType`]:
    ///
    /// ```text
    /// let person_type = HirType::Struct(struct_type_id);
    /// ```
    Struct(DedupPoolId<StructType>),

    /// A tuple type with positional fields.
    ///
    /// Tuples are anonymous fixed-length sequences of values. Each element
    /// can have a different type, and elements are accessed by numeric index.
    ///
    /// # Example
    ///
    /// ```slynx
    /// let pair = (1, "hello");
    /// let first = pair.0;  // 1
    /// ```
    ///
    /// In HIR, this becomes a reference to a [`TupleType`]:
    ///
    /// ```text
    /// let tuple_type = HirType::Tuple(tuple_type_id);
    /// ```
    Tuple(DedupPoolId<TupleType>),

    /// An immutable reference to a type. The difference of this to Reference type is that this represents a reference to a value with the given type, equivalent to C's pointer, the reference type is a reference to another type
    ImutableRef(DedupPoolId<HirType>),

    /// A mutable reference to a type.
    MutableRef(DedupPoolId<HirType>),

    /// A reference to a named type.
    ///
    /// References are used to refer to user-defined types (structs, components,
    /// etc.) and can include generic type parameters.
    ///
    /// # Example
    ///
    /// ```slynx
    /// object Person { name: str }
    ///
    /// // Reference to Person type
    /// let p: Person = Person(name: "Alice");
    ///
    /// // Generic reference: Option<int>
    /// type Option<T> { value: T }
    /// let opt: Option<int> = Option(value: 42);
    /// ```
    ///
    /// In HIR, these become:
    ///
    /// ```rust
    /// # use slynx_hir::model::HirType;
    /// # use common::pool::DedupPoolId;
    /// # let person_id = DedupPoolId::<HirType>::new(0);
    /// # let int_id = DedupPoolId::<HirType>::new(1);
    /// # let option_id = DedupPoolId::<HirType>::new(2);
    ///
    /// // Reference to Person
    /// let person_ref = HirType::new_ref(person_id);
    ///
    /// // Reference to Option<int>
    /// let option_int = HirType::new_generic_ref(option_id, vec![int_id]);
    /// ```
    Reference {
        /// The referenced type ID.
        ///
        /// This points to the base type (e.g., the struct or component type).
        rf: DedupPoolId<HirType>,

        /// Generic type parameters, if any.
        ///
        /// For example, in `Vec<int>`, this would contain `[int]`.
        generics: [DedupPoolId<HirType>; 8],
    },

    /// A function type.
    ///
    /// Represents the signature of a function, including its argument types
    /// and return type.
    ///
    /// # Example
    ///
    /// ```slynx
    /// func add(a: int, b: int): int {
    ///     a + b
    /// }
    /// ```
    ///
    /// In HIR, this becomes a reference to a [`FunctionType`]:
    ///
    /// ```text
    /// let add_type = HirType::Function(function_type_id);
    /// ```
    Function(DedupPoolId<FunctionType>),
    ///A Stylesheet definition
    Style(DedupPoolId<StyleType>),

    /// A boolean type.
    ///
    /// Represents boolean values: `true` or `false`.
    Bool,

    /// A 32-bit floating-point type.
    ///
    /// Represents floating-point numbers.
    Float,

    /// A 32-bit signed integer type.
    ///
    /// Represents integer values.
    Int,

    /// A string type.
    ///
    /// Represents text values.
    Str,

    /// A component type.
    ///
    /// Represents UI components with their properties.
    ///
    /// # Example
    ///
    /// ```slynx
    /// component Button(props: ButtonProps) {
    ///     pub label: str = "Click me";
    ///     private count: int = 0;
    /// }
    /// ```
    ///
    /// In HIR, this becomes a reference to a [`ComponentType`]:
    ///
    /// ```text
    /// let button_type = HirType::Component(component_type_id);
    /// ```
    Component(DedupPoolId<ComponentType>),

    /// The void type.
    ///
    /// Represents the absence of a value. Used for functions that don't return
    /// a value (or return nothing).
    ///
    /// # Example
    ///
    /// ```slynx
    /// func print_hello(): void {
    ///     print("Hello");
    /// }
    /// ```
    Void,

    /// A generic component type.
    ///
    /// Represents a component that can work with generic type parameters.
    GenericComponent,
}

impl HirType {
    /// Creates a new generic reference type.
    ///
    /// # Arguments
    ///
    /// * `rf` — The referenced type ID
    /// * `generics` — The generic type parameters
    ///
    /// # Returns
    ///
    /// A new [`HirType::Reference`] instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use slynx_hir::model::HirType;
    /// # use common::pool::DedupPoolId;
    /// # let vec_id = DedupPoolId::<HirType>::new(0);
    /// # let int_id = DedupPoolId::<HirType>::new(1);
    /// let vec_int = HirType::new_generic_ref(vec_id, vec![int_id]);
    /// ```
    pub fn new_generic_ref(rf: DedupPoolId<HirType>, generics: Vec<DedupPoolId<HirType>>) -> Self {
        assert!(generics.len() <= 8, "Slynx only supports up to 8 generics");
        let mut arr = [DedupPoolId::new_null(); 8];
        for (idx, generic) in generics.into_iter().enumerate() {
            arr[idx] = generic;
        }
        Self::Reference { rf, generics: arr }
    }

    /// Creates a new reference type without generics.
    ///
    /// This is a convenience method for creating simple references.
    ///
    /// # Arguments
    ///
    /// * `rf` — The referenced type ID
    ///
    /// # Returns
    ///
    /// A new [`HirType::Reference`] instance with empty generics.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use slynx_hir::model::HirType;
    /// # use common::pool::DedupPoolId;
    /// # let person_id = DedupPoolId::<HirType>::new(0);
    /// let person_ref = HirType::new_ref(person_id);
    /// ```
    pub fn new_ref(rf: DedupPoolId<HirType>) -> Self {
        Self::new_generic_ref(rf, Vec::new())
    }
}

impl From<ASTBuiltin> for HirType {
    fn from(value: ASTBuiltin) -> Self {
        match value {
            ASTBuiltin::Boolean => Self::Bool,
            ASTBuiltin::F16 | ASTBuiltin::F32 | ASTBuiltin::F64 => Self::Float,
            ASTBuiltin::Int(_) | ASTBuiltin::Uint(_) => Self::Int,
            ASTBuiltin::Void => Self::Void,
            ASTBuiltin::Str => Self::Str,
            ASTBuiltin::AnyComponent => Self::GenericComponent,
        }
    }
}
