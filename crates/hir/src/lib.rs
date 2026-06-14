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

#![warn(rustdoc::broken_intra_doc_links)]
pub mod module_loader;

mod components;
/// Scope, symbol, type, and declaration management modules.
pub mod context;
mod declarations;
/// HIR error types and diagnostic information.
pub mod error;
mod expression;
mod helpers;
/// Unique ID types for HIR elements.
pub mod id;
pub mod model;
/// Name resolution utilities.
pub mod names;
mod queries;
mod statements;

mod file;

pub use crate::error::{HIRError, HIRErrorKind};
use crate::{
    context::{BUILTIN_NAMES, LangItems, SymbolsResolver, TypesContext},
    file::HirFile,
    module_loader::{FileId, SourceNode},
};
use common::SymbolsModule;
use parking_lot::RwLock;
use slynx_parser::{ASTDeclaration, ASTDeclarationKind};

pub use id::{DeclarationId, ExpressionId, PropertyId, TypeId, VariableId};
pub use model::*;

/// Result type for HIR operations.
///
/// This is the standard result type used throughout the HIR module, wrapping
/// successful values or [`HIRError`] instances with detailed diagnostic information.
pub type Result<T> = std::result::Result<T, HIRError>;
pub type SymbolPointer = common::SymbolPointer<SlynxHir>;

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
    /// Resolver for interning and looking up symbol names.
    pub symbols_resolver: SymbolsResolver,
    /// Module managing all types and their IDs.
    pub types_module: TypesContext,
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
    pub files: Vec<RwLock<HirFile>>,
    pub lang_items: LangItems,
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
        let symbols = SymbolsModule::new();
        let builtins = BUILTIN_NAMES.map(|v| symbols.intern(v));
        Self {
            symbols_resolver: SymbolsResolver::new(symbols),
            types_module: TypesContext::new(&builtins),
            files: Vec::new(),
            lang_items: LangItems::new(),
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
    pub fn generate(&mut self, modules: &[SourceNode]) -> Result<()> {
        // Phase 0: allocate file slots (requires &mut self for Vec growth)
        for module in modules {
            let idx = module.id.as_raw() as usize;
            if idx >= self.files.len() {
                self.files
                    .resize_with(idx + 1, || RwLock::new(HirFile::new(FileId::from_raw(0))));
            }
            *self.files[idx].write() = HirFile::new(module.id);
        }
        for module in modules {
            for ast in &module.declarations {
                let mut should_register = None;
                for attribute in &ast.attributes {
                    if attribute.name == "intrinsic" {
                        should_register = attribute.args.first();
                        break;
                    }
                }
                self.hoist(ast, module.id, should_register)?;
            }
        }

        // Phase 2: resolve type-level definitions (object fields, alias targets, import aliases)
        // before body resolution so cross-module type dependencies are available.
        for module in modules {
            let mut import_idx = 0;
            for ast in &module.declarations {
                if matches!(ast.kind, ASTDeclarationKind::Import(_)) {
                    let submodules = &module.import_submodules[import_idx];
                    self.resolve_type(ast, module.id, submodules)?;
                    import_idx += 1;
                } else {
                    self.resolve_type(ast, module.id, &[])?;
                }
            }
        }

        // Phase 3: resolve bodies (functions, components, stylesheets)
        for module in modules {
            for ast in &module.declarations {
                self.resolve_body(ast, module.id)?;
            }
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
    fn hoist(
        &self,
        ast: &ASTDeclaration,
        file: FileId,
        should_register: Option<&String>,
    ) -> Result<()> {
        let declarationid = match &ast.kind {
            ASTDeclarationKind::StyleSheet { name, args, .. } => {
                self.hoist_stylesheet(file, &name.identifier, args, ast.visibility)
            }
            ASTDeclarationKind::Alias { name, target } => {
                let alias_symbol = self.intern_name(&name.identifier);
                self.intern_name(&target.identifier);
                self.create_empty_alias(alias_symbol, file, ast.visibility)
            }
            ASTDeclarationKind::ObjectDeclaration { name, fields } => {
                self.create_empty_object(file, name, fields, ast.visibility)
            }

            ASTDeclarationKind::FuncDeclaration { name, args, .. } => {
                self.hoist_function(file, name, args, ast.visibility)?
            }
            ASTDeclarationKind::ComponentDeclaration { name, members, .. } => {
                self.hoist_component(file, name, members, ast.visibility)?
            }
            ASTDeclarationKind::Import(_) => {
                return Ok(());
                //modules loader already solved so
            }
        };
        if let Some(name) = should_register {
            self.lang_items.register(name, declarationid);
        }

        Ok(())
    }

    /// Resolves type-level definitions (object fields, alias targets, import aliases),
    /// run before body resolution so cross-module type dependencies are available.
    fn resolve_type(
        &mut self,
        ast: &ASTDeclaration,
        file: FileId,
        submodules: &[FileId],
    ) -> Result<()> {
        match &ast.kind {
            ASTDeclarationKind::ObjectDeclaration { name, fields, .. } => {
                self.resolve_object(name, fields)
            }
            ASTDeclarationKind::Alias { name, target } => self.resolve_alias(name, target),
            ASTDeclarationKind::Import(import) => {
                for usage in &import.usages {
                    let content_symbol = self.intern_name(&usage.content_name);
                    let (decl, orig_ty) =
                        self.find_declaration_in_files(&content_symbol, submodules, ast.span)?;
                    let alias_symbol = match &usage.alias {
                        Some(alias) => self.intern_name(alias),
                        None => content_symbol,
                    };
                    self.get_file_mut(file).declarations.register_import_alias(
                        alias_symbol,
                        decl.file_id,
                        decl.local_id,
                        orig_ty,
                    );
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Resolves bodies (functions, components, stylesheets).
    fn resolve_body(&mut self, ast: &ASTDeclaration, file: FileId) -> Result<()> {
        match &ast.kind {
            ASTDeclarationKind::ObjectDeclaration { name, .. } => {
                let symbol = self.intern_name(&name.identifier);
                let (decl, declty) = self.find_declaration_by_name(&symbol, ast.span)?;
                self.get_file_mut(file)
                    .create_declaration(HirDeclaration::new_object(decl, declty, ast.span));
                Ok(())
            }
            ASTDeclarationKind::Alias { name, .. } => {
                let alias_name = self.intern_name(&name.identifier);
                let (decl, ty) = self.find_declaration_by_name(&alias_name, ast.span)?;
                self.get_file_mut(file)
                    .create_declaration(HirDeclaration::new_alias(decl, ty, ast.span));
                Ok(())
            }
            ASTDeclarationKind::FuncDeclaration {
                name,
                args,
                body,
                return_type,
            } => self.resolve_function(file, name, args, return_type, body, &ast.span),
            ASTDeclarationKind::ComponentDeclaration { members, name } => {
                self.resolve_component_declaration(file, members, name, ast.span)
            }
            ASTDeclarationKind::StyleSheet {
                name,
                args,
                usages,
                body,
            } => self.resolve_stylesheet(file, name, args, usages, body, ast.span),
            _ => Ok(()),
        }
    }
}
