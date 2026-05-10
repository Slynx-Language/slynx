use std::{
    fmt,
    ops::{Range, RangeFrom, RangeTo},
};

use std::fmt::Write;

use slynx_codegen::CodegenError;
use slynx_hir::{
    HIRError, HIRErrorKind, SlynxHir,
    module_loader::{SourceError, SourceErrorKind},
};

use slynx_lexer::error::LexerError;
use slynx_parser::error::ParseError;
use slynx_typechecker::error::{TypeError, TypeErrorKind};

#[derive(Debug)]
///A metadata containing the `file` and `message` amd `source` data in a single string. This is being made to don't explode the requirement of sizoef(error) < 128, of rust.
pub struct ErrorMetadata {
    ///The metadata containing `file`, `message` and `source` respectively
    metadata: String,
    ///The indexes where the `message` and `source` initialize. `file` is initialized at 0
    range_metadata: usize,
}

impl ErrorMetadata {
    ///Creates a new metadata containing the given `file`, `message` and `source`
    pub fn new(mut file: String, message: String, source: String) -> Self {
        let file_len = file.len();
        file.push_str(&message);
        file.push_str(&source);
        Self {
            metadata: file,
            range_metadata: file_len << 32 | message.len(),
        }
    }

    fn file_len(&self) -> usize {
        self.range_metadata >> 32
    }
    fn file_range(&self) -> RangeTo<usize> {
        ..self.file_len()
    }
    fn message_range(&self) -> Range<usize> {
        let file_len = self.file_len();

        (file_len)..(file_len + (self.range_metadata & 0xffffffff))
    }
    fn source_range(&self) -> RangeFrom<usize> {
        (self.file_len() + (self.range_metadata & 0xffffffff))..
    }

    ///Retrieves the file metadata
    pub fn file(&self) -> &str {
        &self.metadata[self.file_range()]
    }

    ///Retrieves the message metadata
    pub fn message(&self) -> &str {
        &self.metadata[self.message_range()]
    }

    ///Retrieves the source metadata
    pub fn source(&self) -> &str {
        &self.metadata[self.source_range()]
    }
}
#[derive(Debug, PartialEq)]
pub enum SlynxSuggestion {
    /// this error is raised when the Typeckeck
    IncompatibleTypes(String, String),
    CyclicType(String),
    IncompatibleComponent(String),
    InvalidFuncallArgLength(String, String),
    /// this error is raised when the lexer
    UnrecognizedChar(String, String),
    MalformedNumber(String, String, String),
    UnexpectedToken(String, String),
    NameAlreadyDefined(String),
    DeclarationNotRecognized(String),
}
impl fmt::Display for SlynxSuggestion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SlynxSuggestion::IncompatibleTypes(received, expected) => write!(
                f,
                "expected `{expected}`, but got `{received}` — consider explicitly converting the value to `{expected}`"
            ),
            SlynxSuggestion::CyclicType(type_name) => write!(
                f,
                "type `{type_name}` refers to itself, which creates an infinite cycle — try breaking the cycle with indirection"
            ),
            SlynxSuggestion::IncompatibleComponent(expected) => write!(
                f,
                "this value is not a valid component — it must satisfy `{expected}`"
            ),
            SlynxSuggestion::InvalidFuncallArgLength(received, expected) => write!(
                f,
                "this function expects {expected} argument(s), but {received} were provided"
            ),
            SlynxSuggestion::UnrecognizedChar(char, index) => write!(
                f,
                "the character `{char}` at position {index} is not valid here — remove or replace it"
            ),
            SlynxSuggestion::MalformedNumber(number, init, end) => write!(
                f,
                "`{number}` is not a valid number literal (problem near positions {init}–{end}) — check for invalid characters or incorrect formatting"
            ),
            SlynxSuggestion::UnexpectedToken(token, expected) => {
                write!(f, "unexpected `{token}` — expected {expected} here")
            }
            SlynxSuggestion::NameAlreadyDefined(name) => write!(
                f,
                "`{name}` is already defined in this scope — use a different name or remove the duplicate definition"
            ),
            SlynxSuggestion::DeclarationNotRecognized(decl) => write!(
                f,
                "internal compiler error: declaration `{decl}` was not registered in the IR — this is likely a compiler bug, please report it"
            ),
        }
    }
}
/// this function converts a [`TypeError`] into a [`Vec<SlynxSuggestion>`]
pub fn suggestions_from_type_error(err: &TypeError) -> Vec<SlynxSuggestion> {
    match &err.kind {
        TypeErrorKind::IncompatibleTypes { expected, received } => {
            vec![SlynxSuggestion::IncompatibleTypes(
                format!("{:?}", received),
                format!("{:?}", expected),
            )]
        }
        TypeErrorKind::CyclicType { ty } => vec![SlynxSuggestion::CyclicType(format!("{:?}", ty))],
        TypeErrorKind::IncompatibleComponent { reason } => {
            vec![SlynxSuggestion::IncompatibleComponent(format!(
                "{:?}",
                reason
            ))]
        }
        TypeErrorKind::InvalidFuncallArgLength {
            expected_length,
            received_length,
        } => vec![SlynxSuggestion::InvalidFuncallArgLength(
            format!("{}", received_length),
            format!("{}", expected_length),
        )],
        _ => vec![],
    }
}
/// this function converts a [`LexerError`] into a [`Vec<SlynxSuggestion>`]
pub fn suggestions_from_lexer(err: &LexerError) -> Vec<SlynxSuggestion> {
    match &err {
        LexerError::UnrecognizedChar { char, index } => vec![SlynxSuggestion::UnrecognizedChar(
            format!("{}", char),
            index.to_string(),
        )],
        LexerError::MalformedNumber { number, init, end } => {
            vec![SlynxSuggestion::MalformedNumber(
                number.clone(),
                init.to_string(),
                end.to_string(),
            )]
        }
    }
}
/// this function converts a [`ParseError`] into a [`Vec<SlynxSuggestion>`]
pub fn suggestions_from_parser(err: &ParseError) -> Vec<SlynxSuggestion> {
    match &err {
        ParseError::UnexpectedToken(token, expected) => vec![SlynxSuggestion::UnexpectedToken(
            format!("{}", token),
            expected.to_string(),
        )],
        _ => vec![],
    }
}

