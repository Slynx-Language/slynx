pub mod error;
use common::Span;
pub use error::*;
use logos::Logos;
use std::{collections::VecDeque, ops::Index};
pub mod tokens;
pub use tokens::*;
#[derive(Debug)]
/// A stream of tokens to be used when parsing the content
pub struct TokenStream {
    pub stream: VecDeque<Token>,
    pub new_lines: Vec<usize>,
}

impl Iterator for TokenStream {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        self.stream.pop_front()
    }
}

impl Index<usize> for TokenStream {
    type Output = Token;
    fn index(&self, index: usize) -> &Self::Output {
        &self.stream[index]
    }
}

pub struct Lexer;

impl Lexer {
    pub fn tokenize(source: &str) -> Result<TokenStream, LexerError> {
        let mut stream = VecDeque::new();
        let mut new_lines = Vec::new();

        let raw: Vec<(Result<TokenKind, ()>, std::ops::Range<usize>)> =
            TokenKind::lexer(source).spanned().collect();

        for i in 0..raw.len() {
            let (Ok(result), range) = &raw[i] else {
                let range = &raw[i].1;
                let c = source[range.clone()].chars().next().unwrap_or('?');
                return Err(LexerError::UnrecognizedChar {
                    char: c,
                    index: range.start,
                });
            };
            match result {
                TokenKind::Newline => {
                    new_lines.push(range.start);
                }
                kind => {
                    // Reject `<int>..` (double-dot after number)
                    if matches!(kind, TokenKind::Int(_) | TokenKind::Float(_))
                        && let Some((Ok(TokenKind::Dot), dot1)) = raw.get(i + 1)
                        && let Some((Ok(TokenKind::Dot), _)) = raw.get(i + 2)
                    {
                        return Err(LexerError::MalformedNumber {
                            number: source[range.start..dot1.end].to_string(),
                            init: range.start,
                            end: dot1.end,
                        });
                    }
                    // Reject `<int>._` or `<int_ending_with_underscore>.`
                    // logos won't produce Int with trailing underscore, but catches `1._0`
                    // pattern: Int Dot Identifier-starting-with-underscore
                    if matches!(kind, TokenKind::Int(_)) {
                        if let Some((Ok(TokenKind::Dot), dot_range)) = raw.get(i + 1)
                            && let Some((Ok(TokenKind::Identifier(id)), _)) = raw.get(i + 2)
                            && id.starts_with('_')
                        {
                            return Err(LexerError::MalformedNumber {
                                number: source[range.start..dot_range.end].to_string(),
                                init: range.start,
                                end: dot_range.end,
                            });
                        }
                        // Reject `1_.0`: logos lexes `1_` as error, but let's also check
                        // if the raw source slice for this int ends with '_'
                        if source[range.clone()].ends_with('_') {
                            return Err(LexerError::MalformedNumber {
                                number: source[range.clone()].to_string(),
                                init: range.start,
                                end: range.end,
                            });
                        }
                    }
                    stream.push_back(Token {
                        kind: kind.clone(),
                        span: Span {
                            start: range.start as u32,
                            end: range.end as u32,
                        },
                    });
                }
            }
        }

        Ok(TokenStream { stream, new_lines })
    }
}

#[cfg(test)]
mod tests {
    use super::{Lexer, error::LexerError, tokens::TokenKind};

    #[test]
    fn lexes_tuple_access_index_as_a_standalone_integer() {
        let tokens = Lexer::tokenize("pair.0.age")
            .expect("tuple access should keep the numeric index separate from dots");
        let kinds = tokens
            .stream
            .into_iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>();

        assert!(matches!(kinds[0], TokenKind::Identifier(ref name) if name == "pair"));
        assert!(matches!(kinds[1], TokenKind::Dot));
        assert!(matches!(kinds[2], TokenKind::Int(0)));
        assert!(matches!(kinds[3], TokenKind::Dot));
        assert!(matches!(kinds[4], TokenKind::Identifier(ref name) if name == "age"));
    }

    #[test]
    fn rejects_double_dot_number_patterns() {
        let err = Lexer::tokenize("4..0").expect_err("double-dot numerics should stay malformed");

        assert!(
            matches!(err, LexerError::MalformedNumber { .. }),
            "expected malformed number, got {err:?}"
        );
    }

    #[test]
    fn rejects_numeric_separators_adjacent_to_decimal_points() {
        for source in ["1._0", "1_.0"] {
            let err = Lexer::tokenize(source)
                .expect_err("numeric separators next to decimal points should be malformed");

            assert!(
                matches!(err, LexerError::MalformedNumber { .. }),
                "expected malformed number for {source}, got {err:?}"
            );
        }
    }
}
