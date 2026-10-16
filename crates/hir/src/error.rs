use std::backtrace::Backtrace;

use crate::{ComponentId, SymbolPointer, model::HirExpression, term::TermId};

use common::Span;
use module_loader::FileId;

/// A temporary component key used during signature resolution.
/// (FileId, SymbolPointer) identifies a component before its ComponentId is created.
pub type ComponentKey = (FileId, SymbolPointer);

/// An error produced during HIR generation or type checking.
///
/// Each error carries a [`HIRErrorKind`] describing what went wrong, the
/// [`FileId`] of the file the offending code was written in, and a [`Span`]
/// pointing to the relevant source location. Spans are offsets into a single
/// file, so the file is what makes the span resolvable.
#[derive(Debug)]
pub struct HIRError {
    /// The specific kind of error that occurred.
    pub kind: HIRErrorKind,
    /// The file the offending code was written in.
    pub file: FileId,
    /// The source location associated with this error.
    pub span: Span,
    pub backtrace: Backtrace,
}

#[derive(Debug)]
pub enum InvalidWriteReason {
    ImmutableVariable(SymbolPointer),
    ExpressionNotAssignable,
    ReferenceImmutable,
}

#[derive(Debug)]
pub enum NotMutableReason {
    ImmutableVariable(SymbolPointer),
    ExpressionNotAssignable,
}

#[derive(Debug, Clone)]
pub enum MissingFeature {
    ComplexTypeForBounds,
    GenericInterfaces,
}

#[derive(Debug, Clone)]
pub enum ExpectedTypeUsage {
    InterfaceType,
}

/// All possible error kinds that can occur during HIR generation.
#[derive(Debug)]
pub enum HIRErrorKind {
    UnexpectedTypeUsage {
        actual: TermId,
        expected_usage: ExpectedTypeUsage,
    },

    UnimplementedFeature(MissingFeature),
    ///Error that occurs when a type is used like an enum, but isn't
    InvalidEnumUsage(TermId),

    MissingExtensionMethod {
        method: SymbolPointer,
    },
    MethodNotFound(SymbolPointer, TermId),
    StaticMethodNotFound(SymbolPointer, TermId),
    /// A concrete type implemented the same interface more than once (coherence).
    DuplicateInterfaceImplementation {
        /// The concrete type that was implemented twice.
        ty: TermId,
        /// The interface that was implemented more than once.
        interface: TermId,
    },
    /// A deferred interface call could not be discharged: the receiver's
    /// concrete type has no implementation of the method.
    UnresolvedInterfaceCall {
        /// The name of the interface method that could not be dispatched.
        method: SymbolPointer,
        /// The concrete receiver type that provides no implementation.
        receiver: TermId,
    },
    InvalidTypeAccess,
    ExpressionNotMutable(NotMutableReason),
    InvalidDeref,
    ArrayLengthMismatch {
        expected: usize,
        actual: usize,
    },
    MissingReturn,

    /// A raw-valued enum variant (`Name = <expr>`) had a non-integer value.
    EnumVariantNotAnInt(SymbolPointer),

    /// A `matches` expression's left-hand side is not an enum value.
    MatchesOnNonEnum(TermId),

    /// The right-hand side of a `matches` expression is not a valid pattern
    /// (a bare variant name or a variant call).
    InvalidPattern,

    /// A name was referenced as an enum variant, but no reachable enum
    /// declares a variant with that name.
    VariantNotRecognized(SymbolPointer),

    /// An enum was declared with an unsupported representation (only `int` is
    /// supported for raw/raw-valued enums).
    InvalidEnumRepresentation(SymbolPointer),

    UnexpectedType {
        expected: TermId,
        received: TermId,
    },
    ///Invalid indexing error occurs when the expression being indexed cannot be indexed. So 5[12] cannot be indexed, which then gives this error
    InvalidIndexing(TermId),
    ///Couldnt infer is an error when the type of something could not be inferred
    CouldntInfer,
    NotAComponent(TermId),
    ComponentPropertyMissingType,
    ComponentNotFound(SymbolPointer),

    InvalidWrite(InvalidWriteReason),

