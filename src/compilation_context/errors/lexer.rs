use slynx_lexer::error::LexerError;

use crate::{
    SlynxContext,
    compilation_context::{errors::SlynxError, suggestions_from_lexer},
};

impl SlynxContext {
    pub fn handle_lexer_error(&self, error: LexerError) -> SlynxError {
        let suggestion = suggestions_from_lexer(&error);
        match error {
            LexerError::MalformedNumber { init, .. } => {
                let (line, column, src) = self.get_line_info(&self.entry_point, init);
                SlynxError::new_lexer(
                    line,
                    column,
                    error.to_string(),
                    self.file_name(),
                    src.to_string(),
                    suggestion,
                )
            }
            LexerError::UnrecognizedChar { index, .. } => {
                let (line, column, src) = self.get_line_info(&self.entry_point, index);
                SlynxError::new_lexer(
                    line,
                    column,
                    error.to_string(),
                    self.file_name(),
                    src.to_string(),
                    suggestion,
                )
            }
        }
    }
}
