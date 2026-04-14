use crate::hir::{definitions::HirExpression, types::HirType};

use common::ast::{ComponentExpression, Span};

#[derive(Debug)]
#[warn(unused)]
pub struct HIRError {
    pub kind: HIRErrorKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum HIRErrorKind {
    TypeNotRecognized(String),
    NameNotRecognized(String),
    NameAlreadyDefined(String),
    InvalidBinaryExpression {
        lhs: Box<HirExpression>,
        rhs: Box<HirExpression>,
    },
    MissingProperty {
        prop_names: Vec<String>,
    },
    PropertyNotRecognized {
        prop_names: Vec<String>,
    },
    PropertyNotVisible {
        prop_name: String,
    },
    InvalidChild {
        child: Box<ComponentExpression>,
    },
    InvalidType {
        ty: String,
        reason: InvalidTypeReason,
    },
    RecursiveType {
        ty: String,
    },
    NotAFunction(String, HirType),
    InvalidFuncallArgLength {
        func_name: String,
        expected_length: usize,
        received_length: usize,
    },
}

impl std::fmt::Display for HIRError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match &self.kind {
            HIRErrorKind::InvalidFuncallArgLength {
                func_name,
                expected_length,
                received_length,
            } => {
                format!(
                    "Function '{func_name}' expected to receive {expected_length} arguments, instead got {received_length} arguments"
                )
            }
            HIRErrorKind::NotAFunction(name, ty) => {
                format!(
                    "The value with name '{name}' is being used as a function, but its type is {ty:?}"
                )
            }
            HIRErrorKind::NameNotRecognized(name) => {
                format!(
                    "The name '{name}' is not recognized. Check if it exists or you wrote some typo"
                )
            }
            HIRErrorKind::TypeNotRecognized(name) => {
                format!("Type with name '{name}' is was not defined previously")
            }
            HIRErrorKind::InvalidBinaryExpression { .. } => "Invalid binary expression".to_string(),
            HIRErrorKind::PropertyNotVisible { prop_name } => {
                format!("Property with name '{prop_name}' is not visible")
            }
            HIRErrorKind::InvalidChild { .. } => {
                "Invalid child. Component is not expecting children".to_string()
            }
            HIRErrorKind::InvalidType { ty, reason } => {
                format!("Invalid type '{ty}' because it's {reason}")
            }
            HIRErrorKind::NameAlreadyDefined(name) => {
                format!("The name '{name}' was already defined before. Use a different name")
            }
            HIRErrorKind::MissingProperty { prop_names } => {
                let names = prop_names
                    .iter()
                    .map(|v| format!("'{v}'"))
                    .collect::<Vec<String>>()
                    .join(", ");
                format!("Property(ies) named as {names} is required but wasn't provided")
            }
            HIRErrorKind::PropertyNotRecognized { prop_names } => {
                let names = prop_names
                    .iter()
                    .map(|v| format!("'{v}'"))
                    .collect::<Vec<String>>()
                    .join(", ");
                format!("Property(ies) named as {names} are not recognized for this object")
            }
            HIRErrorKind::RecursiveType { ty } => {
                format!("The type named as '{ty}' is recursive at this point")
            }
        };
        write!(f, "{out}")
    }
}

impl std::error::Error for HIRError {}

#[derive(Debug)]
pub enum InvalidTypeReason {
    MissingGeneric,
    IncorrectUsage,
}

impl std::fmt::Display for InvalidTypeReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidTypeReason::MissingGeneric => write!(f, "missing generic type"),
            InvalidTypeReason::IncorrectUsage => write!(f, "is being used incorrectly"),
        }
    }
}
