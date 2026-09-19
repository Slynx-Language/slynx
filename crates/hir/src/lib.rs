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
//! ```text
//! // Parse source files into an AST with the parser, then generate the HIR:
//! let mut hir = SlynxHir::new(&modules)?;
//! // hir.files contains the full HIR for every compiled file.
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
pub use builders::*;
/// Scope, symbol, type, and declaration management modules.
pub mod context;

/// HIR error types and diagnostic information.
pub mod error;
/// Name resolution utilities.
mod file;
/// Shared generic-related infrastructure.
pub mod generics;
mod helpers;
/// Unique ID types for HIR elements.
pub mod id;
pub mod model;
/// Ownership analysis: move semantics, borrow checking, and place construction.
pub mod ownership;
mod queries;
mod store;

use std::ops::Index;

pub use crate::error::{HIRError, HIRErrorKind};
use crate::{
    context::{SymbolRegistry, TypesContext},
    file::HirFile,
};
use common::{FrontendSymbol, SymbolsModule, pool::PoolId};
pub use helpers::{HirViewer, Visible};

pub use id::{ComponentId, DeclarationId, ExpressionId, VariableId};
pub use model::*;
use module_loader::Modules;

/// Result type for HIR operations.
///
/// This is the standard result type used throughout the HIR module, wrapping
/// successful values or [`HIRError`] instances with detailed diagnostic information.
pub type Result<T> = std::result::Result<T, HIRError>;
pub type SymbolPointer = common::SymbolPointer<FrontendSymbol>;

pub use crate::file::DeclarationsPool;
pub use store::HirStore;

/// The main HIR structure coordinating high-level intermediate representation.
///
/// `SlynxHir` is a read-only facade that provides query access to all HIR data
/// via explicit fields: [`store`](SlynxHir::store) for data pools,
/// [`types`](SlynxHir::types) for the type system, and
/// [`symbols_registry`](SlynxHir::symbols_registry) for declaration lookup.
///
/// # Example
///
/// ```text
/// let hir = SlynxHir::new(&modules)?;
/// // hir.store.files contains the full HIR generated from the parsed modules.
/// ```
///
/// # See Also
///
/// - [`store::HirStore`] — data pools (expressions, statements, files, …)
/// - [`TypesContext`] — type storage, registry, and method table
/// - [`context::SymbolRegistry`] — name→declaration lookup
/// - [`model`] — HIR data structures
#[derive(Debug)]
pub struct SlynxHir<'a> {
    /// Resolver for interning and looking up symbol names.
    pub symbols_resolver: &'a SymbolsModule<FrontendSymbol>,
    pub symbols_registry: SymbolRegistry,
    /// The type system: type storage, name registry, and method table.
    pub types: TypesContext,
    /// All HIR data pools and file declarations.
    pub store: HirStore,
}

impl<'a> SlynxHir<'a> {
    /// Creates a new `SlynxHir` instance by generating the HIR from the given AST modules.
    ///
    /// The returned instance has built-in types registered and all top-level
    /// declarations hoisted and their bodies resolved.
    ///
    /// # See Also
    ///
    /// - [`store::HirStore`] — data pools
    /// - [`crate::builders::generate_hir`] — the build orchestration
    #[inline]
    #[allow(clippy::result_large_err)]
    pub fn new(modules: &'a Modules<'a>) -> std::result::Result<Self, (Self, HIRError)> {
        let out = Self {
            symbols_resolver: modules.symbols(),
            symbols_registry: SymbolRegistry::default(),
            types: TypesContext::new(),
            store: HirStore::new(),
        };
        if let Err(e) = crate::builders::generate_hir(&out, modules) {
            Err((out, e))
        } else {
            Ok(out)
        }
    }
}

impl Index<PoolId<HirExpression>> for SlynxHir<'_> {
    type Output = HirExpression;
    fn index(&self, index: PoolId<HirExpression>) -> &Self::Output {
        &self.store[index]
    }
}

impl Index<PoolId<HirStatement>> for SlynxHir<'_> {
    type Output = HirStatement;
    fn index(&self, index: PoolId<HirStatement>) -> &Self::Output {
        &self.store[index]
    }
}

impl Index<PoolId<HirComponentExpression>> for SlynxHir<'_> {
    type Output = HirComponentExpression;
    fn index(&self, index: PoolId<HirComponentExpression>) -> &Self::Output {
        &self.store[index]
    }
}

impl Index<PoolId<HirPlace>> for SlynxHir<'_> {
    type Output = HirPlace;
    fn index(&self, index: PoolId<HirPlace>) -> &Self::Output {
        &self.store[index]
    }
}
