use module_loader::{SourceError, SourceErrorKind};
use slynx_lexer::LexerError;

use crate::{
    ErrorPosition, SlynxContext,
    compilation_context::errors::{SlynxError, helpers::suggestions_from_source},
    helpers::suggestions_from_parser,
};

impl SlynxContext {
    ///Finds the `Arc<PathBuf>` key for a given path in the context's file map.
    fn find_file_key(&self, path: &std::path::Path) -> Option<std::sync::Arc<std::path::PathBuf>> {
        self.files.iter().find_map(|k| {
            let key = k.key().clone();
            (*key == *path).then_some(key)
        })
    }

    ///Handles a source error. The `generator` is the path of the file that generated the provided `error`
    pub fn handle_source_error(&self, error: SourceError) -> SlynxError {
        let suggestion = suggestions_from_source(&error);
        let file_name = error.entry().display().to_string();
        let file_key = self.find_file_key(error.entry());

        match error.kind() {
            SourceErrorKind::InexistantSource(inner, span, generator) => {
                let file_key = self.find_file_key(generator);
                let (line, col_start, col_end, src) = if let Some(ref key) = file_key {
                    let info = self.get_line_info(key, span.start as usize);
                    (
                        info.line,
                        info.column_start,
                        info.column_end,
                        info.src.to_string(),
                    )
                } else {
                    Default::default()
                };
                let msg = format!("Could not open file '{}': {inner}", error.entry().display());
                SlynxError::new_compiler(
                    ErrorPosition {
                        line,
                        column: col_start,
                        end_column: col_end,
                    },
                    msg,
                    generator.display().to_string(),
                    src,
                    suggestion,
                    error.backtrace(),
                )
            }
            SourceErrorKind::Lexing(lex_err) => {
                let (index, _end_index) = match lex_err {
                    LexerError::MalformedNumber { init, end, .. } => (*init, *end),
                    LexerError::UnrecognizedChar { index, .. } => (*index, *index),
                };
                let (line, col_start, col_end, src) = if let Some(ref key) = file_key {
                    let info = self.get_line_info(key, index);
                    (
                        info.line,
                        info.column_start,
                        info.column_end,
                        info.src.to_string(),
                    )
                } else {
                    (0, 0, 0, String::new())
                };
                SlynxError::new_lexer(
                    ErrorPosition {
                        line,
                        column: col_start,
                        end_column: col_end,
                    },
                    lex_err.to_string(),
                    file_name,
                    src,
                    suggestion,
                    error.backtrace(),
                )
            }
            SourceErrorKind::Parsing(parse_err) => {
                let suggestion = suggestions_from_parser(&parse_err);
                let info = self.get_line_info(&self.entry_point, parse_err.span.start as usize);

                SlynxError::new_parser(
                    ErrorPosition {
                        line: info.line,
                        column: info.column_start,
                        end_column: info.column_end,
                    },
                    parse_err.to_string(),
                    self.file_name(),
                    info.src.to_string(),
                    suggestion,
                    error.backtrace(),
                )
            }
        }
    }
}
