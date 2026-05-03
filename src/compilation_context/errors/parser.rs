use slynx_parser::error::ParseError;

use crate::{
    SlynxContext,
    compilation_context::errors::{SlynxError, helpers::suggestions_from_parser},
};

impl SlynxContext {
    pub fn handle_parser_error(&self, error: &ParseError) -> SlynxError {
        match error {
            err @ ParseError::UnexpectedToken(token, _) => {
                let (line, column, src) = self.get_line_info(&self.entry_point, token.span.start);
                let suggestion = suggestions_from_parser(error);
                SlynxError::new_parser(
                    line,
                    column,
                    err.to_string(),
                    self.file_name(),
                    src.to_string(),
                    suggestion,
                )
            }
            err @ ParseError::UnexpectedEndOfInput => {
                let suggestion = suggestions_from_parser(err);
                let (line, column, src) =
                    self.get_line_info(&self.entry_point, self.entry_point_eof_index());
                SlynxError::new_parser(
                    line,
                    column,
                    err.to_string(),
                    self.file_name(),
                    src.to_string(),
                    suggestion,
                )
            }
        }
    }
}
