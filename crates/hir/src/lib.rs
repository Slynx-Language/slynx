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
//!         HirDeclarationKind::Function { name, .. } => {
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
//! See [`HirType`] for complete type documentation.
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

mod builders;

/// Scope, symbol, type, and declaration management modules.
pub mod context;

/// HIR error types and diagnostic information.
pub mod error;
/// Name resolution utilities.
mod file;
mod helpers;
/// Unique ID types for HIR elements.
pub mod id;
pub mod model;
mod queries;

use std::ops::{Deref, Index};

pub use crate::error::{HIRError, HIRErrorKind};
use crate::{
    builders::HirQueueBuilder,
    context::{LangItems, SymbolRegistry, TypesContext},
    file::HirFile,
};
use common::{
    FrontendSymbol, SymbolsModule,
    pool::{Pool, PoolId},
};
use dashmap::{DashMap, mapref::one::RefMut};

pub use id::{DeclarationId, ExpressionId, VariableId};
pub use model::*;
use module_loader::{FileId, Modules};

/// Result type for HIR operations.
///
/// This is the standard result type used throughout the HIR module, wrapping
/// successful values or [`HIRError`] instances with detailed diagnostic information.
pub type Result<T> = std::result::Result<T, HIRError>;
pub type SymbolPointer = common::SymbolPointer<FrontendSymbol>;

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

#[derive(Debug)]
pub struct SlynxHir<'a> {
    /// Resolver for interning and looking up symbol names.
    pub symbols_resolver: &'a SymbolsModule<FrontendSymbol>,
    pub symbols_registry: SymbolRegistry,
    /// Module managing all types and their IDs.
    pub types_module: TypesContext,
    pub expressions: Pool<HirExpression>,
    pub statements: Pool<HirStatement>,
    pub component_expressions: Pool<HirComponentExpression>,
    /// All top-level declarations generated from the sources.
    ///
    /// This vector contains every function, component, object, and type alias
    /// defined in the source code, in the order they were processed.
    ///
    /// Each declaration includes:
    /// - A unique [`DeclarationId`] for identification
    /// - Its [`HirDeclarationKind`] describing what kind of declaration it is
    /// - The declaration's [`TypeId`]
    /// - The source [`Span`] for error reporting
    pub files: DashMap<FileId, HirFile>,
    pub lang_items: LangItems,
}

impl<'a> SlynxHir<'a> {
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
    pub fn new(modules: &'a Modules<'a>) -> std::result::Result<Self, (Self, HIRError)> {
        let out = Self {
            expressions: Pool::new(),
            statements: Pool::new(),
            component_expressions: Pool::new(),
            symbols_registry: SymbolRegistry::default(),
            symbols_resolver: modules.symbols(),
            types_module: TypesContext::new(),
            files: DashMap::new(),
            lang_items: LangItems::new(),
        };
        if let Err(e) = out.generate(modules) {
            Err((out, e))
        } else {
            Ok(out)
        }
    }

    ///Gets or create an Hir file with the given `id`
    fn get_or_create_file(&self, id: FileId) -> RefMut<'_, FileId, HirFile> {
        self.files.entry(id).or_insert_with(|| HirFile::new(id))
    }

    fn generate(&'a self, modules: &'a Modules<'a>) -> Result<()> {
        let mut builder = HirQueueBuilder::new(self, modules);
        let entry = &modules.entries()[0];
        for func in entry.func() {
            let node = builder.get_node(entry.id);
            builder.enqueue_function(func, node)?;
        }
        builder.close_bodies();
        builder.process()?;
        Ok(())
    }
}

impl<'a> Deref for SlynxHir<'a> {
    type Target = TypesContext;
    fn deref(&self) -> &Self::Target {
        &self.types_module
    }
}

impl Index<PoolId<HirExpression>> for SlynxHir<'_> {
    type Output = HirExpression;
    fn index(&self, index: PoolId<HirExpression>) -> &Self::Output {
        &self.expressions[index]
    }
}

impl Index<PoolId<HirStatement>> for SlynxHir<'_> {
    type Output = HirStatement;
    fn index(&self, index: PoolId<HirStatement>) -> &Self::Output {
        &self.statements[index]
    }
}

impl Index<PoolId<HirComponentExpression>> for SlynxHir<'_> {
    type Output = HirComponentExpression;
    fn index(&self, index: PoolId<HirComponentExpression>) -> &Self::Output {
        &self.component_expressions[index]
    }
}
