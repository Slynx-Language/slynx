use slynx_lexer::error::LexerError;

use crate::{
    LineInfo, SlynxContext, compilation_context::errors::SlynxError,
    helpers::suggestions_from_lexer,
};

impl SlynxContext {
    pub fn handle_lexer_error(&self, error: LexerError) -> SlynxError {
        let suggestion = suggestions_from_lexer(&error);
        match error {
            LexerError::MalformedNumber { init, .. } => {
                let LineInfo {
                    line,
                    column_start,
                    column_end,
                    src,
                } = self.get_line_info(&self.entry_point, init);
                SlynxError::new_lexer(
                    line,
                    column_start,
                    column_end,
                    error.to_string(),
                    self.file_name(),
                    src.to_string(),
                    suggestion,
                )
            }
            LexerError::UnrecognizedChar { index, .. } => {
                let LineInfo {
                    line,
                    column_start,
                    column_end,
                    src,
                } = self.get_line_info(&self.entry_point, index);
                SlynxError::new_lexer(
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
    }
}
