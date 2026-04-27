use frontend::lexer::error::LexerError;

use crate::{
    SlynxContext, SlynxError, SlynxErrorType, compilation_context::suggestions_from_lexer,
};
use color_eyre::Report;

impl SlynxContext {
    pub fn handle_lexer_error(&self, error: LexerError) -> color_eyre::Report {
        let suggestion = suggestions_from_lexer(&error);
        match error {
            LexerError::MalformedNumber { init, .. } => {
                let (line, column, src) = self.get_line_info(&self.entry_point, init);
                let err = SlynxError {
                    line,
                    ty: SlynxErrorType::Lexer,
                    column_start: column,
                    message: error.to_string(),
                    suggestion,
                    file: self.entry_point.to_string_lossy().to_string(),
                    source_code: src.to_string(),
                };
                Report::new(err)
            }
            LexerError::UnrecognizedChar { index, .. } => {
                let (line, column, src) = self.get_line_info(&self.entry_point, index);
                let err = SlynxError {
                    line,
                    ty: SlynxErrorType::Lexer,
                    column_start: column,
                    message: error.to_string(),
                    suggestion,
                    file: self.entry_point.to_string_lossy().to_string(),
                    source_code: src.to_string(),
                };
                Report::new(err)
            }
        }
    }
}
