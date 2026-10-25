use super::token::{Span, Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexMode {
    Normal,
    Expr,
    ExprMl,
    ExprBlock,
}

#[derive(Debug, Clone, Copy)]
pub struct LexViewSave {
    pub pos: usize,
    pub line: usize,
    pub col: usize,
    pub mode_len: usize,
}

pub struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    line: usize,
    col: usize,
    mode_stack: Vec<LexMode>,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            pos: 0,
            line: 1,
            col: 1,
            mode_stack: vec![LexMode::Normal],
        }
    }

    pub fn save(&self) -> LexViewSave {
        LexViewSave {
            pos: self.pos,
            line: self.line,
            col: self.col,
            mode_len: self.mode_stack.len(),
        }
    }

    pub fn restore(&mut self, save: LexViewSave) {
        self.pos = save.pos;
        self.line = save.line;
        self.col = save.col;
        self.mode_stack.truncate(save.mode_len);
    }

    pub fn current_mode(&self) -> LexMode {
        *self.mode_stack.last().unwrap_or(&LexMode::Normal)
    }

    pub fn push_mode(&mut self, mode: LexMode) {
        self.mode_stack.push(mode);
    }

    pub fn pop_mode(&mut self) {
        if self.mode_stack.len() > 1 {
            self.mode_stack.pop();
        }
    }

    pub fn remaining(&self) -> &'a str {
        &self.src[self.pos..]
    }

    pub fn is_eof(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn peek_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn advance_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.pos += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    pub fn skip_whitespace(&mut self) {
        let mode = self.current_mode();
        while let Some(ch) = self.peek_char() {
            // Line comments starting with '#' or '//'
            if ch == '#' || (ch == '/' && self.remaining().starts_with("//")) {
                while let Some(c) = self.advance_char() {
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            }

            match mode {
                LexMode::Normal => {
                    if ch == ' ' || ch == '\t' || ch == '\r' {
                        self.advance_char();
                    } else {
                        break;
                    }
                }
                LexMode::Expr | LexMode::ExprBlock => {
                    if ch == ' ' || ch == '\t' || ch == '\r' {
                        self.advance_char();
                    } else {
                        break;
                    }
                }
                LexMode::ExprMl => {
                    if ch.is_whitespace() {
                        self.advance_char();
                    } else {
                        break;
                    }
                }
            }
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        let start_pos = self.pos;
        let start_line = self.line;
        let start_col = self.col;

        if self.is_eof() {
            return Token::new(
                TokenKind::Eof,
                Span::new(start_pos, start_pos, start_line, start_col),
                "",
            );
        }

        let rem = self.remaining();

        // Newline token in Normal or Expr modes
        if rem.starts_with('\n') {
            self.advance_char();
            let span = Span::new(start_pos, self.pos, start_line, start_col);
            return Token::new(TokenKind::Newline, span, "\n");
        }

        // Two-character operators & tokens
        let two_char = if rem.len() >= 2 { &rem[..2] } else { "" };
        let kind = match two_char {
            "==" => Some((TokenKind::EqEq, 2)),
            "!=" => Some((TokenKind::NotEq, 2)),
            "<=" => Some((TokenKind::LtEq, 2)),
            ">=" => Some((TokenKind::GtEq, 2)),
            "+=" => Some((TokenKind::PlusEq, 2)),
            "-=" => Some((TokenKind::MinusEq, 2)),
            "*=" => Some((TokenKind::StarEq, 2)),
            "/=" => Some((TokenKind::SlashEq, 2)),
            "%=" => Some((TokenKind::RemEq, 2)),
            "->" => Some((TokenKind::Arrow, 2)),
            "=>" => Some((TokenKind::FatArrow, 2)),
            ".." => Some((TokenKind::DotDot, 2)),
            _ => None,
        };

        if let Some((k, len)) = kind {
            for _ in 0..len {
                self.advance_char();
            }
            let span = Span::new(start_pos, self.pos, start_line, start_col);
            return Token::new(k, span, &self.src[start_pos..self.pos]);
        }

        // Single-character tokens
        let first = rem.chars().next().unwrap();
        let single_kind = match first {
            '+' => Some(TokenKind::Plus),
            '-' => Some(TokenKind::Minus),
            '*' => Some(TokenKind::Star),
            '/' => Some(TokenKind::Slash),
            '%' => Some(TokenKind::Rem),
            '=' => Some(TokenKind::Eq),
            '<' => Some(TokenKind::Lt),
            '>' => Some(TokenKind::Gt),
            '.' => Some(TokenKind::Dot),
            '|' => Some(TokenKind::Pipe),
            '(' => Some(TokenKind::LParen),
            ')' => Some(TokenKind::RParen),
            '[' => Some(TokenKind::LBracket),
            ']' => Some(TokenKind::RBracket),
            '{' => Some(TokenKind::LBrace),
            '}' => Some(TokenKind::RBrace),
            ',' => Some(TokenKind::Comma),
            ';' => Some(TokenKind::Semicolon),
            ':' => {
                // If followed by an ident char, it's an atom :tag!
                if rem.len() > 1
                    && (rem.as_bytes()[1].is_ascii_alphabetic() || rem.as_bytes()[1] == b'_')
                {
                    None
                } else {
                    Some(TokenKind::Colon)
                }
            }
            _ => None,
        };

        if let Some(k) = single_kind {
            self.advance_char();
            let span = Span::new(start_pos, self.pos, start_line, start_col);
            return Token::new(k, span, &self.src[start_pos..self.pos]);
        }

        // Atom :name
        if first == ':' {
            self.advance_char(); // skip ':'
            let atom_start = self.pos;
            while let Some(c) = self.peek_char() {
                if c.is_alphanumeric() || c == '_' {
                    self.advance_char();
                } else {
                    break;
                }
            }
            let atom_name = self.src[atom_start..self.pos].to_string();
            let span = Span::new(start_pos, self.pos, start_line, start_col);
            return Token::new(
                TokenKind::Atom(atom_name),
                span,
                &self.src[start_pos..self.pos],
            );
        }

        // String literals
        if first == '"' || first == '\'' {
            let quote = first;
            self.advance_char(); // skip quote
            let mut s = String::new();
            while let Some(c) = self.peek_char() {
                if c == quote {
                    self.advance_char(); // consume quote
                    break;
                } else if c == '\\' {
                    self.advance_char();
                    if let Some(escaped) = self.advance_char() {
                        match escaped {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            'r' => s.push('\r'),
                            '\\' => s.push('\\'),
                            '\'' => s.push('\''),
                            '"' => s.push('"'),
                            other => s.push(other),
                        }
                    }
                } else {
                    s.push(c);
                    self.advance_char();
                }
            }
            let span = Span::new(start_pos, self.pos, start_line, start_col);
            return Token::new(TokenKind::String(s), span, &self.src[start_pos..self.pos]);
        }

        // Numbers (Int or Float)
        if first.is_ascii_digit() {
            let mut is_float = false;
            while let Some(c) = self.peek_char() {
                if c.is_ascii_digit() || c == '_' {
                    self.advance_char();
                } else if c == '.' && !is_float {
                    // Check if followed by digit (so 1..10 is not float 1.)
                    let after = &self.src[self.pos + 1..];
                    if after.starts_with('.') {
                        // DotDot range
                        break;
                    }
                    if let Some(next_c) = after.chars().next() {
                        if next_c.is_ascii_digit() {
                            is_float = true;
                            self.advance_char();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            let text = &self.src[start_pos..self.pos];
            let clean: String = text.chars().filter(|c| *c != '_').collect();
            let span = Span::new(start_pos, self.pos, start_line, start_col);

            if is_float {
                let val: f64 = clean.parse().unwrap_or(0.0);
                return Token::new(TokenKind::Float(val.to_bits()), span, text);
            } else {
                let val: i64 = clean.parse().unwrap_or(0);
                return Token::new(TokenKind::Int(val), span, text);
            }
        }

        // Identifiers & Keywords
        if first.is_alphabetic() || first == '_' {
            while let Some(c) = self.peek_char() {
                if c.is_alphanumeric() || c == '_' {
                    self.advance_char();
                } else {
                    break;
                }
            }

            let text = &self.src[start_pos..self.pos];
            let kind = match text {
                "fn" => TokenKind::Fn,
                "struct" => TokenKind::Struct,
                "enum" => TokenKind::Enum,
                "protocol" => TokenKind::Protocol,
                "impl" => TokenKind::Impl,
                "import" => TokenKind::Import,
                "let" => TokenKind::Let,
                "const" => TokenKind::Const,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "while" => TokenKind::While,
                "loop" => TokenKind::Loop,
                "for" => TokenKind::For,
                "break" => TokenKind::Break,
                "continue" => TokenKind::Continue,
                "return" => TokenKind::Return,
                "match" => TokenKind::Match,
                "spawn" => TokenKind::Spawn,
                "send" => TokenKind::Send,
                "receive" => TokenKind::Receive,
                "and" => TokenKind::And,
                "or" => TokenKind::Or,
                "not" => TokenKind::Not,
                "true" => TokenKind::True,
                "false" => TokenKind::False,
                "unit" => TokenKind::Unit,
                "nil" => TokenKind::Nil,
                other => TokenKind::Ident(other.to_string()),
            };

            let span = Span::new(start_pos, self.pos, start_line, start_col);
            return Token::new(kind, span, text);
        }

        // Unknown character fallback
        self.advance_char();
        let span = Span::new(start_pos, self.pos, start_line, start_col);
        Token::new(TokenKind::Eof, span, &self.src[start_pos..self.pos])
    }
}
