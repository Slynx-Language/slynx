use crate::model::{HirExpression, HirType};

use common::{
    SymbolPointer,
    ast::{ComponentExpression, Span},
};

/// An error produced during HIR generation or type checking.
///
/// Each error carries a [`HIRErrorKind`] describing what went wrong and a
/// [`Span`] pointing to the relevant source location.
#[derive(Debug)]
#[warn(unused)]
pub struct HIRError {
    /// The specific kind of error that occurred.
    pub kind: HIRErrorKind,
    /// The source location associated with this error.
    pub span: Span,
}

/// All possible error kinds that can occur during HIR generation.
#[derive(Debug)]
pub enum HIRErrorKind {
    /// A type name was used but is not defined in the current scope.
    TypeNotRecognized(SymbolPointer),
    /// An identifier was used but is not defined in the current scope.
    NameNotRecognized(SymbolPointer),
    /// A name was declared more than once in the same scope.
    NameAlreadyDefined(SymbolPointer),
    /// A field access was attempted on a type that does not support it.
    InvalidFieldAccessTarget {
        /// The type that was incorrectly accessed.
        ty: HirType,
    },
    /// A tuple index access was attempted on a non-tuple type.
    InvalidTupleAccessTarget {
        /// The type that was incorrectly accessed.
        ty: HirType,
    },
    /// A tuple was indexed out of bounds.
    InvalidTupleIndex {
        /// The index that was used.
        index: usize,
        /// The actual length of the tuple.
        length: usize,
    },
    /// A binary expression was applied to incompatible operand types.
    InvalidBinaryExpression {
        /// The left-hand side expression.
        lhs: Box<HirExpression>,
        /// The right-hand side expression.
        rhs: Box<HirExpression>,
    },
    /// One or more required object properties were not provided.
    MissingProperty {
        /// The names of the missing properties.
        prop_names: Vec<SymbolPointer>,
    },
    /// One or more provided property names do not exist on the target type.
    PropertyNotRecognized {
        /// The names of the unrecognized properties.
        prop_names: Vec<SymbolPointer>,
    },
    /// A property was accessed that exists but is not visible from the current context.
    PropertyNotVisible {
        /// The name of the non-visible property.
        prop_name: SymbolPointer,
    },
    /// A component child expression is not valid in this context.
    InvalidChild {
        /// The invalid child expression.
        child: Box<ComponentExpression>,
    },
    /// A type was used in an invalid way (e.g., missing generics or incorrect usage).
    InvalidType {
        /// The type symbol that was used incorrectly.
        ty: SymbolPointer,
        /// The reason the type usage is invalid.
        reason: InvalidTypeReason,
    },
    /// A type definition is recursive without indirection, which is not allowed.
    RecursiveType {
        /// The type symbol that is recursive.
        ty: SymbolPointer,
    },
    /// A call was made to a value that is not a function.
    NotAFunction(SymbolPointer, HirType),
    /// A function was called with the wrong number of arguments.
    InvalidFuncallArgLength {
        /// The name of the function that was called.
        func_name: SymbolPointer,
        /// The number of arguments the function expects.
        expected_length: usize,
        /// The number of arguments that were provided.
        received_length: usize,
    },
}

impl HIRError {
    /// Creates a [`HIRErrorKind::RecursiveType`] error for the given type symbol.
    pub fn recursive(ty: SymbolPointer, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::RecursiveType { ty },
            span,
        }
    }
    /// Creates a [`HIRErrorKind::TypeNotRecognized`] error for the given type name.
    pub fn type_unrecognized(name: SymbolPointer, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::TypeNotRecognized(name),
            span,
        }
    }
    /// Creates a [`HIRErrorKind::NameNotRecognized`] error for the given identifier.
    pub fn name_unrecognized(name: SymbolPointer, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::NameNotRecognized(name),
            span,
        }
    }
    /// Creates a [`HIRErrorKind::PropertyNotVisible`] error for the given property name.
    pub fn not_visible_property(name: SymbolPointer, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::PropertyNotVisible { prop_name: name },
            span,
        }
    }
    /// Creates a [`HIRErrorKind::InvalidType`] error for the given type symbol and reason.
    pub fn invalid_type(name: SymbolPointer, reason: InvalidTypeReason, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::InvalidType { ty: name, reason },
            span,
        }
    }
    /// Creates a [`HIRErrorKind::MissingProperty`] error listing the missing property names.
    pub fn missing_properties(names: Vec<SymbolPointer>, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::MissingProperty { prop_names: names },
            span,
        }
    }
    /// Creates a [`HIRErrorKind::PropertyNotRecognized`] error listing the unrecognized property names.
    pub fn property_unrecognized(names: Vec<SymbolPointer>, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::PropertyNotRecognized { prop_names: names },
            span,
        }
    }
    /// Creates a [`HIRErrorKind::InvalidFuncallArgLength`] error for a call with the wrong number of arguments.
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
    /// Creates a [`HIRErrorKind::NotAFunction`] error for a call to a non-function value.
    pub fn not_a_func(func: SymbolPointer, ty: HirType, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::NotAFunction(func, ty),
            span,
        }
    }
    /// Creates a [`HIRErrorKind::NameAlreadyDefined`] error for a duplicate declaration.
    pub fn already_defined(name: SymbolPointer, span: Span) -> Self {
        Self {
            kind: HIRErrorKind::NameAlreadyDefined(name),
            span,
        }
    }
}

/// The reason a type was used in an invalid way.
#[derive(Debug)]
pub enum InvalidTypeReason {
    /// The type requires a generic parameter that was not provided.
    MissingGeneric,
    /// The type is being used in a context where it is not valid.
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
