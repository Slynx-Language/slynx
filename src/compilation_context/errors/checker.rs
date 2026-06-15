use slynx_hir::SlynxHir;
use slynx_typechecker::{TypeErrorKind, error::TypeError};

use crate::{
    LineInfo, SlynxContext,
    compilation_context::errors::{SlynxError, helpers::suggestions_from_type_error},
};

impl SlynxContext {
    pub fn handle_checker_error(&self, error: &TypeError, hir: &SlynxHir) -> SlynxError {
        let suggestion = suggestions_from_type_error(error);
        let LineInfo {
            line,
            column_start,
            column_end,
            src,
        } = self.get_line_info(&self.entry_point, error.span.start);
        let error = match error.kind {
            TypeErrorKind::NoMethodFor { ty, name } => {
                format!(
                    "No method named as {} for type {}",
                    hir.get_name(name),
                    hir.get_name(hir.get_name_of_type(ty).expect("Type should be named"))
                )
            }
            _ => error.to_string(),
        };
        SlynxError::new_type(
            line,
            column_start,
            column_end,
            error.to_string(),
            self.file_name(),
            src.to_string(),
            suggestion,
        )
    }
}
