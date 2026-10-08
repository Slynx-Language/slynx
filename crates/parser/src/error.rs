use std::backtrace::Backtrace;

use common::Span;
use slynx_lexer::{TokenKind, tokens::Token};

#[derive(Debug)]
pub enum ParserContext {
    OnlySignatures,
}

#[derive(Debug)]
pub enum ExpectedContent {
    Token(TokenKind),
    Raw(String),
    ParsingContext(ParserContext),
}

#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub backtrace: Backtrace,
}

#[derive(Debug)]
pub enum ParseErrorKind {
    ExpectedBounds(Span),
    ///An error that occurs when the provided `Token` is received when not intended. The provided `String` is a text to explain what was being expected instead. It's shown as 'Instead, was expecting `string`'
    UnexpectedToken(Token, ExpectedContent),
    UnexpectedEndOfInput,
    NoStyleUsagesProvided,
    InvalidPostfix(Span),
}

impl ParseError {
    pub fn new(kind: ParseErrorKind) -> Self {
        Self {
            kind,
            backtrace: Backtrace::capture(),
        }
    }
    pub fn invalid_postfix(span: Span) -> Self {
        Self::new(ParseErrorKind::InvalidPostfix(span))
    }
    pub fn unexpected_token(token: Token, expected: ExpectedContent) -> Self {
        Self::new(ParseErrorKind::UnexpectedToken(token, expected))
    }
    pub fn unexpected_end_of_input() -> Self {
        Self::new(ParseErrorKind::UnexpectedEndOfInput)
    }
    pub fn no_style_usages_provided() -> Self {
        Self::new(ParseErrorKind::NoStyleUsagesProvided)
    }
}

impl std::fmt::Display for ParseError {
    ///Formats the `ParseError` into a human-readable string. It matches on the type of error and constructs an appropriate message. For `UnexpectedToken`, it includes the unexpected token and what was expected. For `UnexpectedEndOfInput`, it simply states that the end of input was unexpected.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.kind {
            ParseErrorKind::ExpectedBounds(span) => {
                write!(f, "Expected bounds, but got none at {span:?}")
            }
            ParseErrorKind::UnexpectedToken(token, expected_ty) => {
                let expected = match expected_ty {
                    ExpectedContent::ParsingContext(ParserContext::OnlySignatures) => {
                        "The parser is trying to handle only signatures, but got body instead"
                            .to_string()
                    }
                    ExpectedContent::Token(kind) => format!("Instead got token of type {kind:?}"),
                    ExpectedContent::Raw(raw) => raw.clone(),
                };
                write!(f, "Unexpected token: {token}. {expected}",)
            }
            ParseErrorKind::UnexpectedEndOfInput => write!(f, "Unexpected end of input"),
            ParseErrorKind::NoStyleUsagesProvided => write!(
                f,
                "A style should use at least another 1 style, instead, got none"
            ),
            ParseErrorKind::InvalidPostfix(_) => write!(f, "Invalid postfix"),
        }
    }
}

impl std::error::Error for ParseError {}
