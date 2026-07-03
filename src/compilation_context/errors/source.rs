use module_loader::{SourceError, SourceErrorKind};
use slynx_lexer::LexerError;

use crate::{
    SlynxContext,
    compilation_context::errors::{SlynxError, helpers::suggestions_from_source},
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
    pub fn handle_source_error(&self, error: &SourceError) -> SlynxError {
        let suggestion = suggestions_from_source(error);
        let file_name = error.entry().display().to_string();
        let file_key = self.find_file_key(error.entry());

        match error.kind() {
            SourceErrorKind::InexsitantSource(inner, span, generator) => {
                let file_key = self.find_file_key(&generator);
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
                    line,
                    col_start,
                    col_end,
                    msg,
                    generator.display().to_string(),
                    src,
                    suggestion,
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
                    line,
                    col_start,
                    col_end,
                    lex_err.to_string(),
                    file_name,
                    src,
                    suggestion,
                )
            }
            SourceErrorKind::Parsing(parse_err) => {
                let index = match parse_err {
                    slynx_parser::error::ParseError::UnexpectedToken(token, _) => token.span.start,
                    _ => 0,
                };
                let (line, col_start, col_end, src) = if let Some(ref key) = file_key {
                    let info = self.get_line_info(key, index as usize);
                    (
                        info.line,
                        info.column_start,
                        info.column_end,
                        info.src.to_string(),
                    )
                } else {
                    (0, 0, 0, String::new())
                };
                SlynxError::new_parser(
                    line,
                    col_start,
                    col_end,
                    parse_err.to_string(),
                    file_name,
                    src,
                    suggestion,
                )
            }
        }
    }
}
