use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, col: usize) -> Self {
        Self {
            start,
            end,
            line,
            col,
        }
    }

    pub fn merge(self, other: Span) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line: self.line.min(other.line),
            col: if self.start <= other.start {
                self.col
            } else {
                other.col
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // Keywords
    Fn,
    Struct,
    Enum,
    Protocol,
    Impl,
    Import,
    Let,
    Const,
    If,
    Else,
    While,
    Loop,
    For,
    Break,
    Continue,
    Return,
    Match,
    Spawn,
    Send,
    Receive,
    And,
    Or,
    Not,
    True,
    False,
    Unit,
    Nil,

    // Identifiers & Atoms
    Ident(String),
    Atom(String), // :tag

    // Literals
    Int(i64),
    Float(u64), // f64 stored as u64 bits for Eq/Hash
    String(String),

    // Arithmetic & Assignment Operators
    Plus,    // +
    Minus,   // -
    Star,    // *
    Slash,   // /
    Rem,     // %
    PlusEq,  // +=
    MinusEq, // -=
    StarEq,  // *=
    SlashEq, // /=
    RemEq,   // %=

    // Comparison & Logic Operators
    Eq,    // =
    EqEq,  // ==
    NotEq, // !=
    Lt,    // <
    LtEq,  // <=
    Gt,    // >
    GtEq,  // >=

    // Arrows & Ranges
    Arrow,    // ->
    FatArrow, // =>
    DotDot,   // ..
    Dot,      // .
    Pipe,     // |

    // Delimiters
    LParen,    // (
    RParen,    // )
    LBracket,  // [
    RBracket,  // ]
    LBrace,    // {
    RBrace,    // }
    Comma,     // ,
    Semicolon, // ;
    Colon,     // :

    // Layout / Trivia
    Newline,
    Indent,
    Dedent,
    Eof,
}

impl TokenKind {
    pub fn float_val(&self) -> Option<f64> {
        match self {
            TokenKind::Float(bits) => Some(f64::from_bits(*bits)),
            _ => None,
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::Struct => write!(f, "struct"),
            TokenKind::Enum => write!(f, "enum"),
            TokenKind::Protocol => write!(f, "protocol"),
            TokenKind::Impl => write!(f, "impl"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::Let => write!(f, "let"),
            TokenKind::Const => write!(f, "const"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::While => write!(f, "while"),
            TokenKind::Loop => write!(f, "loop"),
            TokenKind::For => write!(f, "for"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::Spawn => write!(f, "spawn"),
            TokenKind::Send => write!(f, "send"),
            TokenKind::Receive => write!(f, "receive"),
            TokenKind::And => write!(f, "and"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::Not => write!(f, "not"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Unit => write!(f, "unit"),
            TokenKind::Nil => write!(f, "nil"),
            TokenKind::Ident(s) => write!(f, "{}", s),
            TokenKind::Atom(s) => write!(f, ":{}", s),
            TokenKind::Int(i) => write!(f, "{}", i),
            TokenKind::Float(bits) => write!(f, "{}", f64::from_bits(*bits)),
            TokenKind::String(s) => write!(f, "\"{}\"", s),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Rem => write!(f, "%"),
            TokenKind::PlusEq => write!(f, "+="),
            TokenKind::MinusEq => write!(f, "-="),
            TokenKind::StarEq => write!(f, "*="),
            TokenKind::SlashEq => write!(f, "/="),
            TokenKind::RemEq => write!(f, "%="),
            TokenKind::Eq => write!(f, "="),
            TokenKind::EqEq => write!(f, "=="),
            TokenKind::NotEq => write!(f, "!="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::LtEq => write!(f, "<="),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::GtEq => write!(f, ">="),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::DotDot => write!(f, ".."),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Newline => write!(f, "\\n"),
            TokenKind::Indent => write!(f, "<indent>"),
            TokenKind::Dedent => write!(f, "<dedent>"),
            TokenKind::Eof => write!(f, "<eof>"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub text: String,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span, text: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            text: text.into(),
        }
    }
}
