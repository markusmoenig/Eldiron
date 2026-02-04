use rustc_hash::FxHashMap;

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
    Colon,
    TernaryOperator,
    Percent,

    LineFeed,
    Space,
    Quotation,
    Unknown,
    SingeLineComment,
    HexColor,

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
    IntegerNumber,
    FloatNumber,

    // Keywords.
    And,
    Else,
    False,
    For,
    Match,
    Fn,
    If,
    Void,
    Or,
    // Print,
    Return,
    True,
    While,
    Break,
    Export,
    Const,
    Struct,

    Let,
    Import,

    In,
    Out,
    Inout,

    Int,
    Int2,
    Int3,
    Int4,

    Float,
    Float2,
    Float3,
    Float4,

    Mat2,
    Mat3,
    Mat4,

    Error,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenType,
    pub line: usize,
    pub lexeme: String,
}

#[allow(dead_code)]
impl Token {
    pub fn synthetic(text: String) -> Token {
        Token {
            kind: TokenType::Error,
            lexeme: text,
            line: 0,
        }
    }
}

#[allow(dead_code)]
pub struct Scanner {
    keywords: FxHashMap<&'static str, TokenType>,
    code: String,
    start: usize,
    current: usize,
    line: usize,
}

#[allow(dead_code)]
impl Scanner {
    pub fn new(code: String) -> Scanner {
        let mut keywords = FxHashMap::default();
        keywords.insert("else", TokenType::Else);
        keywords.insert("false", TokenType::False);
        keywords.insert("for", TokenType::For);
        keywords.insert("match", TokenType::Match);
        keywords.insert("fn", TokenType::Fn);
        keywords.insert("if", TokenType::If);
        keywords.insert("void", TokenType::Void);
        // keywords.insert("print", TokenType::Print);
        keywords.insert("return", TokenType::Return);
        keywords.insert("true", TokenType::True);
        keywords.insert("while", TokenType::While);
        keywords.insert("break", TokenType::Break);

        keywords.insert("int", TokenType::Int);
        keywords.insert("ivec2", TokenType::Int2);
        keywords.insert("ivec3", TokenType::Int3);
        keywords.insert("ivec4", TokenType::Int4);

        keywords.insert("float", TokenType::Float);
        keywords.insert("vec2", TokenType::Float2);
        keywords.insert("vec3", TokenType::Float3);
        keywords.insert("vec4", TokenType::Float4);

        keywords.insert("mat2", TokenType::Mat2);
        keywords.insert("mat3", TokenType::Mat3);
        keywords.insert("mat4", TokenType::Mat4);

        keywords.insert("let", TokenType::Let);
        keywords.insert("import", TokenType::Import);

        keywords.insert("in", TokenType::In);
        keywords.insert("out", TokenType::Out);
        keywords.insert("inout", TokenType::Inout);

        Scanner {
            keywords,
            code,
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn scan_token(&mut self) -> Token {
        self.skip_whitespace();
        self.start = self.current;
        if self.is_at_end() {
            return self.make_token(TokenType::Eof);
        }

        match self.advance() {
            //b' ' => self.make_token(TokenType::Space),
            //b'\n' => self.make_token(TokenType::LineFeed),
            b'(' => self.make_token(TokenType::LeftParen),
            b'?' => self.make_token(TokenType::TernaryOperator),
            b')' => self.make_token(TokenType::RightParen),
            b'{' => self.make_token(TokenType::LeftBrace),
            b'}' => self.make_token(TokenType::RightBrace),
            b'$' => self.make_token(TokenType::Dollar),
            b';' => self.make_token(TokenType::Semicolon),
            b',' => self.make_token(TokenType::Comma),
            b'-' => self.make_token(TokenType::Minus),
            b'+' => self.make_token(TokenType::Plus),
            b'#' => self.single_line_comment(),
            b'/' => self.make_token(TokenType::Slash),
            b'*' => self.make_token(TokenType::Star),
            b':' => self.make_token(TokenType::Colon),
            b'%' => self.make_token(TokenType::Percent),
            b'!' if self.matches(b'=') => self.make_token(TokenType::BangEqual),
            b'!' => self.make_token(TokenType::Bang),
            b'&' if self.matches(b'&') => self.make_token(TokenType::And),
            b'|' if self.matches(b'|') => self.make_token(TokenType::Or),
            b'=' if self.matches(b'=') => self.make_token(TokenType::EqualEqual),
            b'=' => self.make_token(TokenType::Equal),
            b'<' if self.matches(b'=') => self.make_token(TokenType::LessEqual),
            b'<' => self.make_token(TokenType::Less),
            b'>' if self.matches(b'=') => self.make_token(TokenType::GreaterEqual),
            b'>' => self.make_token(TokenType::Greater),
            b'"' => self.string(),
            b'`' => self.string2(),
            b'.' if is_digit(self.peek()) => self.float_with_starting_dot(),
            c if is_digit(c) => self.number(),
            b'.' => self.make_token(TokenType::Dot),
            c if is_alpha(c) => self.identifier(),
            _ => self.make_token(TokenType::Unknown), //self.error_token("Unexpected character."),
        }
    }

    fn is_at_end(&self) -> bool {
        self.current == self.code.len()
    }

    fn lexeme(&self) -> String {
        self.code[self.start..self.current].to_string()
    }

    fn make_token(&self, kind: TokenType) -> Token {
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

    fn error_token(&self, message: String) -> Token {
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
                    // Single-line comment
                    while !self.is_at_end() && self.peek() != b'\n' {
                        self.advance();
                    }
                }
                b'/' if self.peek_next() == b'*' => {
                    // Start of a multi-line comment
                    self.advance(); // Advance past '/'
                    self.advance(); // Advance past '*'
                    while !self.is_at_end() {
                        if self.peek() == b'*' && self.peek_next() == b'/' {
                            self.advance(); // Advance past '*'
                            self.advance(); // Advance past '/'
                            break;
                        }
                        if self.peek() == b'\n' {
                            self.line += 1;
                        }
                        self.advance();
                    }
                }
                _ => return,
            }
        }
    }

