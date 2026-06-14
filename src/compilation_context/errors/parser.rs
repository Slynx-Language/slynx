use slynx_parser::error::ParseError;

use crate::{
    LineInfo, SlynxContext,
    compilation_context::errors::{SlynxError, helpers::suggestions_from_parser},
};

impl SlynxContext {
    pub fn handle_parser_error(&self, error: &ParseError) -> SlynxError {
        match error {
            err @ ParseError::InvalidPostfix(span) => {
                let info = self.get_line_info(&self.entry_point, span.start);
                let suggestion = suggestions_from_parser(error);
                SlynxError::new_parser(
                    info.line,
                    info.column_start,
                    info.column_end,
                    err.to_string(),
                    self.file_name(),
                    info.src.to_string(),
                    suggestion,
                )
            }
            err @ ParseError::UnexpectedToken(token, _) => {
                let LineInfo {
                    line,
                    column_start,
                    column_end,
                    src,
                } = self.get_line_info(&self.entry_point, token.span.start);
                let suggestion = suggestions_from_parser(error);
                SlynxError::new_parser(
                    line,
                    column_start,
                    column_end,
                    err.to_string(),
                    self.file_name(),
                    src.to_string(),
                    suggestion,
                )
            }
            err @ ParseError::NoStyleUsagesProvided => {
                let suggestion = suggestions_from_parser(err);
                SlynxError::new_parser(
                    0,
                    0,
                    0,
                    err.to_string(),
                    self.file_name(),
                    String::new(),
                    suggestion,
                )
            }
            err @ ParseError::UnexpectedEndOfInput => {
                let suggestion = suggestions_from_parser(err);
                let LineInfo {
                    line,
                    column_start,
                    column_end,
                    src,
                } = self.get_line_info(&self.entry_point, self.entry_point_eof_index());
                SlynxError::new_parser(
                    line,
                    column_start,
                    column_end,
                    err.to_string(),
                    self.file_name(),
                    src.to_string(),
                    suggestion,
                )
            }
        }
    }
}
