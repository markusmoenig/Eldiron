// Based on https://github.com/ceronman/loxido/blob/master/src/scanner.rs
// Licensed under the MIT license of Manuel Cerón.

use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Eq, PartialEq, Debug, Copy, Clone, Hash)]
pub enum TokenType {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Dollar,

    LineFeed,
    Space,
    Quotation,
    Unknown,
    SingeLineComment,
    HexColor,

    // One or two character tokens.
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Literals.
    Identifier,
    String,
    Number,

    // Keywords.
    And,
    Class,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Let,
    While,

    Error,
    Eof,
}

#[derive(Copy, Clone, Debug)]
pub struct Token<'sourcecode> {
    pub kind: TokenType,
    pub line: usize,
    pub lexeme: &'sourcecode str,
}

#[allow(dead_code)]
impl<'sourcecode> Token<'sourcecode> {
    pub fn synthetic(text: &'sourcecode str) -> Token<'sourcecode> {
        Token {
            kind: TokenType::Error,
            lexeme: text,
            line: 0,
        }
    }
}

#[allow(dead_code)]
pub struct Scanner<'sourcecode> {
    keywords: HashMap<&'static str, TokenType>,
    code: &'sourcecode str,
    start: usize,
    current: usize,
    line: usize,
}

#[allow(dead_code)]
impl<'sourcecode> Scanner<'sourcecode> {
    pub fn new(code: &'sourcecode str) -> Scanner {
        let mut keywords = HashMap::with_capacity(16);
        keywords.insert("and", TokenType::And);
        keywords.insert("class", TokenType::Class);
        keywords.insert("else", TokenType::Else);
        keywords.insert("false", TokenType::False);
        keywords.insert("for", TokenType::For);
        keywords.insert("fun", TokenType::Fun);
        keywords.insert("if", TokenType::If);
        keywords.insert("nil", TokenType::Nil);
        keywords.insert("or", TokenType::Or);
        keywords.insert("print", TokenType::Print);
        keywords.insert("return", TokenType::Return);
        keywords.insert("super", TokenType::Super);
        keywords.insert("this", TokenType::This);
        keywords.insert("true", TokenType::True);
        keywords.insert("let", TokenType::Let);
        keywords.insert("while", TokenType::While);

        Scanner {
            keywords,
            code,
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn scan_token(&mut self) -> Token<'sourcecode> {
        //self.skip_whitespace();
        self.start = self.current;
        if self.is_at_end() {
            return self.make_token(TokenType::Eof);
        }

        match self.advance() {
            b' ' => self.make_token(TokenType::Space),
            b'\n' => self.make_token(TokenType::LineFeed),
            b'(' => self.make_token(TokenType::LeftParen),
            b')' => self.make_token(TokenType::RightParen),
            b'{' => self.make_token(TokenType::LeftBrace),
            b'}' => self.make_token(TokenType::RightBrace),
            b'$' => self.make_token(TokenType::Dollar),
            b';' => self.make_token(TokenType::Semicolon),
            b',' => self.make_token(TokenType::Comma),
            b'.' => self.make_token(TokenType::Dot),
            b'-' => self.make_token(TokenType::Minus),
            b'+' => self.make_token(TokenType::Plus),
            b'/' if self.matches(b'/') => self.single_line_comment(),
            b'#' => self.hex_color(),
            b'/' => self.make_token(TokenType::Slash),
            b'*' => self.make_token(TokenType::Star),
            b'!' if self.matches(b'=') => self.make_token(TokenType::BangEqual),
            b'!' => self.make_token(TokenType::Bang),
            b'=' if self.matches(b'=') => self.make_token(TokenType::EqualEqual),
            b'=' => self.make_token(TokenType::Equal),
            b'<' if self.matches(b'=') => self.make_token(TokenType::LessEqual),
            b'<' => self.make_token(TokenType::Less),
            b'>' if self.matches(b'=') => self.make_token(TokenType::GreaterEqual),
            b'>' => self.make_token(TokenType::Greater),
            b'"' => self.string(),
            b'`' => self.string2(),
            c if is_digit(c) => self.number(),
            c if is_alpha(c) => self.identifier(),
            _ => self.make_token(TokenType::Unknown),//self.error_token("Unexpected character."),
        }
    }

    fn is_at_end(&self) -> bool {
        self.current == self.code.len()
    }

    fn lexeme(&self) -> &'sourcecode str {
        &self.code[self.start..self.current]
    }