    ///An invalid field access occurs when some invalid expression is used to access a field. For example 'struct."j"' this is invalid because we cannot index things with strings since the struct layout is not runtime based
    InvalidFieldAccess,
    /// A type name was used but is not defined in the current scope.
    TypeNotRecognized(SymbolPointer),
    /// An identifier was used but is not defined in the current scope.
    NameNotRecognized(SymbolPointer),
    /// A name was declared more than once in the same scope.
    NameAlreadyDefined(SymbolPointer),
    /// A field access was attempted on a type that does not support it.
    InvalidFieldAccessTarget {
        /// The type that was incorrectly accessed.
        ty: TermId,
    },
    /// A tuple index access was attempted on a non-tuple type.
    InvalidTupleAccessTarget {
        /// The type that was incorrectly accessed.
        ty: TermId,
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
        ty: TermId,
    },
    /// A property was accessed that exists but is not visible from the current context.
    PropertyNotVisible {
        /// The name of the non-visible property.
        prop_name: SymbolPointer,
    },
    /// A method was called that exists but is not visible from the calling file.
    MethodNotVisible {
        /// The name of the non-visible method.
        method_name: SymbolPointer,
    },
    /// A component child expression is not valid in this context.
    InvalidChild,
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
        ty: TermId,
    },
    /// A call was made to a value that is not a function.
    NotAFunction(SymbolPointer, TermId),
    /// A function was called with the wrong number of arguments.
    InvalidFuncallArgLength {
        /// The name of the function that was called.
        func_name: SymbolPointer,
        /// The number of arguments the function expects.
        expected_length: usize,
        /// The number of arguments that were provided.
        received_length: usize,
    },
    ///An error that occurs when an style event is used on a stylesheet. For example:
    ///```slx
    /// stylesheet Example(){
    ///     styles {
    ///       blob(){/*wtf is blob?*/}
    ///     }
    /// }```
    InvalidStyleEvent {
        ///The name of the event that is invalid
        name: SymbolPointer,
    },
    ///An error that occurs when a style definition is invalid, such as `fg: red`, where `fg` does not exist
    InvalidStyleDefinition {
        ///The name of the definition that is invalid
        name: SymbolPointer,
    },
    /// A declaration name resolves to multiple files (ambiguous import).
    AmbiguousDeclaration {
        /// The name that is ambiguous.
        name: SymbolPointer,
        /// The first file where the declaration was found.
        first: FileId,
        /// The second file where the declaration was found.
        second: FileId,
    },
    /// An intrinsic was referenced but is not registered (e.g. missing std library).
    IntrinsicNotRegistered {
        /// The name of the intrinsic that was not found.
        name: SymbolPointer,
    },
    /// A chain of component signatures is cyclic.
    CyclicComponentSignature {
        /// The component that triggered the cycle.
        component: SymbolPointer,
        /// The chain of (FileId, SymbolPointer) pairs forming the cycle.
        chain: Vec<ComponentKey>,
    },
    /// A component body resolution re-entered itself (body → ... → body cycle).
    CyclicComponentBody {
        /// The component whose body caused the cycle.
        component: ComponentId,
    },
    /// A generic function was called with the wrong number of explicit type
    /// arguments (e.g. `compare<int>(a, b)` on a `func compare<T, U>`).
    GenericArityMismatch {
        /// The name of the generic function being instantiated.
        func: SymbolPointer,
        /// The number of type parameters the function declares.
        declared: usize,
        /// The number of type arguments supplied at the call site.
        supplied: usize,
    },
    /// Monomorphizing a generic function would never terminate because each
    /// instantiation requests a new, larger instantiation (e.g. a function
    /// whose generic argument grows without bound).
    CyclicMonomorphization {
        /// The name of the function whose instantiation cycles.
        func: SymbolPointer,
        /// The generic type arguments that keep growing.
        args: Vec<TermId>,
    },
    /// A construct that is not yet supported was encountered. The named
    /// declaration cannot be processed by the current implementation.
    NotImplemented(SymbolPointer),
}

impl HIRError {
    pub fn new(file: FileId, err: impl Into<HIRErrorKind>, span: Span) -> Self {
        Self {
            kind: err.into(),
            file,
            span,
            backtrace: Backtrace::capture(),
        }
    }

