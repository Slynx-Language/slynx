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
pub enum ParseError {
    ///An error that occurs when the provided `Token` is received when not intended. The provided `String` is a text to explain what was being expected instead. It's shown as 'Instead, was expecting `string`'
    UnexpectedToken(Token, ExpectedContent),
    UnexpectedEndOfInput,
    NoStyleUsagesProvided,
    InvalidPostfix(Span),
}

impl std::fmt::Display for ParseError {
    ///Formats the `ParseError` into a human-readable string. It matches on the type of error and constructs an appropriate message. For `UnexpectedToken`, it includes the unexpected token and what was expected. For `UnexpectedEndOfInput`, it simply states that the end of input was unexpected.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedToken(token, expected_ty) => {
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
            ParseError::UnexpectedEndOfInput => write!(f, "Unexpected end of input"),
            ParseError::NoStyleUsagesProvided => write!(
                f,
                "A style should use at least another 1 style, instead, got none"
            ),
            ParseError::InvalidPostfix(_) => write!(f, "Invalid postfix"),
        }
    }
}

impl std::error::Error for ParseError {}
