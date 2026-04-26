use crate::hir::model::{HirExpression, HirType};

use common::{
    SymbolPointer,
    ast::{ComponentExpression, Span},
};

#[derive(Debug)]
#[warn(unused)]
pub struct HIRError {
    pub kind: HIRErrorKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum HIRErrorKind {
    TypeNotRecognized(SymbolPointer),
    NameNotRecognized(SymbolPointer),
    NameAlreadyDefined(SymbolPointer),
    InvalidFieldAccessTarget {
        ty: HirType,
    },
    InvalidTupleAccessTarget {
        ty: HirType,
    },
    InvalidTupleIndex {
        index: usize,
        length: usize,
    },
    InvalidBinaryExpression {
        lhs: Box<HirExpression>,
        rhs: Box<HirExpression>,
    },
    MissingProperty {
        prop_names: Vec<SymbolPointer>,
    },
    PropertyNotRecognized {
        prop_names: Vec<SymbolPointer>,
    },
    PropertyNotVisible {
        prop_name: SymbolPointer,
    },
    InvalidChild {
        child: Box<ComponentExpression>,
    },
    InvalidType {
        ty: SymbolPointer,
        reason: InvalidTypeReason,
    },
    RecursiveType {
        ty: SymbolPointer,
    },
    NotAFunction(SymbolPointer, HirType),
    InvalidFuncallArgLength {
        func_name: SymbolPointer,
        expected_length: usize,
        received_length: usize,
    },
}

impl HIRError {
    pub fn recursive(ty: SymbolPointer, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::RecursiveType { ty },
            span,
        }
    }
    pub fn type_unrecognized(name: SymbolPointer, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::TypeNotRecognized(name),
            span,
        }
    }
    pub fn name_unrecognized(name: SymbolPointer, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::NameNotRecognized(name),
            span,
        }
    }
    pub fn not_visible_property(name: SymbolPointer, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::PropertyNotVisible { prop_name: name },
            span,
        }
    }
    pub fn invalid_type(name: SymbolPointer, reason: InvalidTypeReason, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::InvalidType {
                ty: name,
                reason: reason,
            },
            span,
        }
    }
    pub fn missing_properties(names: Vec<SymbolPointer>, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::MissingProperty { prop_names: names },
            span,
        }
    }
    pub fn property_unrecognized(names: Vec<SymbolPointer>, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::PropertyNotRecognized { prop_names: names },
            span,
        }
    }
    pub fn invalid_funcall_arg_length(
        func: SymbolPointer,
        expected: usize,
        received: usize,
        span: Span,
    ) -> Self {
        Self {
            kind: HIRErrorKind::InvalidFuncallArgLength {
                func_name: func,
                expected_length: expected,
                received_length: received,
            },
            span,
        }
    }
    pub fn not_a_func(func: SymbolPointer, ty: HirType, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::NotAFunction(func, ty),
            span,
        }
    }
    pub fn already_defined(name: SymbolPointer, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::NameAlreadyDefined(name),
            span,
        }
    }
}

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
