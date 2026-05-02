//! Type checking errors for the Slynx type checker.
//!
//! This module defines the error types used during type checking, including
//! type mismatch errors, cyclic type errors, and component-related errors.

use common::ast::Span;
use slynx_hir::{DeclarationId, VariableId, model::HirType};

/// Represents the reason for an incompatible component error.
///
/// This enum is used to specify the specific reason why two components are
/// incompatible, such as having a different number of properties.
#[derive(Debug)]
pub enum IncompatibleComponentReason {
    DifferentPropAmount { rhs: usize, lhs: usize },
}

/// Represents a type error that occurred during type checking.
///
/// This struct holds the type error kind and the span where the error occurred.
#[derive(Debug)]
pub struct TypeError {
    pub kind: TypeErrorKind,
    pub span: Span,
}

/// Represents the kind of type error that occurred.
///
/// This enum is used to specify the specific type error that occurred, such as
/// a type mismatch or a cyclic type reference.
#[derive(Debug)]
pub enum TypeErrorKind {
    CannotCastType {
        expected: HirType,
        received: HirType,
    },
    CiclicType {
        ty: HirType,
    },
    IncompatibleComponent {
        reason: IncompatibleComponentReason,
    },
    IncompatibleTypes {
        expected: HirType,
        received: HirType,
    },
    InvalidTupleAccessTarget {
        received: HirType,
    },
    InvalidTupleIndex {
        index: usize,
        length: usize,
    },
    InvalidFuncallArgLength {
        expected_length: usize,
        received_length: usize,
    },
    InvalidFunctionCallTarget {
        declaration: DeclarationId,
        received: HirType,
    },
    MissingReturnValue {
        expected: HirType,
    },
    NotARef(VariableId, HirType),
    NotAStruct(HirType),
    Unrecognized,
}

impl std::fmt::Display for TypeError {
    /// Formats the type error as a string.
    ///
    /// This implementation uses the error kind to generate a human-readable error
    /// message, including the expected and received types for type mismatch errors.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match &self.kind {
            TypeErrorKind::CannotCastType { expected, received } => {
                format!("Could not cast type '{expected:?}' into '{received:?}'")
            }
            TypeErrorKind::CiclicType { ty } => {
                format!("The type '{ty:?}' is cyclic and cannot exist without recursion")
            }
            TypeErrorKind::IncompatibleComponent { reason } => {
                format!("The component is incompatible because of '{reason:?}'")
            }
            TypeErrorKind::IncompatibleTypes { expected, received } => format!(
                "Incompatible types. Was expecting to receive type '{expected:?}' instead got type '{received:?}'"
            ),
            TypeErrorKind::InvalidTupleAccessTarget { received } => {
                format!("Type '{received:?}' does not support tuple-style access")
            }
            TypeErrorKind::InvalidTupleIndex { index, length } => {
                format!(
                    "Tuple index {index} is out of bounds. The tuple only exposes {length} fields"
                )
            }
            TypeErrorKind::InvalidFuncallArgLength {
                expected_length,
                received_length,
            } => format!(
                "Invalid function call arg length. Expected {expected_length} args but received {received_length}"
            ),
            TypeErrorKind::InvalidFunctionCallTarget {
                declaration,
                received,
            } => format!(
                "Invalid function call target at declaration {declaration:?}. Expected function type but received {received:?}"
            ),
            TypeErrorKind::MissingReturnValue { expected } => {
                format!("Function is missing a return value of type '{expected:?}'")
            }
            TypeErrorKind::NotARef(v, ty) => format!(
                "Variable with id {v:?} has got type {ty:?} instead was expecting to be an object"
            ),
            TypeErrorKind::NotAStruct(t) => {
                format!("Using type {t:?} as a Struct, even though it isn't")
            }
            TypeErrorKind::Unrecognized => {
                "Type checker could not resolve the requested symbol/type".to_string()
            }
        };
        write!(f, "{out}")
    }
}

impl std::error::Error for TypeError {}