    pub fn missing_interface_method(file: FileId, name: SymbolPointer, span: Span) -> Self {
        Self::new(
            file,
            HIRErrorKind::MissingExtensionMethod { method: name },
            span,
        )
    }

    pub fn expected_interface_type(file: FileId, found: TermId, span: Span) -> Self {
        Self::new(
            file,
            HIRErrorKind::UnexpectedTypeUsage {
                expected_usage: ExpectedTypeUsage::InterfaceType,
                actual: found,
            },
            span,
        )
    }

    pub fn unimplemented(file: FileId, feature: MissingFeature, span: Span) -> Self {
        Self::new(file, HIRErrorKind::UnimplementedFeature(feature), span)
    }

    pub fn not_an_enum(file: FileId, ty: TermId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::InvalidEnumUsage(ty), span)
    }

    pub fn method_not_found(file: FileId, name: SymbolPointer, target: TermId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::MethodNotFound(name, target), span)
    }

    pub fn static_method_not_found(
        file: FileId,
        name: SymbolPointer,
        target: TermId,
        span: Span,
    ) -> Self {
        Self::new(file, HIRErrorKind::StaticMethodNotFound(name, target), span)
    }

    pub fn duplicate_interface_implementation(
        file: FileId,
        ty: TermId,
        interface: TermId,
        span: Span,
    ) -> Self {
        Self::new(
            file,
            HIRErrorKind::DuplicateInterfaceImplementation { ty, interface },
            span,
        )
    }

    ///A deferred interface call on a receiver that no extension implements.
    pub fn unresolved_interface_call(
        file: FileId,
        method: SymbolPointer,
        receiver: TermId,
        span: Span,
    ) -> Self {
        Self::new(
            file,
            HIRErrorKind::UnresolvedInterfaceCall { method, receiver },
            span,
        )
    }

    pub fn invalid_type_access(file: FileId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::InvalidTypeAccess, span)
    }

    pub fn expression_not_mutable(file: FileId, reason: NotMutableReason, span: Span) -> Self {
        Self::new(file, HIRErrorKind::ExpressionNotMutable(reason), span)
    }

    pub fn invalid_deref(file: FileId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::InvalidDeref, span)
    }
    pub fn array_length_mismatch(file: FileId, expected: usize, actual: usize, span: Span) -> Self {
        Self::new(
            file,
            HIRErrorKind::ArrayLengthMismatch { expected, actual },
            span,
        )
    }
    pub fn missing_return(file: FileId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::MissingReturn, span)
    }

    pub fn enum_variant_must_be_an_int(file: FileId, name: SymbolPointer, span: Span) -> Self {
        Self::new(file, HIRErrorKind::EnumVariantNotAnInt(name), span)
    }

    pub fn matches_on_non_enum(file: FileId, ty: TermId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::MatchesOnNonEnum(ty), span)
    }

    pub fn invalid_pattern(file: FileId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::InvalidPattern, span)
    }

    pub fn variant_unrecognized(file: FileId, name: SymbolPointer, span: Span) -> Self {
        Self::new(file, HIRErrorKind::VariantNotRecognized(name), span)
    }

    pub fn invalid_enum_representation(file: FileId, name: SymbolPointer, span: Span) -> Self {
        Self::new(file, HIRErrorKind::InvalidEnumRepresentation(name), span)
    }

    pub fn invalid_indexing(file: FileId, expr_type: TermId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::InvalidIndexing(expr_type), span)
    }

    pub fn unexpected_type(file: FileId, received: TermId, expected: TermId, span: Span) -> Self {
        Self::new(
            file,
            HIRErrorKind::UnexpectedType { received, expected },
            span,
        )
    }
    pub fn couldnt_infer(file: FileId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::CouldntInfer, span)
    }
    pub fn component_not_found(file: FileId, name: SymbolPointer, span: Span) -> Self {
        Self::new(file, HIRErrorKind::ComponentNotFound(name), span)
    }

    pub fn component_missing_prop_type(file: FileId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::ComponentPropertyMissingType, span)
    }

    pub fn invalid_ref_write(file: FileId, span: Span) -> Self {
        Self::new(
            file,
            HIRErrorKind::InvalidWrite(InvalidWriteReason::ReferenceImmutable),
            span,
        )
    }

    pub fn invalid_variable_write(file: FileId, name: SymbolPointer, span: Span) -> Self {
        Self::new(
            file,
            HIRErrorKind::InvalidWrite(InvalidWriteReason::ImmutableVariable(name)),
            span,
        )
    }
    pub fn invalid_expr_write(file: FileId, span: Span) -> Self {
        Self::new(
            file,
            HIRErrorKind::InvalidWrite(InvalidWriteReason::ExpressionNotAssignable),
            span,
        )
    }
    pub fn invalid_field_access(file: FileId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::InvalidFieldAccess, span)
    }

    /// Creates a new [`HIRErrorKind::InvalidTupleIndex`] error for a tuple index out of bounds,
    /// where the given `span` is the span on the code that generated it.
    pub fn invalid_tuple_index(file: FileId, index: usize, max_index: usize, span: Span) -> Self {
        Self::new(
            file,
            HIRErrorKind::InvalidTupleIndex {
                index,
                length: max_index,
            },
            span,
        )
    }
    /// Creates a new [`HIRErrorKind::InvalidTupleAccessTarget`] error for accessing a non-tuple
    /// type as a tuple, where the given `span` is the span on the code that generated it.
    pub fn invalid_tuple_target(file: FileId, target: TermId, span: Span) -> Self {
        Self::new(
            file,
            HIRErrorKind::InvalidTupleAccessTarget { ty: target },
            span,
        )
    }
    pub fn invalid_style_definition(file: FileId, name: SymbolPointer, span: Span) -> Self {
        Self::new(file, HIRErrorKind::InvalidStyleDefinition { name }, span)
    }
    /// Creates a new [`HIRErrorKind::InvalidStyleEvent`] error, where the `name` is the invalid style event
    pub fn invalid_style_event(file: FileId, name: SymbolPointer, span: Span) -> Self {
        Self::new(file, HIRErrorKind::InvalidStyleEvent { name }, span)
    }

    /// Creates a [`HIRErrorKind::RecursiveType`] error for the given type symbol.
    pub fn recursive(file: FileId, ty: TermId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::RecursiveType { ty }, span)
    }
    /// Creates a [`HIRErrorKind::TypeNotRecognized`] error for the given type name.
    pub fn type_unrecognized(file: FileId, name: SymbolPointer, span: Span) -> Self {
        Self::new(file, HIRErrorKind::TypeNotRecognized(name), span)
    }
    /// Creates a [`HIRErrorKind::NameNotRecognized`] error for the given identifier.
    pub fn name_unrecognized(file: FileId, name: SymbolPointer, span: Span) -> Self {
        Self::new(file, HIRErrorKind::NameNotRecognized(name), span)
    }
    /// Creates a [`HIRErrorKind::PropertyNotVisible`] error for the given property name.
    pub fn not_visible_property(file: FileId, name: SymbolPointer, span: Span) -> Self {
        Self::new(
            file,
            HIRErrorKind::PropertyNotVisible { prop_name: name },
            span,
        )
    }
    /// Creates a [`HIRErrorKind::MethodNotVisible`] error for the given method name.
    pub fn not_visible_method(file: FileId, name: SymbolPointer, span: Span) -> Self {
        Self::new(
            file,
            HIRErrorKind::MethodNotVisible { method_name: name },
            span,
        )
    }
    /// Creates a [`HIRErrorKind::InvalidFieldAccessTarget`] for accessing a non-struct type as a struct.
    pub fn not_a_struct(file: FileId, ty: TermId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::InvalidFieldAccessTarget { ty }, span)
    }
    /// Creates a [`HIRErrorKind::InvalidFieldAccessTarget`] for accessing a non-struct type as a struct.
    pub fn not_a_component(file: FileId, name: TermId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::NotAComponent(name), span)
    }
    /// Creates a [`HIRErrorKind::InvalidTupleAccessTarget`] for accessing a non-tuple type as a tuple.
    pub fn not_a_tuple(file: FileId, ty: TermId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::InvalidTupleAccessTarget { ty }, span)
    }
    /// Creates a [`HIRErrorKind::InvalidType`] error for the given type symbol and reason.
    pub fn invalid_type(
        file: FileId,
        name: SymbolPointer,
        reason: InvalidTypeReason,
        span: Span,
    ) -> Self {
        Self::new(file, HIRErrorKind::InvalidType { ty: name, reason }, span)
    }
    /// Creates a [`HIRErrorKind::MissingProperty`] error listing the missing property names.
    pub fn missing_properties(file: FileId, names: Vec<SymbolPointer>, span: Span) -> Self {
        Self::new(
            file,
            HIRErrorKind::MissingProperty { prop_names: names },
            span,
        )
    }
    /// Creates a [`HIRErrorKind::PropertyNotRecognized`] error listing the unrecognized property names.
    pub fn property_unrecognized(
        file: FileId,
        ty: TermId,
        names: Vec<SymbolPointer>,
        span: Span,
    ) -> Self {
        Self::new(
            file,
            HIRErrorKind::PropertyNotRecognized {
                prop_names: names,
                ty,
            },
            span,
        )
    }
    /// Creates a [`HIRErrorKind::InvalidFuncallArgLength`] error for a call with the wrong number of arguments.
    pub fn invalid_funcall_arg_length(
        file: FileId,
        func: SymbolPointer,
        expected: usize,
        received: usize,
        span: Span,
    ) -> Self {
        Self::new(
            file,
            HIRErrorKind::InvalidFuncallArgLength {
                func_name: func,
                expected_length: expected,
                received_length: received,
            },
            span,
        )
    }
    /// Creates a [`HIRErrorKind::NotAFunction`] error for a call to a non-function value.
    pub fn not_a_func(file: FileId, func: SymbolPointer, ty: TermId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::NotAFunction(func, ty), span)
    }
    /// Creates a [`HIRErrorKind::NameAlreadyDefined`] error for a duplicate declaration.
    pub fn already_defined(file: FileId, name: SymbolPointer, span: Span) -> Self {
        Self::new(file, HIRErrorKind::NameAlreadyDefined(name), span)
    }
    /// Creates a [`HIRErrorKind::IntrinsicNotRegistered`] error for the given intrinsic name.
    pub fn intrinsic_not_registered(file: FileId, name: SymbolPointer, span: Span) -> Self {
        Self::new(file, HIRErrorKind::IntrinsicNotRegistered { name }, span)
    }
    /// Creates a [`HIRErrorKind::CyclicComponentSignature`] error.
    pub fn cyclic_component_signature(
        file: FileId,
        component: SymbolPointer,
        chain: Vec<ComponentKey>,
        span: Span,
    ) -> Self {
        Self::new(
            file,
            HIRErrorKind::CyclicComponentSignature { component, chain },
            span,
        )
    }

    /// Creates a [`HIRErrorKind::CyclicComponentBody`] error.
    pub fn cyclic_component_body(file: FileId, component: ComponentId, span: Span) -> Self {
        Self::new(file, HIRErrorKind::CyclicComponentBody { component }, span)
    }

    /// Creates a [`HIRErrorKind::GenericArityMismatch`] error.
    pub fn generic_arity_mismatch(
        file: FileId,
        func: SymbolPointer,
        declared: usize,
        supplied: usize,
        span: Span,
    ) -> Self {
        Self::new(
            file,
            HIRErrorKind::GenericArityMismatch {
                func,
                declared,
                supplied,
            },
            span,
        )
    }

    /// Creates a [`HIRErrorKind::CyclicMonomorphization`] error.
    pub fn cyclic_monomorphization(
        file: FileId,
        func: SymbolPointer,
        args: Vec<TermId>,
        span: Span,
    ) -> Self {
        Self::new(
            file,
            HIRErrorKind::CyclicMonomorphization { func, args },
            span,
        )
    }

    /// Creates a [`HIRErrorKind::AmbiguousDeclaration`] error.
    pub fn ambiguous_declaration(
        file: FileId,
        name: SymbolPointer,
        first: FileId,
        second: FileId,
        span: Span,
    ) -> Self {
        Self::new(
            file,
            HIRErrorKind::AmbiguousDeclaration {
                name,
                first,
                second,
            },
            span,
        )
    }

    /// Creates a [`HIRErrorKind::NotImplemented`] error for the given declaration.
    pub fn not_implemented(file: FileId, name: SymbolPointer, span: Span) -> Self {
        Self::new(file, HIRErrorKind::NotImplemented(name), span)
    }
}

