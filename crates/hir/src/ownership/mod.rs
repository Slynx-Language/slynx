//! Ownership analysis for the HIR.
//!
//! This module implements move semantics and borrow checking, similar to
//! Rust's ownership system but simplified for Slynx's needs.
//!
//! # Overview
//!
//! The ownership pass runs after HIR construction and validates:
//! - **Move semantics**: Non-Copy values are moved on use
//! - **Borrow checking**: References must not outlive the value they borrow
//! - **Use-after-move**: Using a moved value is an error
//! - **Borrow conflicts**: Conflicting borrows are errors
//!
//! # Architecture
//!
//! ```text
//! HIR Builder (types + names + basic validation)
//!       ↓
//! Ownership Pass (move/borrow analysis)
//!       ↓
//! Codegen (reads ownership info)
//! ```
//!
//! The ownership pass produces an [`OwnershipAnalysis`] that stores:
//! - Per-function ownership state
//! - Per-expression use kind (Move, Copy, Borrow, BorrowMut)
//! - A pool of [`HirPlace`]s constructed during analysis
//!
//! # Copy Types
//!
//! For now, the following types are considered Copy:
//! - `int`, `float`, `bool`, `str`
//!
//! All other types (structs, tuples, arrays, etc.) are Move-only.

use crate::{HirType, SlynxHir, VariableId};

mod analysis;
mod place;
mod state;
pub use analysis::OwnershipAnalysis;
pub use state::*;

/// How an expression uses a place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionUse {
    /// Reading the value (copies if Copy type, moves if not).
    Read,
    /// Moving the value out of the place.
    Move,
    /// Taking an immutable reference to the place.
    Borrow,
    /// Taking a mutable reference to the place.
    BorrowMut,
}

/// An error produced during ownership analysis.
#[derive(Debug)]
pub struct OwnershipError {
    pub kind: OwnershipErrorKind,
    pub span: common::Span,
}

#[derive(Debug)]
pub enum OwnershipErrorKind {
    /// Using a variable after it has been moved.
    UseAfterMove { variable: VariableId },
    /// Borrowing a variable that is already mutably borrowed.
    ConflictingBorrow {
        variable: VariableId,
        existing_borrow: BorrowKind,
        new_borrow: BorrowKind,
    },
    /// Moving a variable that is currently borrowed.
    MoveWhileBorrowed { variable: VariableId },
    /// Trying to borrow mutably an immutable variable.
    MutablyBorrowImmutable { variable: VariableId },
}

/// Check if a type is Copy (can be implicitly duplicated).
pub fn is_copy_type(hir: &SlynxHir, ty: common::pool::DedupPoolId<HirType>) -> bool {
    matches!(
        hir.types_module[ty],
        HirType::Int | HirType::Float | HirType::Bool | HirType::Str
    )
}
