use slynx_parser::error::ParseError;

use crate::{
    ErrorPosition, SlynxContext,
    compilation_context::errors::{SlynxError, helpers::suggestions_from_parser},
};

impl SlynxContext {
    pub fn handle_parser_error(&self, error: ParseError) -> SlynxError {
        let suggestion = suggestions_from_parser(&error);
        let info = self.get_line_info(&self.entry_point, error.span.start as usize);

        SlynxError::new_parser(
            ErrorPosition {
                line: info.line,
                column: info.column_start,
                end_column: info.column_end,
            },
            error.to_string(),
            self.file_name(),
            info.src.to_string(),
            suggestion,
            error.backtrace,
        )
    }
}
