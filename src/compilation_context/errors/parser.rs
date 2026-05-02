use slynx_parser::error::ParseError;

use crate::{
    SlynxContext, SlynxError, SlynxErrorType,
    compilation_context::errors::helpers::suggestions_from_parser,
};

impl SlynxContext {
    pub fn handle_parser_error(&self, error: &ParseError) -> color_eyre::Report {
        match error {
            err @ ParseError::UnexpectedToken(token, _) => {
                let (line, column, src) = self.get_line_info(&self.entry_point, token.span.start);
                let suggestion = suggestions_from_parser(error);
                let err = SlynxError {
                    line,
                    ty: SlynxErrorType::Parser,
                    column_start: column,
                    message: err.to_string(),
                    file: self.file_name(),
                    suggestion,
                    source_code: src.to_string(),
                };
                err.into()
            }
            err @ ParseError::UnexpectedEndOfInput => {
                let suggestion = suggestions_from_parser(err);
                let (line, column, src) =
                    self.get_line_info(&self.entry_point, self.entry_point_eof_index());
                let err = SlynxError {
                    line,
                    ty: SlynxErrorType::Parser,
                    column_start: column,
                    message: err.to_string(),
                    file: self.file_name(),
                    suggestion,
                    source_code: src.to_string(),
                };
                err.into()
            }
        }
    }
}
