use crate::{
    diagnostics::Diagnostic,
    token::{Span, Token, TokenKind},
};

pub struct Lexer<'a> {
    source: &'a str,
    file: &'a str,
    chars: Vec<char>,
    current: usize,
    byte: usize,
    line: usize,
    column: usize,
    line_has_token: bool,
    tokens: Vec<Token>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, file: &'a str) -> Self {
        Self {
            source,
            file,
            chars: source.chars().collect(),
            current: 0,
            byte: 0,
            line: 1,
            column: 1,
            line_has_token: false,
            tokens: vec![],
        }
    }

    pub fn scan(mut self) -> Result<Vec<Token>, Diagnostic> {
        while !self.done() {
            let start = self.mark();
            let c = self.advance();
            match c {
                ' ' | '\t' | '\r' => {}
                '\n' => {
                    self.tokens.push(Token {
                        kind: TokenKind::Newline,
                        span: start,
                    });
                    self.line += 1;
                    self.column = 1;
                    self.line_has_token = false;
                }
                '/' if self.peek() == Some('/') && !self.line_has_token => {
                    while self.peek().is_some() && self.peek() != Some('\n') {
                        self.advance();
                    }
                }
                '/' if self.peek() == Some('/') => {
                    self.advance();
                    self.push(TokenKind::SlashSlash, start);
                }
                '/' if self.peek() == Some('=') => {
                    self.advance();
                    self.push(TokenKind::SlashEqual, start);
                }
                '/' => self.push(TokenKind::Slash, start),
                '+' if self.peek() == Some('=') => {
                    self.advance();
                    self.push(TokenKind::PlusEqual, start);
                }
                '+' => self.push(TokenKind::Plus, start),
                '-' if self.peek() == Some('=') => {
                    self.advance();
                    self.push(TokenKind::MinusEqual, start);
                }
                '-' => self.push(TokenKind::Minus, start),
                '*' if self.peek() == Some('*') => {
                    self.advance();
                    self.push(TokenKind::StarStar, start);
                }
                '*' if self.peek() == Some('=') => {
                    self.advance();
                    self.push(TokenKind::StarEqual, start);
                }
                '*' => self.push(TokenKind::Star, start),
                '%' => self.push(TokenKind::Percent, start),
                '=' if self.peek() == Some('=') => {
                    self.advance();
                    self.push(TokenKind::EqualEqual, start);
                }
                '=' => self.push(TokenKind::Equal, start),
                '!' if self.peek() == Some('=') => {
                    self.advance();
                    self.push(TokenKind::BangEqual, start);
                }
                '<' if self.peek() == Some('=') => {
                    self.advance();
                    self.push(TokenKind::LessEqual, start);
                }
                '<' => self.push(TokenKind::Less, start),
                '>' if self.peek() == Some('=') => {
                    self.advance();
                    self.push(TokenKind::GreaterEqual, start);
                }
                '>' => self.push(TokenKind::Greater, start),
                '(' => self.push(TokenKind::LeftParen, start),
                ')' => self.push(TokenKind::RightParen, start),
                '{' => self.push(TokenKind::LeftBrace, start),
                '}' => self.push(TokenKind::RightBrace, start),
                '[' => self.push(TokenKind::LeftBracket, start),
                ']' => self.push(TokenKind::RightBracket, start),
                ',' => self.push(TokenKind::Comma, start),
                ':' => self.push(TokenKind::Colon, start),
                ';' => self.push(TokenKind::Semicolon, start),
                '.' if self.peek() == Some('.') => {
                    self.advance();
                    self.push(TokenKind::DotDot, start);
                }
                '.' => self.push(TokenKind::Dot, start),
                '"' => self.string(start)?,
                c if c.is_ascii_digit() => self.number(start)?,
                c if is_ident_start(c) => self.identifier(start),
                _ => {
                    return Err(self.error(
                        start,
                        "unexpected character",
                        format!("`{c}` is not valid Shine syntax."),
                        "Remove it or replace it with a valid token.",
                    ))
                }
            }
        }
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: self.mark(),
        });
        Ok(self.tokens)
    }

    fn string(&mut self, start: Span) -> Result<(), Diagnostic> {
        let triple = self.peek() == Some('"') && self.peek_next() == Some('"');
        if triple {
            self.advance();
            self.advance();
        }
        let mut value = String::new();
        loop {
            if self.done() {
                return Err(self.error(
                    start,
                    "unterminated string",
                    "The string reaches the end of the file.",
                    "Add a closing quote.",
                ));
            }
            if triple
                && self.peek() == Some('"')
                && self.peek_next() == Some('"')
                && self.peek_n(2) == Some('"')
            {
                self.advance();
                self.advance();
                self.advance();
                break;
            }
            if !triple && self.peek() == Some('"') {
                self.advance();
                break;
            }
            let c = self.advance();
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            }
            if c == '\\' {
                let escaped = match self.advance() {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '"' => '"',
                    '\\' => '\\',
                    other => other,
                };
                value.push(escaped);
            } else {
                value.push(c);
            }
        }
        self.push(TokenKind::String(value), start);
        Ok(())
    }

    fn number(&mut self, start: Span) -> Result<(), Diagnostic> {
        let begin = start.start;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '_') {
            self.advance();
        }
        let mut float = false;
        if self.peek() == Some('.')
            && self.peek_next() != Some('.')
            && matches!(self.peek_next(), Some(c) if c.is_ascii_digit())
        {
            float = true;
            self.advance();
            while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '_') {
                self.advance();
            }
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            float = true;
            self.advance();
            if matches!(self.peek(), Some('+' | '-')) {
                self.advance();
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '_') {
                self.advance();
            }
        }
        let raw = self.source[begin..self.byte].replace('_', "");
        let kind: Result<TokenKind, ()> = if float {
            raw.parse::<f64>().map(TokenKind::Float).map_err(|_| ())
        } else {
            raw.parse::<i64>().map(TokenKind::Int).map_err(|_| ())
        };
        match kind {
            Ok(kind) => {
                self.push(kind, start);
                Ok(())
            }
            Err(_) => Err(self.error(
                start,
                "invalid number",
                format!("`{raw}` is not a supported number."),
                "Check the number's digits and exponent.",
            )),
        }
    }

    fn identifier(&mut self, start: Span) {
        while matches!(self.peek(), Some(c) if is_ident_continue(c)) {
            self.advance();
        }
        let text = &self.source[start.start..self.byte];
        let kind = match text {
            "import" => TokenKind::Import,
            "from" => TokenKind::From,
            "as" => TokenKind::As,
            "export" => TokenKind::Export,
            "class" => TokenKind::Class,
            "private" => TokenKind::Private,
            "fn" => TokenKind::Fn,
            "const" => TokenKind::Const,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "loop" => TokenKind::Loop,
            "in" => TokenKind::In,
            "step" => TokenKind::Step,
            "return" => TokenKind::Return,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "none" => TokenKind::None,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            _ => TokenKind::Identifier(text.to_string()),
        };
        self.push(kind, start);
    }

    fn push(&mut self, kind: TokenKind, mut span: Span) {
        span.length = self.byte.saturating_sub(span.start);
        self.line_has_token = true;
        self.tokens.push(Token { kind, span });
    }
    fn error(
        &self,
        span: Span,
        message: impl Into<String>,
        explanation: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Diagnostic {
        Diagnostic::at(
            "Syntax Error",
            message,
            self.file,
            self.source,
            span,
            explanation,
            suggestion,
        )
    }
    fn mark(&self) -> Span {
        Span {
            start: self.byte,
            length: 1,
            line: self.line,
            column: self.column,
        }
    }
    fn done(&self) -> bool {
        self.current >= self.chars.len()
    }
    fn peek(&self) -> Option<char> {
        self.chars.get(self.current).copied()
    }
    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.current + 1).copied()
    }
    fn peek_n(&self, n: usize) -> Option<char> {
        self.chars.get(self.current + n).copied()
    }
    fn advance(&mut self) -> char {
        let c = self.chars[self.current];
        self.current += 1;
        self.byte += c.len_utf8();
        self.column += 1;
        c
    }
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}
fn is_ident_continue(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit()
}
