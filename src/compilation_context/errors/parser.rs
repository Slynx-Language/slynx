use slynx_parser::{ParseErrorKind, error::ParseError};

use crate::{
    ErrorPosition, LineInfo, SlynxContext,
    compilation_context::errors::{SlynxError, helpers::suggestions_from_parser},
};

impl SlynxContext {
    pub fn handle_parser_error(&self, error: ParseError) -> SlynxError {
        let suggestion = suggestions_from_parser(&error);
        match &error.kind {
            ParseErrorKind::ExpectedBounds(span) => {
                let info = self.get_line_info(&self.entry_point, span.start as usize);

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
            ParseErrorKind::InvalidPostfix(span) => {
                let info = self.get_line_info(&self.entry_point, span.start as usize);

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
            ParseErrorKind::UnexpectedToken(token, _) => {
                let LineInfo {
                    line,
                    column_start,
                    column_end,
                    src,
                } = self.get_line_info(&self.entry_point, token.span.start as usize);
                SlynxError::new_parser(
                    ErrorPosition {
                        line,
                        column: column_start,
                        end_column: column_end,
                    },
                    error.to_string(),
                    self.file_name(),
                    src.to_string(),
                    suggestion,
                    error.backtrace,
                )
            }
            ParseErrorKind::NoStyleUsagesProvided => SlynxError::new_parser(
                ErrorPosition {
                    line: 0,
                    column: 0,
                    end_column: 0,
                },
                error.to_string(),
                self.file_name(),
                String::new(),
                suggestion,
                error.backtrace,
            ),
            ParseErrorKind::UnexpectedEndOfInput => {
                let LineInfo {
                    line,
                    column_start,
                    column_end,
                    src,
                } = self.get_line_info(&self.entry_point, self.entry_point_eof_index());
                SlynxError::new_parser(
                    ErrorPosition {
                        line,
                        column: column_start,
                        end_column: column_end,
                    },
                    error.to_string(),
                    self.file_name(),
                    src.to_string(),
                    suggestion,
                    error.backtrace,
                )
            }
        }
    }
}
