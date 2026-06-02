use slynx_typechecker::error::TypeError;

use crate::{
    LineInfo, SlynxContext,
    compilation_context::errors::{SlynxError, helpers::suggestions_from_type_error},
};

impl SlynxContext {
    pub fn handle_checker_error(&self, error: &TypeError) -> SlynxError {
        let suggestion = suggestions_from_type_error(error);
        let LineInfo {
            line,
            column_start,
            column_end,
            src,
        } = self.get_line_info(&self.entry_point, error.span.start);
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