impl std::fmt::Display for HIRError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            HIRErrorKind::MissingExtensionMethod { .. } => {
                write!(f, "Method not found for interface",)
            }
            HIRErrorKind::UnexpectedTypeUsage {
                expected_usage,
                actual,
            } => {
                write!(
                    f,
                    "Unexpected type usage: expected {:?}, got {:?}",
                    expected_usage, actual
                )
            }
            HIRErrorKind::UnimplementedFeature(feature) => {
                write!(f, "Unimplemented feature: '{:?}'", feature)
            }
            HIRErrorKind::InvalidEnumUsage(_) => write!(f, "Invalid enum usage"),
            HIRErrorKind::MethodNotFound(_, _) => {
                write!(f, "Method not found")
            }
            HIRErrorKind::StaticMethodNotFound(_, _) => {
                write!(f, "Static method not found")
            }
            HIRErrorKind::DuplicateInterfaceImplementation { .. } => {
                write!(f, "Type implements the same interface more than once")
            }
            HIRErrorKind::UnresolvedInterfaceCall { .. } => {
                write!(f, "No implementation of this interface method was found")
            }
            HIRErrorKind::InvalidTypeAccess => write!(f, "Invalid type access"),
            HIRErrorKind::ExpressionNotMutable(_) => write!(f, "Expression not mutable"),
            HIRErrorKind::InvalidDeref => write!(f, "Invalid deref"),
            HIRErrorKind::ArrayLengthMismatch { expected, actual } => {
                write!(
                    f,
                    "Array length mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            HIRErrorKind::MissingReturn => write!(
                f,
                "This function does not return at all, even though it should"
            ),
            HIRErrorKind::EnumVariantNotAnInt(_) => {
                write!(f, "Valued enum variants must have an integer literal value")
            }
            HIRErrorKind::MatchesOnNonEnum(_) => write!(
                f,
                "The left-hand side of a `matches` expression must be an enum value"
            ),
            HIRErrorKind::InvalidPattern => write!(
                f,
                "The right-hand side of a `matches` expression must be a variant name (`Foo`) or a variant call (`Foo(...)`)"
            ),
            HIRErrorKind::VariantNotRecognized(_) => {
                write!(f, "No reachable enum declares a variant with this name")
            }
            HIRErrorKind::InvalidEnumRepresentation(_) => write!(
                f,
                "Only `int` is supported as an enum representation; raw enums must use `repr: int`"
            ),
            HIRErrorKind::UnexpectedType { .. } => {
                write!(f, "Received mismatched types")
            }

            HIRErrorKind::InvalidIndexing(_) => write!(
                f,
                "The given expression cannot be indexed because it is not an array nor vector"
            ),
            HIRErrorKind::CouldntInfer => write!(f, "Expression type couldn't be inferred"),
            HIRErrorKind::ComponentNotFound(_) => write!(f, "Component not found"),
            HIRErrorKind::NotAComponent(_) => write!(f, "Attempt to use value as a component"),
            HIRErrorKind::ComponentPropertyMissingType => {
                write!(f, "Component property is missing type definition")
            }

            HIRErrorKind::InvalidWrite(InvalidWriteReason::ImmutableVariable(_)) => {
                write!(f, "Attempt to write on an immutable variable")
            }
            HIRErrorKind::InvalidWrite(InvalidWriteReason::ExpressionNotAssignable) => {
                write!(f, "The expression is not assignable")
            }
            HIRErrorKind::InvalidWrite(InvalidWriteReason::ReferenceImmutable) => {
                write!(f, "Reference being written is immutable")
            }
            HIRErrorKind::InvalidFieldAccess => write!(f, "Invalid field access"),
            HIRErrorKind::TypeNotRecognized(_) => write!(f, "Type not recognized"),
            HIRErrorKind::NameNotRecognized(_) => write!(f, "Name not recognized"),
            HIRErrorKind::NameAlreadyDefined(_) => write!(f, "Name already defined"),
            HIRErrorKind::InvalidFieldAccessTarget { .. } => {
                write!(f, "Invalid field access target")
            }
            HIRErrorKind::InvalidTupleAccessTarget { .. } => {
                write!(f, "Invalid tuple access target")
            }
            HIRErrorKind::InvalidTupleIndex { index, length } => {
                write!(f, "Tuple index {index} out of bounds (length: {length})")
            }
            HIRErrorKind::InvalidBinaryExpression { .. } => write!(f, "Invalid binary expression"),
            HIRErrorKind::MissingProperty { .. } => write!(f, "Missing required property"),
            HIRErrorKind::PropertyNotRecognized { .. } => write!(f, "Property not recognized"),
            HIRErrorKind::PropertyNotVisible { .. } => write!(f, "Property not visible"),
            HIRErrorKind::MethodNotVisible { .. } => write!(f, "Method not visible"),
            HIRErrorKind::InvalidChild => write!(f, "Invalid child expression"),
            HIRErrorKind::InvalidType { reason, .. } => write!(f, "Invalid type: {reason}"),
            HIRErrorKind::RecursiveType { .. } => write!(f, "Recursive type definition"),
            HIRErrorKind::NotAFunction(..) => write!(f, "Not a function"),
            HIRErrorKind::InvalidFuncallArgLength {
                expected_length,
                received_length,
                ..
            } => {
                write!(
                    f,
                    "Expected {expected_length} arguments, got {received_length}"
                )
            }
            HIRErrorKind::InvalidStyleEvent { .. } => write!(f, "Invalid style event"),
            HIRErrorKind::InvalidStyleDefinition { .. } => {
                write!(f, "Invalid style definition name")
            }
            HIRErrorKind::AmbiguousDeclaration {
                name: _,
                first,
                second,
            } => {
                write!(
                    f,
                    "Ambiguous declaration found in files {:?} and {:?}",
                    first, second
                )
            }
            HIRErrorKind::IntrinsicNotRegistered { name } => {
                write!(
                    f,
                    "intrinsic '{:?}' is not defined — ensure the standard library is loaded",
                    name
                )
            }
            HIRErrorKind::CyclicComponentSignature {
                component: _,
                chain,
            } => {
                write!(f, "cyclic component signature: ")?;
                for (i, (_, name)) in chain.iter().enumerate() {
                    if i > 0 {
                        write!(f, " → ")?;
                    }
                    write!(f, "{:?}", name)?;
                }
                Ok(())
            }
            HIRErrorKind::CyclicComponentBody { component: _ } => {
                write!(f, "cyclic component body resolution")
            }
            HIRErrorKind::GenericArityMismatch {
                func,
                declared,
                supplied,
            } => {
                write!(
                    f,
                    "generic function '{func:?}' expects {declared} type argument(s), got {supplied}"
                )
            }
            HIRErrorKind::CyclicMonomorphization { func, .. } => {
                write!(
                    f,
                    "monomorphization of generic function '{func:?}' does not terminate"
                )
            }
            HIRErrorKind::NotImplemented(name) => {
                write!(f, "'{name:?}' uses a construct that is not implemented yet")
            }
        }
    }
}

impl std::error::Error for HIRError {}
///The reason why some type is being used incorrectly
#[derive(Debug)]
pub enum InvalidTypeReason {
    /// The type requires a generic parameter that was not provided.
    MissingGeneric,
    /// The type is being used in a context where it is not valid.
    IncorrectUsage,
    /// The type could not be inferred from context (e.g. variable without initializer and no type annotation).
    CouldntInfer,
}

impl std::fmt::Display for InvalidTypeReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidTypeReason::MissingGeneric => write!(f, "missing generic type"),
            InvalidTypeReason::IncorrectUsage => write!(f, "being used incorrectly"),
            InvalidTypeReason::CouldntInfer => write!(f, "could not infer type"),
        }
    }
}
