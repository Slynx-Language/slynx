#[derive(Debug)]
pub enum LexerError {  
    UnrecognizedChar {  //indicates a character not recognized by the lexer, pointing out its index (position)
        char: char,
        index: usize,  
    },
    MalformedNumber { //indicates where the error occurred when the lexer attempted to process a sequence as a number,
        number: String,  // showing the start and end of the sequence
        init: usize,
        end: usize,
    },
}
impl std::error::Error for LexerError {} //treats LexerError as a formal/official Rust error

//handles error display, identifying the specific element (number or char) and indicating the cause of the error
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
