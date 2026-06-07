use common::Span;

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

            Self::If => "if".to_string(),
            Self::Else => "else".to_string(),
            Self::StyleSheet => "stylesheet".to_string(),

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
            Self::Import => "import".to_string(),

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
    StyleSheet,

    Object,

    Let,
    Mut,

    True,
    False,

    Import,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'", self.kind)
    }
}

macro_rules! impl_token {
    // start=end=pos
    (
        simple:
        $(
            $fn_name:ident => $kind:ident
        ),* $(,)?
    ) => {
        $(
            pub fn $fn_name(pos: usize) -> Self {
                Self {
                    kind: TokenKind::$kind,
                    span: Span {
                        start: pos,
                        end: pos,
                    },
                }
            }
        )*
    };

    // literal.len()
    (
        literal:
        $(
            $fn_name:ident($lit:literal) => $kind:ident
        ),* $(,)?
    ) => {
        $(
            pub fn $fn_name(pos: usize) -> Self {
                Self {
                    kind: TokenKind::$kind,
                    span: Span {
                        start: pos,
                        end: pos + $lit.len(),
                    },
                }
            }
        )*
    };

    // TokenKind::X(value)
    (
        value:
        $(
            $fn_name:ident(
                $arg:ident : $arg_ty:ty
            ) => $kind:ident
        ),* $(,)?
    ) => {
        $(
            pub fn $fn_name(
                $arg: $arg_ty,
                start: usize,
                end: usize
            ) -> Self {
                Self {
                    kind: TokenKind::$kind($arg),
                    span: Span { start, end },
                }
            }
        )*
    };

    // sem payload, mas recebe start/end
    (
        span:
        $(
            $fn_name:ident => $kind:ident
        ),* $(,)?
    ) => {
        $(
            pub fn $fn_name(
                start: usize,
                end: usize
            ) -> Self {
                Self {
                    kind: TokenKind::$kind,
                    span: Span { start, end },
                }
            }
        )*
    };

    // end = start + CONST
    (
        custom_end:
        $(
            $fn_name:ident($offset:expr) => $kind:ident
        ),* $(,)?
    ) => {
        $(
            pub fn $fn_name(start: usize) -> Self {
                Self {
                    kind: TokenKind::$kind,
                    span: Span {
                        start,
                        end: start + $offset,
                    },
                }
            }
        )*
    };
}

impl Token {
    impl_token! {
        simple:
        plus => Plus,
        sub => Sub,
        star => Star,
        slash => Slash,
        comma => Comma,
        colon => Colon,
        semicolon => SemiColon,
        rbrace => RBrace,
        lbrace => LBrace,
        rparen => RParen,
        lparen => LParen,
        dot => Dot,

        bitand => BitAnd,
        bitor => BitOr,
        xor => Xor,
        shr => ShiftRight,
        shl => ShiftLeft,

        eq => Eq,
        gt => Gt,
        lt => Lt,
        or => Or,
        and => And,

        lte => LtEq,
        gte => GtEq,
        pluseq => PlusEq,
        subeq => SubEq,
        stareq => StarEq,
        slasheq => SlashEq,
        eqeq => EqEq
    }

    impl_token! {
        span:
        component => Component
    }

    impl_token! {
        custom_end:
        arrow(1) => Arrow
    }

    impl_token! {
        value:
        int(value: i32) => Int,
        float(value: f32) => Float,
        string(value: String) => String
    }

    pub fn identifier(buffer: &str, start: usize, end: usize) -> Self {
        Self {
            kind: TokenKind::Identifier(buffer.to_string()),
            span: Span { start, end },
        }
    }
}
