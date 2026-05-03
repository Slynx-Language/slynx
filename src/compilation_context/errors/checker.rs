use slynx_typechecker::error::TypeError;

use crate::{
    SlynxContext,
    compilation_context::errors::{SlynxError, helpers::suggestions_from_type_error},
};

impl SlynxContext {
    pub fn handle_checker_error(&self, error: &TypeError) -> SlynxError {
        let suggestion = suggestions_from_type_error(error);
        let (line, column, src) = self.get_line_info(&self.entry_point, error.span.start);
        SlynxError::new_type(
            line,
            column,
            error.to_string(),
            self.file_name(),
            src.to_string(),
            suggestion,
        )
    }
}
