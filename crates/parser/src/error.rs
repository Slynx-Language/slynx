use slynx_lexer::tokens::Token;

#[derive(Debug)]
pub enum ParseError {
    ///An error that occurs when the provided `Token` is received when not intended. The provided `String` is a text to explain what was being expected instead. It's shown as 'Instead, was expecting `string`'
    UnexpectedToken(Token, String),
    UnexpectedEndOfInput,
}

impl std::fmt::Display for ParseError {
    ///Formats the `ParseError` into a human-readable string. It matches on the type of error and constructs an appropriate message. For `UnexpectedToken`, it includes the unexpected token and what was expected. For `UnexpectedEndOfInput`, it simply states that the end of input was unexpected.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedToken(token, expected_ty) => {
                write!(
                    f,
                    "Unexpected token: {token}. Instead, was expecing {expected_ty}",
                )
            }
            ParseError::UnexpectedEndOfInput => write!(f, "Unexpected end of input"),
        }
    }
}

impl std::error::Error for ParseError {}
