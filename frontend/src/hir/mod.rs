//! High-Level Intermediate Representation (HIR)
//!
//! The HIR (High-Level Intermediate Representation) module is responsible for transforming
//! the Abstract Syntax Tree (AST) into a more semantically rich representation that preserves
//! the original code's meaning while preparing it for type analysis and code generation.
//!
//! # Overview
//!
//! The HIR serves as a bridge between the syntactic representation (AST) and lower-level
//! representations (MIR, IR). It provides:
//! - Rich type information with explicit type relationships
//! - Scoped variable and declaration tracking
//! - Structured representation of components, functions, and objects
//! - Type inference support through `HirType::Infer`
//!
//! # Architecture
//!
//! The module is organized into several submodules, each with distinct responsibilities:
//!
//! - **[`model`]** — Data structures representing HIR elements (declarations, expressions, types)
//! - **[`implementation`]** — Logic for transforming AST into HIR
//! - **[`modules`]** — Management of scopes, symbols, types, and declarations
//! - **[`helpers`]** — Utility functions for constructing HIR elements
//!
//! # Quick Start
//!
//! ```rust
//! use crate::hir::SlynxHir;
//! use common::ast::ASTDeclaration;
//!
//! // Create a new HIR instance
//! let mut hir = SlynxHir::new();
//!
//! // Transform AST declarations into HIR
//! let ast: Vec<ASTDeclaration> = /* parsed AST */;
//! hir.generate(ast)?;
//!
//! // Access the resulting HIR
//! for decl in &hir.declarations {
//!     match &decl.kind {
//!         hir::model::HirDeclarationKind::Function { name, .. } => {
//!             println!("Function: {}", hir.names.symbol_name(name));
//!         }
//!         _ => {}
//!     }
//! }
//! ```
//!
//! # Type System
//!
//! The HIR uses a rich type system represented by [`HirType`] that includes:
//! - Primitive types: `int`, `float`, `str`, `bool`, `void`
//! - Composite types: `struct`, `tuple`, `function`, `component`
//! - Reference types with generics
//! - Special types: `infer` for type inference
//!
//! See [`model::types::HirType`] for complete type documentation.
//!
//! # Error Handling
//!
//! HIR operations return [`Result<T, HIRError>`] where [`HIRError`] provides detailed
//! diagnostic information including source spans and error kinds.
//!
//! Common errors include:
//! - [`HIRErrorKind::NameNotRecognized`] — Undefined identifier
//! - [`HIRErrorKind::PropertyNotRecognized`] — Invalid field access
//! - [`HIRErrorKind::NotAFunction`] — Call of non-function value
//! - [`HIRErrorKind::MissingProperty`] — Missing required object fields

#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

pub mod error;
mod helpers;
pub mod id;
mod implementation;
pub mod model;
pub mod modules;
pub mod names;

use crate::hir::{
    error::HIRError,
    model::{HirDeclaration, HirDeclarationKind, HirType},
    modules::HirModules,
};
use common::{
    SymbolPointer,
    ast::{ASTDeclaration, ASTDeclarationKind},
};

pub use id::{DeclarationId, ExpressionId, PropertyId, TypeId, VariableId};

/// Result type for HIR operations.
///
/// This is the standard result type used throughout the HIR module, wrapping
/// successful values or [`HIRError`] instances with detailed diagnostic information.
pub type Result<T> = std::result::Result<T, HIRError>;

