#[derive(Debug)]
pub enum LexerError {
    UnrecognizedChar {
        char: char,
        index: usize,
    },
    MalformedNumber {
        number: String,
        init: usize,
        end: usize,
    },
}
impl std::error::Error for LexerError {}

impl std::fmt::Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedNumber { number, .. } => write!(
                f,
                "The number {number} is malformed and could not be tokenized."
            ),
            Self::UnrecognizedChar { char, .. } => {
                write!(
                    f,
                    "The char '{char}' is not recognized by the lexer. Check for the language syntax to know what chars are accepted"
                )
            }
        }
    }
}
