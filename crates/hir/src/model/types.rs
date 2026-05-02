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
//! ```rust
//! # use slynx_frontend::hir::model::HirType;
//!
//! // Primitive types
//! let int_type = HirType::Int;
//! let bool_type = HirType::Bool;
//!
//! // Struct type with fields
//! let struct_type = HirType::Struct {
//!     fields: vec![type_id1, type_id2],
//! };
//!
//! // Function type
//! let func_type = HirType::Function {
//!     args: vec![int_type_id, int_type_id],
//!     return_type: int_type_id,
//! };
//!
//! // Reference to a named type with generics
//! let ref_type = HirType::Reference {
//!     rf: type_id,
//!     generics: vec![int_type_id],
//! };
//! ```
//!
//! # Related Types
//!
//! - [`HirType`] — The main type enum
//! - [`ComponentProperty`] — Property of a component type
//! - [`crate::hir::TypeId`] — Type identifiers
//! - [`crate::hir::modules::TypesModule`] — Type management

use common::{SymbolPointer, VisibilityModifier};

use crate::{TypeId, VariableId};

/// A method for accessing fields on types.
///
/// This enum describes how field accesses are resolved in the type system.
/// Field accesses can target fields on concrete types, variables, or tuples.
///
/// # Variants
///
/// ## `Type`
///
/// Access a field on a concrete type by its type ID and field index.
///
/// ```slynx
/// object Person { name: str, age: int }
/// let p = Person(name: "Maria", age: 30);
/// p.age  // Field(Type(Person, 1))
/// ```
///
/// ## `Variable`
///
/// Access a field on a variable whose type may be a reference to a type.
/// The actual type must be resolved during type checking.
///
/// ```slynx
/// let x = get_person();  // x has type Person
/// x.name  // Field(Variable(x_id, "name"))
/// ```
///
/// ## `Tuple`
///
/// Access a field on a tuple by its numeric index.
///
/// ```slynx
/// let t = (1, "hello", true);
/// t.0  // Field(Tuple(t_id, 0))
/// ```
#[derive(Debug, Clone)]
pub enum FieldMethod {
    /// Access a field on a concrete type.
    ///
    /// # Fields
    ///
    /// - `0` — The type ID of the containing type
    /// - `1` — The index of the field within the type
    Type(TypeId, usize),

    /// Access a field on a variable.
    ///
    /// # Fields
    ///
    /// - `0` — The variable ID
    /// - `1` — The field name as a symbol
    Variable(VariableId, SymbolPointer),

