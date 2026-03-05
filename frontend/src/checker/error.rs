use common::ast::Span;
use middleend::hir::{DeclarationId, VariableId, types::HirType};

#[derive(Debug)]
pub enum IncompatibleComponentReason {
    DifferentPropAmount { rhs: usize, lhs: usize },
}

#[derive(Debug)]
pub struct TypeError {
    pub kind: TypeErrorKind,
    pub span: Span,
}

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
    InvalidFuncallArgLength {
        expected_length: usize,
        received_length: usize,
    },
    InvalidFunctionCallTarget {
        declaration: DeclarationId,
        received: HirType,
    },
    NotARef(VariableId, HirType),
    Unrecognized,
}

impl std::fmt::Display for TypeError {
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
            TypeErrorKind::NotARef(v, ty) => format!(
                "Variable with id {v:?} has got type {ty:?} instead was expecting to be an object"
            ),
            TypeErrorKind::Unrecognized => {
                "Type checker could not resolve the requested symbol/type".to_string()
            }
        };
        write!(f, "{out}")
    }
}

impl std::error::Error for TypeError {}