/// The main HIR structure coordinating high-level intermediate representation.
///
/// `SlynxHir` manages the transformation of AST declarations into a complete HIR
/// representation. It maintains all declarations, type information, and provides
/// the context for type checking and analysis.
///
/// # Structure
///
/// The HIR is built through a two-phase process:
///
/// 1. **Hoisting Phase** — Top-level declarations (functions, components, objects)
///    are registered in their respective scopes before their bodies are resolved.
///    This ensures forward references are valid (e.g., calling a function defined
///    later in the source).
///
/// 2. **Resolution Phase** — The bodies of functions and components are processed,
///    expressions are typed, and variable references are resolved to their
///    declarations.
///
/// # Fields
///
/// - [`modules`](SlynxHir::modules) — Manages scopes, symbols, types, and declarations
/// - [`declarations`](SlynxHir::declarations) — All top-level declarations in the HIR
/// - `types` — Internal cache mapping type IDs to their [`HirType`] representations
///
/// # Example
///
/// ```rust
/// # use slynx_frontend::hir::{SlynxHir, Result};
/// # use common::ast::{ASTDeclaration, ASTDeclarationKind, GenericIdentifier, Span};
/// # fn example() -> Result<()> {
/// let mut hir = SlynxHir::new();
///
/// // The HIR is populated by generating from AST
/// let ast: Vec<ASTDeclaration> = vec![
///     // Your parsed declarations here
/// ];
///
/// hir.generate(ast)?;
///
/// // Now hir.declarations contains the full HIR
/// assert!(!hir.declarations.is_empty());
/// # Ok(())
/// # }
/// ```
///
/// # See Also
///
/// - [`generate`](SlynxHir::generate) — Main entry point for AST → HIR transformation
/// - [`model`] module — HIR data structures
/// - [`modules::HirModules`] — Scopes and symbol management
#[derive(Debug, Default)]
pub struct SlynxHir {
    /// Manages all modules, scopes, symbols, and type information.
    ///
    /// This is the primary interface for working with the HIR's namespace and
    /// type system. It provides methods for creating variables, looking up
    /// declarations, and managing nested scopes.
    pub modules: HirModules,

    /// All top-level declarations generated from the source.
    ///
    /// This vector contains every function, component, object, and type alias
    /// defined in the source code, in the order they were processed.
    ///
    /// Each declaration includes:
    /// - A unique [`DeclarationId`] for identification
    /// - Its [`HirDeclarationKind`] describing what kind of declaration it is
    /// - The declaration's [`TypeId`]
    /// - The source [`Span`] for error reporting
    pub declarations: Vec<HirDeclaration>,
}

impl SlynxHir {
    /// Creates a new, empty `SlynxHir` instance.
    ///
    /// The returned instance has no declarations and an initialized module
    /// context with built-in types (int, float, str, bool, void, etc.).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use slynx_frontend::hir::SlynxHir;
    /// let hir = SlynxHir::new();
    /// assert!(hir.declarations.is_empty());
    /// ```
    ///
    /// # See Also
    ///
    /// - [`generate`](SlynxHir::generate) — Populate the HIR from AST
    /// - [`modules::HirModules::new`](crate::hir::modules::HirModules::new)
    #[inline]
    pub fn new() -> Self {
        Self {
            modules: HirModules::new(),
            declarations: Vec::new(),
        }
    }

