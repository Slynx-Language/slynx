use common::ast::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = match self {
            Self::While => "while".to_string(),
            Self::CommonComent => "//".to_string(),
            Self::And => "&&".to_string(),
            Self::Or => "||".to_string(),

            Self::If => "if".to_string(),
            Self::Else => "else".to_string(),
            Self::Dot => ".".to_string(),
            Self::LParen => "(".to_string(),
            Self::RParen => ")".to_string(),
            Self::LBrace => "{".to_string(),
            Self::RBrace => "}".to_string(),
            Self::SemiColon => ";".to_string(),
            Self::Lt => "<".to_string(),
            Self::LtEq => "<=".to_string(),
            Self::Gt => ">".to_string(),
            Self::GtEq => ">=".to_string(),
            Self::Eq => "=".to_string(),
            Self::EqEq => "==".to_string(),
            Self::Plus => "+".to_string(),
            Self::PlusEq => "+=".to_string(),
            Self::Sub => "-".to_string(),
            Self::SubEq => "-=".to_string(),
            Self::Star => "*".to_string(),
            Self::StarEq => "*=".to_string(),
            Self::Slash => "/".to_string(),
            Self::SlashEq => "/=".to_string(),
            Self::Arrow => "->".to_string(),
            Self::Comma => ",".to_string(),
            Self::Colon => ":".to_string(),
            Self::Component => "component".to_string(),
            Self::Func => "func".to_string(),
            Self::Pub => "pub".to_string(),
            Self::Prop => "prop".to_string(),
            Self::Alias => "alias".to_string(),
            Self::Object => "object".to_string(),
            Self::Let => "let".to_string(),
            Self::Mut => "mut".to_string(),
            Self::True => "true".to_string(),
            Self::False => "false".to_string(),
            Self::ShiftRight => "<<".to_string(),
            Self::ShiftLeft => ">>".to_string(),
            Self::Xor => "^".to_string(),
            Self::BitAnd => "&".to_string(),
            Self::BitOr => "|".to_string(),
            Self::BitNot => "~".to_string(),
            Self::Float(value) => value.to_string(),
            Self::Int(value) => value.to_string(),
            Self::String(value) => value.to_string(),
            Self::Identifier(value) => value.to_string(),
        };
        write!(f, "{}", result)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TokenKind {
    While,
    CommonComent,
    LParen,
    If,
    Else,
    RParen,
    LBrace,
    RBrace,
    SemiColon,

    ShiftRight,
    ShiftLeft,
    Xor,
    BitAnd,
    BitOr,
    BitNot,

    Lt,
    LtEq,

    Gt,
    GtEq,

    Eq,
    EqEq,

    Plus,
    PlusEq,

    Sub,
    SubEq,

    Star,
    StarEq,

    Slash,
    SlashEq,

    And,
    Or,

    Arrow,

    Comma,

    Colon,

    Dot,
    Float(f32),
    Int(i32),
    String(String),
    Identifier(String),

    Component,
    Func,
    Pub,
    Prop,
    Alias,

    Object,

    Let,
    Mut,

    True,
    False,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'", self.kind)
    }
}

impl Token {
    pub fn comcomment(pos: usize) -> Self {
        Self {
            kind: TokenKind::CommonComent,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    // constructor helpers for keywords; snake_case to satisfy lint
    pub fn if_token(pos: usize) -> Self {
        Self {
            kind: TokenKind::If,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn else_token(pos: usize) -> Self {
        Self {
            kind: TokenKind::Else,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn shr(pos: usize) -> Self {
        Self {
            kind: TokenKind::ShiftRight,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn shl(pos: usize) -> Self {
        Self {
            kind: TokenKind::ShiftLeft,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }

    pub fn xor(pos: usize) -> Self {
        Self {
            kind: TokenKind::Xor,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn bitnot(pos: usize) -> Self {
        Self {
            kind: TokenKind::BitNot,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }

    pub fn bitor(pos: usize) -> Self {
        Self {
            kind: TokenKind::BitOr,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn bitand(pos: usize) -> Self {
        Self {
            kind: TokenKind::BitAnd,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn or(pos: usize) -> Self {
        Self {
            kind: TokenKind::Or,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn and(pos: usize) -> Self {
        Self {
            kind: TokenKind::And,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn dot(pos: usize) -> Self {
        Self {
            kind: TokenKind::Dot,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn lparen(pos: usize) -> Self {
        Self {
            kind: TokenKind::LParen,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn rparen(pos: usize) -> Self {
        Self {
            kind: TokenKind::RParen,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn lbrace(pos: usize) -> Self {
        Self {
            kind: TokenKind::LBrace,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn rbrace(pos: usize) -> Self {
        Self {
            kind: TokenKind::RBrace,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn semicolon(pos: usize) -> Self {
        Self {
            kind: TokenKind::SemiColon,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn lt(pos: usize) -> Self {
        Self {
            kind: TokenKind::Lt,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn gt(pos: usize) -> Self {
        Self {
            kind: TokenKind::Gt,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn eq(pos: usize) -> Self {
        Self {
            kind: TokenKind::Eq,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn colon(pos: usize) -> Self {
        Self {
            kind: TokenKind::Colon,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn comma(pos: usize) -> Self {
        Self {
            kind: TokenKind::Comma,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn plus(pos: usize) -> Self {
        Self {
            kind: TokenKind::Plus,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn sub(pos: usize) -> Self {
        Self {
            kind: TokenKind::Sub,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn star(pos: usize) -> Self {
        Self {
            kind: TokenKind::Star,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn slash(pos: usize) -> Self {
        Self {
            kind: TokenKind::Slash,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn lte(pos: usize) -> Self {
        Self {
            kind: TokenKind::LtEq,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn gte(pos: usize) -> Self {
        Self {
            kind: TokenKind::GtEq,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn pluseq(pos: usize) -> Self {
        Self {
            kind: TokenKind::PlusEq,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn subeq(pos: usize) -> Self {
        Self {
            kind: TokenKind::SubEq,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn stareq(pos: usize) -> Self {
        Self {
            kind: TokenKind::StarEq,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn slasheq(pos: usize) -> Self {
        Self {
            kind: TokenKind::SlashEq,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn eqeq(pos: usize) -> Self {
        Self {
            kind: TokenKind::EqEq,
            span: Span {
                end: pos,
                start: pos,
            },
        }
    }
    pub fn float(value: f32, start: usize, end: usize) -> Self {
        Self {
            kind: TokenKind::Float(value),
            span: Span { end, start },
        }
    }
    pub fn int(value: i32, start: usize, end: usize) -> Self {
        Self {
            kind: TokenKind::Int(value),
            span: Span { end, start },
        }
    }
    pub fn identifier(buffer: &str, start: usize, end: usize) -> Self {
        Self {
            kind: TokenKind::Identifier(buffer.to_string()),
            span: Span { end, start },
        }
    }
    pub fn component(start: usize, end: usize) -> Self {
        Self {
            kind: TokenKind::Component,
            span: Span { end, start },
        }
    }
    pub fn arrow(start: usize) -> Self {
        Self {
            kind: TokenKind::Arrow,
            span: Span {
                start,
                end: start + 1,
            },
        }
    }
    pub fn string(str: String, start: usize, end: usize) -> Self {
        Self {
            kind: TokenKind::String(str),
            span: Span { end, start },
        }
    }
}
