use slynx_hir::{
    SlynxHir,
    error::{HIRError, HIRErrorKind, InvalidWriteReason, NotMutableReason},
    ownership::{OwnershipError, OwnershipErrorKind},
};

use crate::{
    LineInfo, SlynxContext,
    compilation_context::errors::{SlynxError, helpers::suggestions_from_hir},
};

impl SlynxContext {
    fn hir_error_to_string(&self, hir: &SlynxHir, err: &HIRError) -> String {
        match &err.kind {
            HIRErrorKind::ExpressionNotMutable(NotMutableReason::ExpressionNotAssignable) => {
                "Expression cannot be mutable".to_string()
            }
            HIRErrorKind::ExpressionNotMutable(NotMutableReason::ImmutableVariable(variable)) => {
                format!(
                    "Variable '{}' is immutable and cannot be mutated",
                    hir.get_name(*variable)
                )
            }
            HIRErrorKind::InvalidDeref => {
                "Invalid deref, value being dereferenced is not a reference(mutable or imutable)"
                    .to_string()
            }
            HIRErrorKind::ArrayLengthMismatch { expected, actual } => {
                format!(
                    "Array length mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            HIRErrorKind::MissingReturn => {
                "Function does not contain return, but its return type is NOT void".to_string()
            }
            HIRErrorKind::UnexpectedType { expected, received } => {
                let expected_name = hir.view(*expected).name();
                let received_name = hir.view(*received).name();
                format!(
                    "Received an incorrect type. Expected {expected_name} instead, received type {received_name}"
                )
            }
            HIRErrorKind::InvalidIndexing(ty) => {
                format!(
                    "Expression cannot be indexed. Type is '{}', instead expected an array/vector type.",
                    hir.view(*ty).name()
                )
            }
            HIRErrorKind::CouldntInfer => "Could not infer the type of expression".to_string(),
            HIRErrorKind::ComponentNotFound(name) => format!(
                "Component named as '{}' could not be found",
                hir.get_name(*name)
            ),
            HIRErrorKind::NotAComponent(name) => {
                let name = hir.get_name(*name);
                format!("'{name}' is not a component")
            }
            HIRErrorKind::ComponentPropertyMissingType => {
                "Component property is missing type definition".to_string()
            }
            HIRErrorKind::InvalidWrite(InvalidWriteReason::ExpressionNotAssignable) => {
                "Expression is not assignable".to_string()
            }
            HIRErrorKind::InvalidWrite(InvalidWriteReason::ImmutableVariable(v)) => format!(
                "Invalid write to '{}' variable, which is immutable.",
                hir.get_name(*v)
            ),
            HIRErrorKind::InvalidWrite(InvalidWriteReason::ReferenceImmutable) => {
                "Reference being written is immutable".to_string()
            }
            HIRErrorKind::InvalidFieldAccess => "Invalid field access".to_string(),
            HIRErrorKind::InvalidFuncallArgLength {
                func_name,
                expected_length,
                received_length,
            } => {
                let func_name = hir.get_name(*func_name);
                format!(
                    "Function '{func_name}' expected to receive {expected_length} arguments, instead got {received_length} arguments"
                )
            }
            HIRErrorKind::NotAFunction(name, ty) => {
                let name = hir.get_name(*name);
                format!(
                    "The value with name '{name}' is being used as a function, but its type is {ty:?}"
                )
            }
            HIRErrorKind::NameNotRecognized(name) => {
                let name = hir.get_name(*name);
                format!(
                    "The name '{name}' is not recognized. Check if it exists or you wrote some typo"
                )
            }
            HIRErrorKind::TypeNotRecognized(name) => {
                let name = hir.get_name(*name);
                format!("Type with name '{name}' was not defined")
            }
            HIRErrorKind::InvalidFieldAccessTarget { ty } => {
                let ty = hir.view(*ty).name();
                format!("Type '{ty}' does not support field-style access")
            }
            HIRErrorKind::InvalidTupleAccessTarget { ty } => {
                let ty = hir.view(*ty).name();
                format!("Type '{ty}' does not support tuple-style access")
            }
            HIRErrorKind::InvalidTupleIndex { index, length } => {
                format!(
                    "Tuple index {index} is out of bounds. The tuple only exposes {length} fields"
                )
            }
            HIRErrorKind::InvalidBinaryExpression { .. } => "Invalid binary expression".to_string(),
            HIRErrorKind::PropertyNotVisible { prop_name } => {
                let prop_name = hir.get_name(*prop_name);
                format!("Property with name '{prop_name}' is not visible")
            }
            HIRErrorKind::InvalidChild => {
                "Invalid child. Component is not expecting children".to_string()
            }
            HIRErrorKind::InvalidType { ty, reason } => {
                let ty = hir.get_name(*ty);
                format!("Invalid type '{ty}' because it's {reason}")
            }
            HIRErrorKind::NameAlreadyDefined(name) => {
                let name = hir.get_name(*name);
                format!("The name '{name}' was already defined before. Use a different name")
            }
            HIRErrorKind::MissingProperty { prop_names } => {
                let property = if prop_names.len() == 1 {
                    "Property"
                } else {
                    "Properties"
                };
                let names = prop_names
                    .iter()
                    .map(|v| format!("'{}'", hir.get_name(*v)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{property} {names} is required but wasn't provided")
            }
            HIRErrorKind::PropertyNotRecognized { prop_names, ty } => {
                let property = if prop_names.len() == 1 {
                    "Property"
                } else {
                    "Properties"
                };
                let objname = hir.view(*ty).name();
                let names = prop_names
                    .iter()
                    .map(|v| format!("'{}'", hir.get_name(*v)))
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("{property} {names} are not recognized for object {objname}",)
            }
            HIRErrorKind::RecursiveType { ty } => {
                let name = hir.view(*ty).name();
                format!("The type named as '{name}' is recursive at this point")
            }
            HIRErrorKind::InvalidStyleEvent { name } => {
                let name = hir.get_name(*name);
                format!("Invalid style event '{name}'")
            }
            HIRErrorKind::InvalidStyleDefinition { name } => {
                let name = hir.get_name(*name);
                format!("Invalid style definition '{name}'")
            }
            HIRErrorKind::AmbiguousDeclaration {
                name,
                first,
                second,
            } => {
                let name = hir.get_name(*name);
                format!(
                    "The name '{name}' is ambiguous: it was found in files {first:?} and {second:?}",
                )
            }
            HIRErrorKind::IntrinsicNotRegistered { name } => {
                let name = hir.get_name(*name);
                format!("intrinsic '{name}' is not defined — ensure the standard library is loaded")
            }
            HIRErrorKind::CyclicComponentSignature { component, chain } => {
                let comp_name = hir.get_name(*component);
                let chain_str = chain
                    .iter()
                    .map(|(_, n)| hir.get_name(*n))
                    .collect::<Vec<_>>()
                    .join(" → ");
                format!("cyclic component signature: component '{comp_name}' at chain: {chain_str}")
            }
            HIRErrorKind::CyclicComponentBody { component: _ } => {
                "cyclic component body resolution".to_string()
            }
            HIRErrorKind::GenericArityMismatch {
                func,
                declared,
                supplied,
            } => {
                let func = hir.get_name(*func);
                format!(
                    "Generic function '{func}' expects {declared} type argument(s), got {supplied}"
                )
            }
            HIRErrorKind::CyclicMonomorphization { func, .. } => {
                let func = hir.get_name(*func);
                format!("Monomorphization of generic function '{func}' does not terminate")
            }
        }
    }

    pub fn handle_hir_error(&self, hir: &SlynxHir, error: &HIRError) -> SlynxError {
        let suggestion = suggestions_from_hir(hir, error);
        let LineInfo {
            line,
            column_start,
            column_end,
            src,
        } = self.get_line_info(&self.entry_point, error.span.start as usize);
        SlynxError::new_hir(
            line,
            column_start,
            column_end,
            self.hir_error_to_string(hir, error),
            self.file_name(),
            src.to_string(),
            suggestion,
        )
    }

    pub fn handle_ownership_error(&self, hir: &SlynxHir, error: &OwnershipError) -> SlynxError {
        let message = match &error.kind {
            OwnershipErrorKind::UseAfterMove { variable } => {
                format!(
                    "Variable '{}' is used after it was moved",
                    hir.get_variable_name(*variable)
                )
            }
            OwnershipErrorKind::ConflictingBorrow {
                variable,
                existing_borrow,
                new_borrow,
            } => {
                format!(
                    "Cannot borrow variable '{}' as {} because it is already {}",
                    hir.get_variable_name(*variable),
                    new_borrow,
                    existing_borrow
                )
            }
            OwnershipErrorKind::MoveWhileBorrowed { variable } => {
                format!(
                    "Cannot move variable '{}' because it is currently borrowed",
                    hir.get_variable_name(*variable)
                )
            }
            OwnershipErrorKind::MutablyBorrowImmutable { variable } => {
                format!(
                    "Cannot borrow variable '{}' as mutable because it is immutable",
                    hir.get_variable_name(*variable)
                )
            }
        };

        let LineInfo {
            line,
            column_start,
            column_end,
            src,
        } = self.get_line_info(&self.entry_point, error.span.start as usize);
        SlynxError::new_hir(
            line,
            column_start,
            column_end,
            message,
            self.file_name(),
            src.to_string(),
            vec![],
        )
    }
}