    /// Generates HIR declarations from the provided AST declarations.
    ///
    /// This is the primary entry point for transforming source code into the
    /// high-level intermediate representation. The process occurs in two phases:
    ///
    /// ## Phase 1: Hoisting
    ///
    /// Each declaration is hoisted to register it in the appropriate scope before
    /// its body is resolved. This allows forward references within the same
    /// scope. For example:
    ///
    /// ```slynx
    /// func later() { earlier(); }  // Valid: earlier is hoisted
    /// func earlier() { }
    /// ```
    ///
    /// During hoisting:
    /// - Functions are registered with their signatures
    /// - Components have their property layouts established
    /// - Objects declare their field structure
    /// - Type aliases create name → type mappings
    ///
    /// ## Phase 2: Resolution
    ///
    /// The bodies of declarations are processed to:
    /// - Type-check expressions and statements
    /// - Resolve variable and function references
    /// - Validate field accesses and method calls
    /// - Build the complete HIR representation
    ///
    /// # Arguments
    ///
    /// * `ast` — A vector of AST declarations to transform into HIR
    ///
    /// # Returns
    ///
    /// * [`Ok(())`] — HIR generation succeeded
    /// * [`Err(HIRError)`] — A semantic error was encountered
    ///
    /// # Errors
    ///
    /// This function can return various [`HIRError`] kinds, including:
    ///
    /// - [`NameNotRecognized`] — Reference to undefined identifier
    /// - [`PropertyNotRecognized`] — Invalid field access on object/component
    /// - [`NotAFunction`] — Attempt to call a non-function value
    /// - [`MissingProperty`] — Required object field not provided
    /// - [`RecursiveType`] — Illegal recursive type definition
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use slynx_frontend::hir::{SlynxHir, Result};
    /// # use common::ast::{ASTDeclaration, ASTDeclarationKind};
    /// # fn process(source: Vec<ASTDeclaration>) -> Result<()> {
    /// let mut hir = SlynxHir::new();
    /// hir.generate(source)?;
    ///
    /// // HIR is now ready for analysis
    /// for decl in &hir.declarations {
    ///     println!("Decl: {:?}", decl.kind);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Implementation Details
    ///
    /// The function iterates over the AST declarations twice:
    ///
    /// 1. First pass (hoisting): Calls [`hoist`](SlynxHir::hoist) on each declaration
    ///    to register it without resolving its body
    ///
    /// 2. Second pass (resolution): Calls [`resolve`](SlynxHir::resolve) on each
    ///    declaration to process its full body and build HIR nodes
    ///
    /// This two-pass approach ensures that all names are available before any
    /// references to them are resolved, supporting mutual recursion.
    ///
    /// # See Also
    ///
    /// - [`hoist`](SlynxHir::hoist) — Phase 1: Declaration registration
    /// - [`resolve`](SlynxHir::resolve) — Phase 2: Body resolution
    /// - [`modules::HirModules`] — Scope and symbol management during generation
    pub fn generate(&mut self, ast: Vec<ASTDeclaration>) -> Result<()> {
        // Phase 1: Hoist all declarations to register them in their scopes
        for ast in &ast {
            self.hoist(ast)?;
        }

        // Phase 2: Resolve all declaration bodies to build complete HIR
        for ast in ast {
            self.resolve(ast)?;
        }

        Ok(())
    }

    /// Hoists a single AST declaration, registering it in its scope without
    /// resolving its body.
    ///
    /// Hoisting is the first phase of HIR generation where declarations are
    /// made known to the type system before their implementations are processed.
    /// This enables:
    ///
    /// - Forward references within the same scope
    /// - Mutual recursion between functions
    /// - Type-safe references to later-defined declarations
    ///
    /// # Arguments
    ///
    /// * `ast` — The AST declaration to hoist
    ///
    /// # Returns
    ///
    /// * [`Ok(())`] — Declaration was successfully registered
    /// * [`Err(HIRError)`] — A semantic error occurred during hoisting
    ///
    /// # Processing by Declaration Kind
    ///
    /// | Declaration Kind | Hoisting Action |
    /// |-----------------|----------------|
    /// | [`Alias`] | Creates type alias mapping |
    /// | [`ObjectDeclaration`] | Registers object layout with fields |
    /// | [`FuncDeclaration`] | Registers function signature |
    /// | [`ComponentDeclaration`] | Registers component with property types |
    ///
    /// # Errors
    ///
    /// - [`RecursiveType`] — Object field references its own type
    /// - [`NameAlreadyDefined`] — Duplicate declaration in same scope
    ///
    /// # Note
    ///
    /// This method is called during the first pass of [`generate`](SlynxHir::generate).
    /// It does not process function bodies, component children, or expression
    /// details — those are handled during the resolution phase.
    ///
    /// # See Also
    ///
    /// - [`generate`](SlynxHir::generate) — Main entry point (calls this in phase 1)
    /// - [`resolve`](SlynxHir::resolve) — Phase 2: Body resolution
    /// - [`implementation::declarations::hoist_function`](crate::hir::implementation::declarations::hoist_function)
    fn hoist(&mut self, ast: &ASTDeclaration) -> Result<()> {
        match &ast.kind {
            ASTDeclarationKind::Alias { name, target } => {
                self.modules
                    .create_alias(&target.identifier, &name.identifier);
            }
            ASTDeclarationKind::ObjectDeclaration { name, fields } => {
                self.modules.create_object(&name.identifier, fields)
            }

            ASTDeclarationKind::FuncDeclaration {
                name,
                args,
                return_type,
                ..
            } => self.hoist_function(name, args, return_type)?,
            ASTDeclarationKind::ComponentDeclaration { name, members, .. } => {
                self.hoist_component(name, members)?
            }
        }
        Ok(())
    }