/// this function converts a [`IRError`] into a [`Vec<SlynxSuggestion>`]
pub fn suggestions_from_ir(err: &CodegenError) -> Vec<SlynxSuggestion> {
    match &err {
        CodegenError::DeclarationNotRecognized(sla) => {
            let mut buf = String::new();
            let _ = write!(buf, "{sla:?}");
            vec![SlynxSuggestion::DeclarationNotRecognized(buf)]
        }
        _ => vec![],
    }
}

/// this function converts a [`SourceError`] into a [`Vec<SlynxSuggestion>`]
pub fn suggestions_from_source(err: &SourceError) -> Vec<SlynxSuggestion> {
    match err.kind() {
        SourceErrorKind::Lexing(lex_err) => suggestions_from_lexer(lex_err),
        SourceErrorKind::Parsing(parse_err) => suggestions_from_parser(parse_err),
        SourceErrorKind::InexsitantSource(_, _, _) => vec![],
    }
}

/// this function converts a [`HIRError`] into a [`Vec<SlynxSuggestion>`]
pub fn suggestions_from_hir(hir: &SlynxHir, err: &HIRError) -> Vec<SlynxSuggestion> {
    match &err.kind {
        HIRErrorKind::NameAlreadyDefined(name) => {
            vec![SlynxSuggestion::NameAlreadyDefined(
                hir.get_name(*name).to_string(),
            )]
        }
        _ => vec![],
    }
}
/// tests for [`suggestions_from_ir`]
#[cfg(test)]
mod tests {

    use super::*;

    use slynx_hir::DeclarationId;
    use slynx_lexer::tokens::{Token, TokenKind};

    #[test]
    /// tests that [`suggestions_from_parser`] returns [`SlynxSuggestion::UnexpectedToken`] for [`ParseError::UnexpectedToken`]
    fn test_suggestions_parser() {
        let token = Token {
            kind: TokenKind::Identifier("foo".to_string()),
            span: common::Span { start: 0, end: 3 },
        };
        let err = ParseError::UnexpectedToken(token, "sla".to_string());
        let result = suggestions_from_parser(&err);
        assert_eq!(
            result,
            vec![SlynxSuggestion::UnexpectedToken(
                "'Identifier(\"foo\")'".to_string(),
                "sla".to_string()
            )]
        );
    }
    #[test]
    /// tests that [`suggestions_from_lexer`] returns [`SlynxSuggestion::UnrecognizedChar`] for [`LexerError::UnrecognizedChar`]
    fn test_suggestions_lexer() {
        let err = LexerError::UnrecognizedChar {
            char: 'a',
            index: 0,
        };
        let result = suggestions_from_lexer(&err);
        assert_eq!(
            result,
            vec![SlynxSuggestion::UnrecognizedChar(
                "a".to_string(),
                "0".to_string()
            )]
        );
    }
    #[test]
    /// tests that [`suggestions_from_ir`] returns [`SlynxSuggestion::DeclarationNotRecognized`] for [`IRError::DeclarationNotRecognized`]
    fn test_suggestions_ir() {
        use slynx_hir::id::LocalDeclId;
        use slynx_hir::module_loader::FileId;
        let id = DeclarationId::new(FileId::from_raw(0), LocalDeclId::from_raw(0));
        let err = CodegenError::DeclarationNotRecognized(id);
        let result = suggestions_from_ir(&err);
        assert_eq!(
            result,
            vec![SlynxSuggestion::DeclarationNotRecognized(
                "DeclarationId { file_id: FileId(0), local_id: LocalDeclId(0) }".to_string()
            )]
        );
    }
}