    fn string(&mut self) -> Token {
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
            //self.error_token("Unterminated string.");
            self.current = b_current;
            self.make_token(TokenType::Quotation)
        } else {
            self.advance();
            self.make_token(TokenType::String)
        }
    }

    fn string2(&mut self) -> Token {
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

    fn number(&mut self) -> Token {
        let mut lexeme: Vec<u8> = Vec::new();

        let code = self.lexeme();
        if let Some(last) = code.chars().last() {
            lexeme.push(last as u8);
        }

        while is_digit(self.peek()) {
            lexeme.push(self.advance());
        }

        let mut is_float = false;
        if self.peek() == b'.' && is_digit(self.peek_next()) {
            is_float = true;
            lexeme.push(self.advance());
            while is_digit(self.peek()) {
                lexeme.push(self.advance());
            }
        } else if self.peek() == b'.' && !is_digit(self.peek_next()) {
            is_float = true;
            lexeme.push(self.advance());
            lexeme.push(b'0');
        }

        if is_float {
            Token {
                kind: TokenType::FloatNumber,
                lexeme: lexeme.iter().map(|&c| c as char).collect(),
                line: self.line,
            }
        } else {
            Token {
                kind: TokenType::IntegerNumber,
                lexeme: lexeme.iter().map(|&c| c as char).collect(),
                line: self.line,
            }
        }
    }

    fn float_with_starting_dot(&mut self) -> Token {
        let mut lexeme: Vec<u8> = Vec::new();

        lexeme.push(b'0');
        lexeme.push(b'.');

        while is_digit(self.peek()) {
            lexeme.push(self.advance());
        }

        Token {
            kind: TokenType::FloatNumber,
            lexeme: lexeme.iter().map(|&c| c as char).collect(),
            line: self.line,
        }
    }

    fn hex_color(&mut self) -> Token {
        while !self.is_at_end() && self.peek() != b'\n' {
            self.advance();
        }
        self.make_token(TokenType::HexColor)
    }

    fn single_line_comment(&mut self) -> Token {
        while !self.is_at_end() && self.peek() != b'\n' {
            self.advance();
        }
        self.make_token(TokenType::SingeLineComment)
    }

    fn identifier(&mut self) -> Token {
        while is_alpha(self.peek()) || is_digit(self.peek()) {
            self.advance();
        }
        self.make_token(self.identifier_type())
    }

    fn identifier_type(&self) -> TokenType {
        self.keywords
            .get(self.lexeme().as_str())
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
