use std::fmt;

use frontend::{
    checker::error::{TypeError, TypeErrorKind},
    hir::error::{HIRError, HIRErrorKind},
    lexer::error::LexerError,
    parser::error::ParseError,
};
use middleend::IRError;

#[derive(Debug, PartialEq)]
pub enum SlynxSuggestion {
    /// this error is raised when the Typeckeck
    IncompatibleTypes(String, String),
    CannotCastType(String, String),
    CiclicType(String),
    IncompatibleComponent(String),
    InvalidFuncallArgLength(String, String),
    NotARef(String, String),
    /// this error is raised when the lexer
    UnrecognizedChar(String, String),
    MalformedNumber(String, String, String),
    UnexpectedToken(String, String),
    NameAlreadyDefined(String),
    IRTypeNotRecognized(String),
    DeclarationNotRecognized(String),
}
impl fmt::Display for SlynxSuggestion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SlynxSuggestion::IncompatibleTypes(received, expected) => write!(
                f,
                "expected `{expected}`, but got `{received}` — consider explicitly converting the value to `{expected}`"
            ),
            SlynxSuggestion::CannotCastType(received, expected) => write!(
                f,
                "`{received}` cannot be cast to `{expected}` — there is no known conversion between these two types"
            ),
            SlynxSuggestion::CiclicType(type_name) => write!(
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
            SlynxSuggestion::NotARef(received, expected) => write!(
                f,
                "expected a reference to `{expected}`, but got `{received}` — try passing a reference instead"
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
            SlynxSuggestion::IRTypeNotRecognized(type_) => write!(
                f,
                "internal compiler error: type `{type_}` was not registered in the IR — this is likely a compiler bug, please report it"
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
        TypeErrorKind::CannotCastType { expected, received } => {
            vec![SlynxSuggestion::CannotCastType(
                format!("{:?}", received),
                format!("{:?}", expected),
            )]
        }
        TypeErrorKind::CiclicType { ty } => vec![SlynxSuggestion::CiclicType(format!("{:?}", ty))],
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
        TypeErrorKind::NotARef(a, b) => vec![SlynxSuggestion::NotARef(
            format!("{:?}", a),
            format!("{:?}", b),
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

/// this function converts a [`HIRError`] into a [`Vec<SlynxSuggestion>`]
pub fn suggestions_from_hir(hir: &crate::hir::SlynxHir, err: &HIRError) -> Vec<SlynxSuggestion> {
    match &err.kind {
        HIRErrorKind::NameAlreadyDefined(name) => {
            vec![SlynxSuggestion::NameAlreadyDefined(
                hir.get_name(*name).to_string(),
            )]
        }
        _ => vec![],
    }
}
/// this function converts a [`IRError`] into a [`Vec<SlynxSuggestion>`]
pub fn suggestions_from_ir(err: &IRError) -> Vec<SlynxSuggestion> {
    match &err {
        IRError::DeclarationNotRecognized(sla) => vec![SlynxSuggestion::DeclarationNotRecognized(
            format!("{}", sla.as_raw()),
        )],
        _ => vec![],
    }
}
/// tests for [`suggestions_from_ir`]
#[cfg(test)]
mod tests {

    use super::*;

    use frontend::{
        hir::DeclarationId,
        lexer::tokens::{Token, TokenKind},
    };

    #[test]
    /// tests that [`suggestions_from_hir`] returns [`SlynxSuggestion::NameAlreadyDefined`] for [`HIRErrorKind::NameAlreadyDefined`]
    fn test_suggestions_from_hir_name_already_defined() {
        // let err = HIRError {
        //     kind: HIRErrorKind::NameAlreadyDefined("foo".to_string()),
        //     span: common::Span { start: 0, end: 3 },
        // };

        // let result = suggestions_from_hir(&err);
        // assert_eq!(
        //     result,
        //     vec![SlynxSuggestion::NameAlreadyDefined("foo".to_string())]
        // );
    }
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
                "'foo'".to_string(),
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
        let id = DeclarationId::from_raw(42);
        let err = IRError::DeclarationNotRecognized(id);
        let result = suggestions_from_ir(&err);
        assert_eq!(
            result,
            vec![SlynxSuggestion::DeclarationNotRecognized("42".to_string())]
        );
    }
}