    /// Resolves an AST declaration, processing its full body to build HIR nodes.
    ///
    /// Resolution is the second phase of HIR generation where the actual
    /// implementation details are processed. This includes:
    ///
    /// - Type-checking all expressions
    /// - Resolving variable and function references
    /// - Building [`HirExpression`] and [`HirStatement`] nodes
    /// - Validating field accesses and type correctness
    ///
    /// # Arguments
    ///
    /// * `ast` — The AST declaration to resolve (consumed)
    ///
    /// # Returns
    ///
    /// * [`Ok(())`] — Declaration was successfully resolved and added to [`declarations`](SlynxHir::declarations)
    /// * [`Err(HIRError)`] — A type error or semantic error was encountered
    ///
    /// # Processing by Declaration Kind
    ///
    /// | Declaration Kind | Resolution Action |
    /// |-----------------|-------------------|
    /// | [`ObjectDeclaration`] | Validates field types and records object structure |
    /// | [`FuncDeclaration`] | Processes body statements, resolves expressions, handles implicit returns |
    /// | [`ComponentDeclaration`] | Resolves child members and property initializers |
    /// | [`Alias`] | Links alias name to target type |
    ///
    /// # Errors
    ///
    /// Resolution can fail with various [`HIRErrorKind`] values, including:
    ///
    /// - [`TypeNotRecognized`] — Unknown type name
    /// - [`NotAFunction`] — Call to non-function value
    /// - [`PropertyNotRecognized`] — Invalid field or property access
    /// - [`InvalidFuncallArgLength`] — Wrong number of function arguments
    ///
    /// # Implementation Details
    ///
    /// For functions and components, this method:
    ///
    /// 1. Enters a new scope for local variables
    /// 2. Processes parameters to create variable bindings
    /// 3. Resolves all statements in the body
    /// 4. Handles implicit returns (last expression in function body)
    /// 5. Exits the scope and records the declaration
    ///
    /// # Note
    ///
    /// This method is called during the second pass of [`generate`](SlynxHir::generate).
    /// All names should already be registered via [`hoist`](SlynxHir::hoist).
    ///
    /// # See Also
    ///
    /// - [`generate`](SlynxHir::generate) — Main entry point (calls this in phase 2)
    /// - [`hoist`](SlynxHir::hoist) — Phase 1: Declaration registration
    /// - [`implementation::declarations::resolve_function`](crate::hir::implementation::declarations::resolve_function)
    fn resolve(&mut self, ast: ASTDeclaration) -> Result<()> {
        match ast.kind {
            ASTDeclarationKind::ObjectDeclaration { name, fields, .. } => {
                self.resolve_object(name, fields, ast.span)?
            }
            ASTDeclarationKind::FuncDeclaration {
                name, args, body, ..
            } => self.resolve_function(&name, &args, body, &ast.span)?,
            ASTDeclarationKind::ComponentDeclaration { members, name } => {
                self.modules.enter_scope();
                let symbol = self.modules.intern_name(&name.identifier);
                let Some((decl, ty)) = self.modules.get_declaration_by_name(&symbol) else {
                    return Err(HIRError::name_unrecognized(symbol, ast.span));
                };

                let defs = self.resolve_component_defs(members)?;
                self.declarations.push(HirDeclaration {
                    id: decl,
                    kind: HirDeclarationKind::ComponentDeclaration {
                        props: defs,
                        name: symbol,
                    },
                    ty,
                    span: ast.span,
                });
                self.modules.exit_scope();
            }
            ASTDeclarationKind::Alias { name, target } => {
                let target_ty = self.get_typeid_of_name(&target.identifier, &target.span)?;

                let alias_name = self.modules.intern_name(&name.identifier);
                let Some(alias_ty) = self
                    .modules
                    .types_module
                    .get_type_from_name_mut(&alias_name)
                else {
                    return Err(HIRError::name_unrecognized(alias_name, name.span));
                };
                *alias_ty = HirType::new_ref(target_ty);
                let Some((decl, ty)) = self.modules.get_declaration_by_name(&alias_name) else {
                    return Err(HIRError::name_unrecognized(alias_name, name.span));
                };
                self.declarations
                    .push(HirDeclaration::new_alias(decl, ty, ast.span));
            }
        }
        Ok(())
    }
}