    fn make_token(&self, kind: TokenType) -> Token<'sourcecode> {
        Token {
            kind,
            lexeme: self.lexeme(),
            line: self.line,
        }
    }

    fn peek(&self) -> u8 {
        if self.is_at_end() {
            0
        } else {
            self.code.as_bytes()[self.current]
        }
    }
    fn peek_next(&self) -> u8 {
        if self.current > self.code.len() - 2 {
            b'\0'
        } else {
            self.code.as_bytes()[self.current + 1]
        }
    }

    fn error_token(&self, message: &'static str) -> Token<'static> {
        Token {
            kind: TokenType::Error,
            lexeme: message,
            line: self.line,
        }
    }

    fn advance(&mut self) -> u8 {
        let char = self.peek();
        self.current += 1;
        char
    }

    fn matches(&mut self, expected: u8) -> bool {
        if self.is_at_end() || self.peek() != expected {
            false
        } else {
            self.current += 1;
            true
        }
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                b' ' | b'\r' | b'\t' => {
                    self.advance();
                }
                b'\n' => {
                    self.line += 1;
                    self.advance();
                }
                b'/' if self.peek_next() == b'/' => {
                    while self.peek() != b'\n' && !self.is_at_end() {
                        self.advance();
                    }
                }
                _ => return,
            }
        }
    }

    fn string(&mut self) -> Token<'sourcecode> {
        let b_current = self.current;

        while self.peek() != b'"' && !self.is_at_end() {
            if self.peek() == b'\n' {
                //self.line += 1;
                self.current = b_current;
                return self.make_token(TokenType::Quotation);
            }
            self.advance();
        }

        if self.is_at_end() {
            //self.error_token("Unterminated string.")
            self.current = b_current;
            self.make_token(TokenType::Quotation)
        } else {
            self.advance();
            self.make_token(TokenType::String)
        }
    }

    fn string2(&mut self) -> Token<'sourcecode> {
        let b_current = self.current;

        while self.peek() != b'`' && !self.is_at_end() {
            if self.peek() == b'\n' {
                //self.line += 1;
                self.current = b_current;
                return self.make_token(TokenType::Quotation);
            }
            self.advance();
        }

        if self.is_at_end() {
            //self.error_token("Unterminated string.")
            self.current = b_current;
            self.make_token(TokenType::Quotation)
        } else {
            self.advance();
            self.make_token(TokenType::String)
        }
    }

    fn number(&mut self) -> Token<'sourcecode> {
        while is_digit(self.peek()) {
            self.advance();
        }

        if self.peek() == b'.' && is_digit(self.peek_next()) {
            self.advance();
            while is_digit(self.peek()) {
                self.advance();
            }
        }

        self.make_token(TokenType::Number)
    }

    fn hex_color(&mut self) -> Token<'sourcecode> {
        while self.is_at_end() == false && self.peek() != b'\n' {
            self.advance();
        }
        self.make_token(TokenType::HexColor)
    }

    fn single_line_comment(&mut self) -> Token<'sourcecode> {
        while self.is_at_end() == false && self.peek() != b'\n' {
            self.advance();
        }
        self.make_token(TokenType::SingeLineComment)
    }

    fn identifier(&mut self) -> Token<'sourcecode> {
        while is_alpha(self.peek()) || is_digit(self.peek()) {
            self.advance();
        }
        self.make_token(self.identifier_type())
    }

    fn identifier_type(&self) -> TokenType {
        self.keywords
            .get(self.lexeme())
            .cloned()
            .unwrap_or(TokenType::Identifier)
    }
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn is_alpha(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}