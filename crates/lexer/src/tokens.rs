use common::Span;
use logos::Logos;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Logos, Debug, PartialEq, Clone)]
pub enum TokenKind {
    #[regex(r"[ \t\r\f\n]+", logos::skip)]
    Newline,
    // Comments (skip)
    #[regex(r"//[^\n]*", logos::skip, allow_greedy = true)]
    #[regex(r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/", logos::skip)]
    CommonComent,

    #[token("@")]
    At,
    // Keywords
    #[token("while")]
    While,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("component")]
    Component,
    #[token("func")]
    Func,
    #[token("pub")]
    Pub,
    #[token("prop")]
    Prop,
    #[token("alias")]
    Alias,
    #[token("object")]
    Object,
    #[token("let")]
    Let,
    #[token("mut")]
    Mut,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("stylesheet")]
    StyleSheet,
    #[token("import")]
    Import,
    // Multi-char operators (must come before single-char)
    #[token("&&")]
    And,
    #[token("||")]
    Or,
    #[token("<<")]
    ShiftLeft,
    #[token(">>")]
    ShiftRight,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("==")]
    EqEq,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    SubEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,
    #[token("->")]
    Arrow,

    // Single-char operators
    #[token("&")]
    BitAnd,
    #[token("|")]
    BitOr,
    #[token("^")]
    Xor,
    #[token("~")]
    BitNot,
    #[token(".")]
    Dot,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(";")]
    SemiColon,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("=")]
    Eq,
    #[token("+")]
    Plus,
    #[token("-")]
    Sub,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,

    // Literals
    #[regex(r"0x[0-9a-fA-F]+", |lex| i32::from_str_radix(&lex.slice()[2..], 16).ok())]
    #[regex(r"0b[01]+", |lex| i32::from_str_radix(&lex.slice()[2..], 2).ok())]
    #[regex(r"0o[0-7]+", |lex| i32::from_str_radix(&lex.slice()[2..], 8).ok())]
    #[regex(r"[0-9][0-9_]*", |lex| lex.slice().replace('_', "").parse::<i32>().ok())]
    Int(i32),

    // No underscore immediately before or after the decimal point
    #[regex(r"[0-9]([0-9]|_[0-9])*\.[0-9]([0-9]|_[0-9])*", |lex| lex.slice().replace('_', "").parse::<f32>().ok())]
    Float(f32),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        // strip surrounding quotes and process escapes
        let inner = &s[1..s.len()-1];
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('\\') => out.push('\\'),
                    Some('"') => out.push('"'),
                    Some(c) => { out.push('\\'); out.push(c); }
                    None => out.push('\\'),
                }
            } else {
                out.push(c);
            }
        }
        Some(out)
    })]
    String(String),

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{:?}'", self.kind)
    }
}
