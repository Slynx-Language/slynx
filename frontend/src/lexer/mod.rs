pub mod error;
use std::{collections::VecDeque, ops::Index};

use crate::lexer::{
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
                '.' => Token::dot(idx),
                '(' => Token::lparen(idx),
                ')' => Token::rparen(idx),
                '{' => Token::lbrace(idx),
                '}' => Token::rbrace(idx),
                ';' => Token::semicolon(idx),
                '<' => {
                    if let Some('=') = chars.get(idx + 1) {
                        idx += 1;
                        Token::lte(idx)
                    } else {
                        Token::lt(idx)
                    }
                }
                '>' => {
                    if let Some('=') = chars.get(idx + 1) {
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
                    let mut should_err = false;
                    while let Some(c) = chars.get(idx)
                        && (c.is_ascii_digit() || *c == '.' || *c == '_')
                    {
                        if *c == '.' {
                            float_value = true;
                        }
                        if *c == '_' && last_is_underscore {
                            should_err = true;
                        }
                        last_is_underscore = *c == '_';
                        buffer.push(*c);
                        idx += 1;
                    }
                    if should_err || buffer.ends_with("_") {
                        return Err(LexerError::MalformedNumber {
                            number: buffer,
                            init: start,
                            end: idx,
                        });
                    }
                    idx -= 1;
                    let buffer = buffer.replace('_', "");
                    if float_value {
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