    /// Access a field on a tuple.
    ///
    /// # Fields
    ///
    /// - `0` — The tuple's type ID
    /// - `1` — The numeric index of the field
    Tuple(TypeId, usize),
}

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
pub struct ComponentProperty(VisibilityModifier, String, TypeId);

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
    pub fn new(visibility: VisibilityModifier, name: String, ty: TypeId) -> Self {
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
    pub fn new_public(name: String, ty: TypeId) -> Self {
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
    pub fn new_private(name: String, ty: TypeId) -> Self {
        Self::new(VisibilityModifier::Private, name, ty)
    }

    /// Returns the property's visibility modifier.
    pub fn visibility(&self) -> &VisibilityModifier {
        &self.0
    }

    /// Returns the property's name.
    pub fn name(&self) -> &str {
        &self.1
    }

    /// Returns the property's type ID.
    pub fn prop_type(&self) -> &TypeId {
        &self.2
    }

    /// Returns a mutable reference to the property's type ID.
    pub fn prop_type_mut(&mut self) -> &mut TypeId {
        &mut self.2
    }
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
/// - [`VarReference`](HirType::VarReference) — References to variables
/// - [`Field`](HirType::Field) — Type-level field accesses
///
/// ## Special Types
///
/// - [`Infer`](HirType::Infer) — A placeholder type to be inferred during type checking
/// - [`GenericComponent`](HirType::GenericComponent) — A generic component type
///
/// # Examples
///
/// ```rust
/// # use slynx_frontend::hir::model::HirType;
/// # use crate::slynx_frontend::hir::TypeId;
/// # let type_id = TypeId::from_raw(0);
/// # let field_type = TypeId::from_raw(1);
///
/// // Primitive types
/// let int_type = HirType::Int;
/// let bool_type = HirType::Bool;
/// let void_type = HirType::Void;
///
/// // Struct type
/// let person_type = HirType::Struct {
///     fields: vec![type_id, type_id],
/// };
///
/// // Tuple type
/// let tuple_type = HirType::Tuple {
///     fields: vec![int_type_id, bool_type_id],
/// };
///
/// // Function type: (int, int) -> int
/// let func_type = HirType::Function {
///     args: vec![int_type_id, int_type_id],
///     return_type: int_type_id,
/// };
///
/// // Reference type with generics: Vec<int>
/// let vec_type = HirType::Reference {
///     rf: vec_type_id,
///     generics: vec![int_type_id],
/// };
///
/// // Field access type: person.age
/// let field_type = HirType::Field(field_access_method);
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
#[derive(Debug, Clone)]
pub enum HirType {
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
    /// In HIR, this becomes:
    ///
    /// ```rust
    /// # use slynx_frontend::hir::model::HirType;
    /// # use crate::slynx_frontend::hir::TypeId;
    /// # let str_id = TypeId::from_raw(0);
    /// # let int_id = TypeId::from_raw(1);
    /// let person_type = HirType::Struct {
    ///     fields: vec![str_id, int_id],
    /// };
    /// ```
    Struct {
        /// The type IDs of the struct's fields, in declaration order.
        fields: Vec<TypeId>,
    },

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
    /// In HIR, this becomes:
    ///
    /// ```rust
    /// # use slynx_frontend::hir::model::HirType;
    /// # use crate::slynx_frontend::hir::TypeId;
    /// # let int_id = TypeId::from_raw(0);
    /// # let str_id = TypeId::from_raw(1);
    /// let tuple_type = HirType::Tuple {
    ///     fields: vec![int_id, str_id],
    /// };
    /// ```
    Tuple {
        /// The type IDs of the tuple's elements, in order.
        fields: Vec<TypeId>,
    },

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
    /// # use slynx_frontend::hir::model::HirType;
    /// # use crate::slynx_frontend::hir::TypeId;
    /// # let person_id = TypeId::from_raw(0);
    /// # let int_id = TypeId::from_raw(1);
    /// # let option_id = TypeId::from_raw(2);
    ///
    /// // Reference to Person
    /// let person_ref = HirType::Reference {
    ///     rf: person_id,
    ///     generics: vec![],
    /// };
    ///
    /// // Reference to Option<int>
    /// let option_int = HirType::Reference {
    ///     rf: option_id,
    ///     generics: vec![int_id],
    /// };
    /// ```
    Reference {
        /// The referenced type ID.
        ///
        /// This points to the base type (e.g., the struct or component type).
        rf: TypeId,

        /// Generic type parameters, if any.
        ///
        /// For example, in `Vec<int>`, this would contain `[int]`.
        generics: Vec<TypeId>,
    },

    /// A reference to a variable's type.
    ///
    /// This represents the type of a variable at a specific point in the program.
    /// Used during type checking to resolve variable references.
    ///
    /// # Example
    ///
    /// ```slynx
    /// let x = 42;        // x has type int
    /// let y = x + 1;     // Field(VarReference(x_id), "...")
    /// ```
    VarReference(VariableId),

    /// A field access on a type or variable.
    ///
    /// Represents the type of a field when accessed through a type or variable.
    /// The actual type is resolved during type checking using the [`FieldMethod`].
    ///
    /// # Example
    ///
    /// ```slynx
    /// object Person { name: str, age: int }
    /// let p = Person(name: "Alice", age: 30);
    /// let n = p.name;    // Field(FieldMethod::Variable(p_id, "name"))
    /// let a = p.age;     // Field(FieldMethod::Type(Person, 1))
    /// ```
    Field(FieldMethod),

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
    /// In HIR, this becomes:
    ///
    /// ```rust
    /// # use slynx_frontend::hir::model::HirType;
    /// # use crate::slynx_frontend::hir::TypeId;
    /// # let int_id = TypeId::from_raw(0);
    /// let add_type = HirType::Function {
    ///     args: vec![int_id, int_id],
    ///     return_type: int_id,
    /// };
    /// ```
    Function {
        /// The types of the function's arguments.
        args: Vec<TypeId>,

        /// The function's return type.
        return_type: TypeId,
    },

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
    /// In HIR, this becomes:
    ///
    /// ```rust
    /// # use slynx_frontend::hir::model::{HirType, ComponentProperty};
    /// # use crate::slynx_frontend::hir::TypeId;
    /// # use common::VisibilityModifier;
    /// # let str_id = TypeId::from_raw(0);
    /// # let int_id = TypeId::from_raw(1);
    /// let button_type = HirType::Component {
    ///     props: vec![
    ///         ComponentProperty::new_public("label".into(), str_id),
    ///         ComponentProperty::new_private("count".into(), int_id),
    ///     ],
    /// };
    /// ```
    Component {
        /// The component's properties.
        props: Vec<ComponentProperty>,
    },

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

    /// A type to be inferred.
    ///
    /// Used as a placeholder during type checking when the type cannot be
    /// determined immediately. The type checker will attempt to infer the
    /// concrete type from context.
    ///
    /// # Example
    ///
    /// ```slynx
    /// let x = 42;  // Type is inferred as `int`
    /// ```
    ///
    /// During HIR generation, this is used for:
    /// - Variables without explicit type annotations
    /// - Integer and float literals
    /// - Expressions where type inference is needed
    Infer,

    /// A generic component type.
    ///
    /// Represents a component that can work with generic type parameters.
    GenericComponent,
}

impl HirType {
    /// Creates a new `Generic` type from a generic name.
    ///
    /// # Arguments
    ///
    /// * `generic` — The name of the generic type (e.g., "int", "bool", "Component")
    ///
    /// # Returns
    ///
    /// * `Some(HirType)` — If the name matches a known primitive type
    /// * `None` — If the name is not a recognized primitive type
    ///
    /// # Example
    ///
    /// ```rust
    /// # use slynx_frontend::hir::model::HirType;
    /// let int_type = HirType::new("int");      // Some(HirType::Int)
    /// let bool_type = HirType::new("bool");    // Some(HirType::Bool)
    /// let unknown = HirType::new("Unknown");   // None
    /// ```
    pub fn new(generic: &str) -> Option<Self> {
        match generic {
            "Component" => Some(Self::GenericComponent),
            "void" => Some(Self::Void),
            "bool" => Some(Self::Bool),
            "int" => Some(Self::Int),
            "float" => Some(Self::Float),
            "str" => Some(Self::Str),
            _ => None,
        }
    }

    /// Creates a new struct type with the given field types.
    ///
    /// # Arguments
    ///
    /// * `fields` — The type IDs of the struct's fields, in declaration order
    ///
    /// # Returns
    ///
    /// A new [`HirType::Struct`] instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use slynx_frontend::hir::model::HirType;
    /// # use crate::slynx_frontend::hir::TypeId;
    /// # let str_id = TypeId::from_raw(0);
    /// # let int_id = TypeId::from_raw(1);
    /// let person_type = HirType::new_struct(vec![str_id, int_id]);
    /// ```
    pub fn new_struct(fields: Vec<TypeId>) -> Self {
        Self::Struct { fields }
    }

    /// Creates a new tuple type with the given element types.
    ///
    /// # Arguments
    ///
    /// * `fields` — The type IDs of the tuple's elements, in order
    ///
    /// # Returns
    ///
    /// A new [`HirType::Tuple`] instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use slynx_frontend::hir::model::HirType;
    /// # use crate::slynx_frontend::hir::TypeId;
    /// # let int_id = TypeId::from_raw(0);
    /// # let str_id = TypeId::from_raw(1);
    /// let tuple_type = HirType::new_tuple(vec![int_id, str_id]);
    /// ```
    pub fn new_tuple(fields: Vec<TypeId>) -> Self {
        Self::Tuple { fields }
    }

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
    /// # use slynx_frontend::hir::model::HirType;
    /// # use crate::slynx_frontend::hir::TypeId;
    /// # let vec_id = TypeId::from_raw(0);
    /// # let int_id = TypeId::from_raw(1);
    /// let vec_int = HirType::new_generic_ref(vec_id, vec![int_id]);
    /// ```
    pub fn new_generic_ref(rf: TypeId, generics: Vec<TypeId>) -> Self {
        Self::Reference { rf, generics }
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
    /// # use slynx_frontend::hir::model::HirType;
    /// # use crate::slynx_frontend::hir::TypeId;
    /// # let person_id = TypeId::from_raw(0);
    /// let person_ref = HirType::new_ref(person_id);
    /// ```
    pub fn new_ref(rf: TypeId) -> Self {
        Self::new_generic_ref(rf, Vec::new())
    }

    /// Creates a new function type.
    ///
    /// # Arguments
    ///
    /// * `args` — The types of the function's arguments
    /// * `return_type` — The function's return type
    ///
    /// # Returns
    ///
    /// A new [`HirType::Function`] instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use slynx_frontend::hir::model::HirType;
    /// # use crate::slynx_frontend::hir::TypeId;
    /// # let int_id = TypeId::from_raw(0);
    /// let add_type = HirType::new_function(vec![int_id, int_id], int_id);
    /// ```
    pub fn new_function(args: Vec<TypeId>, return_type: TypeId) -> Self {
        Self::Function { args, return_type }
    }

    /// Creates a new component type.
    ///
    /// # Arguments
    ///
    /// * `props` — The component's properties
    ///
    /// # Returns
    ///
    /// A new [`HirType::Component`] instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use slynx_frontend::hir::model::{HirType, ComponentProperty};
    /// # use crate::slynx_frontend::hir::TypeId;
    /// # use common::VisibilityModifier;
    /// # let str_id = TypeId::from_raw(0);
    /// let button_type = HirType::new_component(vec![
    ///     ComponentProperty::new_public("label".into(), str_id),
    /// ]);
    /// ```
    pub fn new_component(props: Vec<ComponentProperty>) -> Self {
        Self::Component { props }
    }

    /// Creates a field access type from a variable and field name.
    ///
    /// # Arguments
    ///
    /// * `var` — The variable ID
    /// * `field` — The field name as a symbol
    ///
    /// # Returns
    ///
    /// A new [`HirType::Field`] instance with `FieldMethod::Variable`.
    pub fn variable_field(var: VariableId, field: SymbolPointer) -> Self {
        Self::Field(FieldMethod::Variable(var, field))
    }

    /// Creates a field access type from a type and field index.
    ///
    /// # Arguments
    ///
    /// * `ty` — The type ID of the containing type
    /// * `field` — The field index
    ///
    /// # Returns
    ///
    /// A new [`HirType::Field`] instance with `FieldMethod::Type`.
    pub fn type_field(ty: TypeId, field: usize) -> Self {
        Self::Field(FieldMethod::Type(ty, field))
    }
}
