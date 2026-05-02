pub mod error;
use std::{collections::VecDeque, ops::Index};

use crate::{
    error::LexerError,
    tokens::{Token, TokenKind},
};
use common::ast::Span;
pub mod tokens;
#[derive(Debug)]
///A stream of tokens to be used when parsing the content
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
        let mut out = VecDeque::new();
        let mut lines = Vec::new();
        let chars = source.chars().collect::<Vec<_>>();
        let mut idx = 0;
        while idx < chars.len() {
            let Some(tk) = chars.get(idx) else {
                break;
            };
            let tk = match tk {
                '&' => {
                    if let Some('&') = chars.get(idx + 1) {
                        idx += 1;
                        Token::and(idx)
                    } else {
                        Token::bitand(idx)
                    }
                }
                '|' => {
                    if let Some('|') = chars.get(idx + 1) {
                        idx += 1;
                        Token::or(idx)
                    } else {
                        Token::bitor(idx)
                    }
                }
                '^' => Token::xor(idx),
                '.' => Token::dot(idx),
                '(' => Token::lparen(idx),
                ')' => Token::rparen(idx),
                '{' => Token::lbrace(idx),
                '}' => Token::rbrace(idx),
                ';' => Token::semicolon(idx),
                '<' => {
                    if let Some('<') = chars.get(idx + 1) {
                        idx += 1;
                        Token::shl(idx)
                    } else if let Some('=') = chars.get(idx + 1) {
                        idx += 1;
                        Token::lte(idx)
                    } else {
                        Token::lt(idx)
                    }
                }
                '>' => {
                    if let Some('>') = chars.get(idx + 1) {
                        idx += 1;
                        Token::shr(idx)
                    } else if let Some('=') = chars.get(idx + 1) {
                        idx += 1;
                        Token::gte(idx)
                    } else {
                        Token::gt(idx)
                    }
                }
                '=' => {
                    if let Some('=') = chars.get(idx + 1) {
                        idx += 1;
                        Token::eqeq(idx)
                    } else {
                        Token::eq(idx)
                    }
                }

                ':' => Token::colon(idx),
                ',' => Token::comma(idx),
                '+' => {
                    if let Some('=') = chars.get(idx + 1) {
                        idx += 1;
                        Token::pluseq(idx)
                    } else {
                        Token::plus(idx)
                    }
                }
                '-' => {
                    if let Some('=') = chars.get(idx + 1) {
                        idx += 1;
                        Token::subeq(idx)
                    } else if let Some('>') = chars.get(idx + 1) {
                        let out = Token::arrow(idx);
                        idx += 1;
                        out
                    } else {
                        Token::sub(idx)
                    }
                }
                '*' => {
                    if let Some('=') = chars.get(idx + 1) {
                        idx += 1;
                        Token::stareq(idx)
                    } else {
                        Token::star(idx)
                    }
                }
                '/' => {
                    if let Some('*') = chars.get(idx + 1) {
                        while idx < chars.len() {
                            if let Some('\n') = chars.get(idx) {
                                lines.push(idx);
                            }
                            if let Some('*') = chars.get(idx)
                                && let Some('/') = chars.get(idx + 1)
                            {
                                break;
                            }
                            idx += 1;
                        }
                        continue;
                    } else if let Some('=') = chars.get(idx + 1) {
                        idx += 1;
                        Token::slasheq(idx)
                    } else if let Some('/') = chars.get(idx + 1) {
                        while idx < chars.len() {
                            if chars.get(idx) == Some(&'\n') {
                                lines.push(idx);
                            }
                            idx += 1;
                        }
                        continue;
                    } else {
                        Token::slash(idx)
                    }
                }

                '"' => {
                    let mut buffer = String::new();
                    idx += 1;
                    let start = idx;
                    while idx < chars.len()
                        && let Some(c) = chars.get(idx)
                        && *c != '"'
                    {
                        if *c == '\\' {
                            idx += 1;
                            match c {
                                'n' => buffer.push('\n'),
                                't' => buffer.push('\t'),
                                'r' => buffer.push('\r'),
                                c @ '0'..='9' => buffer.push(*c),
                                '\\' => buffer.push('\\'),
                                '"' => buffer.push('"'),
                                _ => buffer.push('\\'),
                            }
                        } else {
                            buffer.push(*c);
                        }

                        idx += 1;
                    }
                    Token::string(buffer, start, idx)
                }
                c if c.is_whitespace() => {
                    if *c == '\n' {
                        lines.push(idx);
                    }
                    idx += 1;
                    continue;
                }
                c if c.is_ascii_digit() => {
                    let mut buffer = String::new();
                    let start = idx;
                    let mut float_value = false;
                    let mut last_is_underscore = false;
                    let mut last_is_dot = false;
                    let mut should_err = false;
                    let mut hex_value = false;
                    let mut binary_value = false;
                    let mut radix = 0;
                    let mut octal_value = false;

                    if let Some('0') = chars.get(idx) {
                        match chars.get(idx + 1) {
                            Some('x') => {
                                radix += 16;
                                idx += 2;
                                hex_value = true;
                                while let Some(c) = chars.get(idx)
                                    && (c.is_ascii_hexdigit())
                                {
                                    buffer.push(*c);
                                    idx += 1;
                                }
                            }
                            Some('b') => {
                                binary_value = true;
                                radix += 2;
                                idx += 2;
                                while let Some(c) = chars.get(idx)
                                    && (*c == '0' || *c == '1')
                                {
                                    buffer.push(*c);
                                    idx += 1;
                                }
                            }
                            Some('o') => {
                                octal_value = true;
                                radix += 8;
                                idx += 2;
                                while let Some(c) = chars.get(idx)
                                    && (c.is_digit(radix))
                                {
                                    buffer.push(*c);
                                    idx += 1;
                                }
                            }
                            _ => (),
                        }
                    }

                    if !hex_value && !binary_value && !octal_value {
                        while let Some(c) = chars.get(idx) {
                            if c.is_ascii_digit() {
                                last_is_dot = false;
                                last_is_underscore = false;
                                buffer.push(*c);
                                idx += 1;
                                continue;
                            }

                            if *c == '_' {
                                if last_is_underscore || last_is_dot {
                                    should_err = true;
                                }
                                last_is_dot = false;
                                last_is_underscore = true;
                                buffer.push(*c);
                                idx += 1;
                                continue;
                            }

                            if *c == '.' {
                                let next = chars.get(idx + 1);
                                if !float_value
                                    && matches!(next, Some(next) if next.is_ascii_digit())
                                {
                                    // Only keep '.' inside a number when it actually starts
                                    // the fractional part of a float literal.
                                    float_value = true;
                                    if last_is_underscore {
                                        should_err = true;
                                    }
                                    last_is_dot = true;
                                    last_is_underscore = false;
                                    buffer.push(*c);
                                    idx += 1;
                                    continue;
                                }

                                if matches!(next, Some('.') | Some('_')) {
                                    should_err = true;
                                    buffer.push(*c);
                                    idx += 1;
                                }
                                break;
                            }

                            break;
                        }
                    }
                    if should_err || buffer.ends_with('_') || buffer.ends_with('.') {
                        return Err(LexerError::MalformedNumber {
                            number: buffer,
                            init: start,
                            end: idx,
                        });
                    }

                    idx -= 1;
                    let buffer = buffer.replace('_', "");
                    if hex_value || binary_value || octal_value {
                        let decimal_value = i32::from_str_radix(&buffer, radix);

                        match decimal_value {
                            Ok(value) => Token::int(value, start, idx),
                            Err(_) => {
                                return Err(LexerError::MalformedNumber {
                                    number: buffer,
                                    init: start,
                                    end: idx,
                                });
                            }
                        }
                    } else if float_value {
                        match buffer.parse::<f32>() {
                            Ok(value) => Token::float(value, start, idx),
                            Err(_) => {
                                return Err(LexerError::MalformedNumber {
                                    number: buffer,
                                    init: start,
                                    end: idx,
                                });
                            }
                        }
                    } else {
                        match buffer.parse::<i32>() {
                            Ok(value) => Token::int(value, start, idx),
                            Err(_) => {
                                return Err(LexerError::MalformedNumber {
                                    number: buffer,
                                    init: start,
                                    end: idx,
                                });
                            }
                        }
                    }
                }
                c if c.is_alphabetic() || *c == '_' => {
                    let mut buffer = String::new();
                    let start = idx;
                    while let Some(c) = chars.get(idx)
                        && (chars[idx].is_alphanumeric() || *c == '_')
                    {
                        buffer.push(*c);
                        idx += 1;
                    }

                    idx -= 1;
                    let span = Span { start, end: idx };
                    match buffer.as_str() {
                        "while" => Token {
                            kind: TokenKind::While,
                            span,
                        },

                        "let" => Token {
                            kind: TokenKind::Let,
                            span,
                        },
                        "mut" => Token {
                            kind: TokenKind::Mut,
                            span,
                        },
                        "object" => Token {
                            kind: TokenKind::Object,
                            span,
                        },
                        "component" => Token {
                            kind: TokenKind::Component,
                            span,
                        },
                        "func" => Token {
                            kind: TokenKind::Func,
                            span,
                        },
                        "pub" => Token {
                            kind: TokenKind::Pub,
                            span,
                        },
                        "prop" => Token {
                            kind: TokenKind::Prop,
                            span,
                        },
                        "if" => Token {
                            kind: TokenKind::If,
                            span,
                        },
                        "else" => Token {
                            kind: TokenKind::Else,
                            span,
                        },
                        "true" => Token {
                            kind: TokenKind::True,
                            span,
                        },
                        "false" => Token {
                            kind: TokenKind::False,
                            span,
                        },
                        "alias" => Token {
                            kind: TokenKind::Alias,
                            span,
                        },
                        _ => Token::identifier(&buffer, start, idx),
                    }
                }
                c => {
                    return Err(LexerError::UnrecognizedChar {
                        char: *c,
                        index: idx,
                    });
                }
            };
            out.push_back(tk);
            idx += 1;
        }
        Ok(TokenStream {
            stream: out,
            new_lines: lines,
        })
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
