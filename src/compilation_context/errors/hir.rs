use slynx_hir::{
    SlynxHir,
    error::{HIRError, HIRErrorKind},
};

use crate::{
    SlynxContext,
    compilation_context::errors::{SlynxError, helpers::suggestions_from_hir},
};

impl SlynxContext {
    fn hir_error_to_string(&self, hir: &SlynxHir, err: &HIRError) -> String {
        match &err.kind {
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
                format!("Type with name '{name}' is was not defined previously")
            }
            HIRErrorKind::InvalidFieldAccessTarget { ty } => {
                format!("Type '{ty:?}' does not support field-style access")
            }
            HIRErrorKind::InvalidTupleAccessTarget { ty } => {
                format!("Type '{ty:?}' does not support tuple-style access")
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
            HIRErrorKind::InvalidChild { .. } => {
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
                let names = prop_names
                    .iter()
                    .map(|v| hir.get_name(*v))
                    .collect::<Vec<&str>>()
                    .join(", ");
                format!("Property(ies) named as {names} is required but wasn't provided")
            }
            HIRErrorKind::PropertyNotRecognized { prop_names } => {
                let names = prop_names
                    .iter()
                    .map(|v| hir.get_name(*v))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("Property(ies) named as {names} are not recognized for this object")
            }
            HIRErrorKind::RecursiveType { ty } => {
                let ty = hir.get_name(*ty);
                format!("The type named as '{ty}' is recursive at this point")
            }
        }
    }

    pub fn handle_hir_error(&self, hir: &SlynxHir, error: &HIRError) -> SlynxError {
        let suggestion = suggestions_from_hir(hir, error);
        let (line, column, src) = self.get_line_info(&self.entry_point, error.span.start);
        SlynxError::new_hir(
            line,
            column,
            self.hir_error_to_string(hir, error),
            self.file_name(),
            src.to_string(),
            suggestion,
        )
    }
}
