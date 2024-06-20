type Pos = u32;

#[derive(Debug, Eq, PartialEq)]
pub enum Token {
    LParen,
    RParen,
    Symbol,
    Number,
    Color,
    String,
    Ident,
    Error,
}

#[derive(Debug)]
pub struct Lexeme {
    pub token: Token,
    pub text: String,
}

#[derive(Debug)]
pub struct Lexer<'a> {
    input: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    offset: Pos,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.char_indices().peekable(),
            offset: 0,
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }

    fn next_char(&mut self) -> Option<char> {
        self.chars.next().map(|(off, c)| {
            self.offset = off as Pos;
            c
        })
    }

    fn consume(&mut self) {
        self.next_char();
    }

    fn consume_if<P: FnOnce(char) -> bool>(&mut self, pred: P) -> Option<char> {
        self.chars.next_if(|(_, c)| pred(*c)).map(|(ix, c)| {
            self.offset = ix as Pos;
            c
        })
    }

    fn consume_while<P: FnMut(bool, char) -> bool>(&mut self, mut pred: P) -> usize {
        let start = self.pos();

        while let Some((ix, _)) = self.chars.next_if(|(ix, c)| pred(*ix as Pos > start, *c)) {
            self.offset = ix as Pos;
        }

        (self.pos() - start) as usize
    }

    fn skip_whitespace_and_comments(&mut self) {
        while let Some(c) = self.peek_char() {
            match c {
                ';' => self.skip_line(),
                c if c.is_whitespace() => self.consume(),
                _ => break,
            }
        }
    }

    fn pos(&self) -> Pos {
        self.offset
    }

    /// Skip to the next line.
    fn skip_line(&mut self) {
        while self.consume_if(|c| c != '\n').is_some() {}
    }

    /// Consume an identifier.
    fn consume_ident(&mut self, is_tail: bool) -> bool {
        self.consume_while(|mut consumed, c| {
            if c.is_ascii_alphabetic() {
                return true;
            }

            consumed = consumed || is_tail;

            if !consumed {
                return false;
            }

            c.is_ascii_digit() || "-_!?".contains(c)
        }) > 0
    }

    fn consume_number(&mut self) {
        let mut dot = false;

        self.consume_while(|_, c| {
            if c.is_ascii_digit() {
                return true;
            }

            if !dot && c == '.' {
                dot = true;
                return true;
            }

            c.is_ascii_digit()
        });
    }

    fn consume_color(&mut self) -> bool {
        self.consume_while(|_, c| c.is_ascii_hexdigit()) > 0
    }

    fn consume_string(&mut self) -> bool {
        let mut done = false;
        let mut prev = '"';
        self.consume_while(|consumed, c| {
            if done {
                return false;
            }

            done = c == '"' && (consumed && prev != '\\');
            prev = c;
            true
        }) > 0
    }

    fn text(&self, start: Pos, end: Pos) -> String {
        let slice = self.input.get(start as usize..=end as usize).unwrap();
        String::from(slice)
    }

    /// Construct a lexeme.
    fn lexeme(&self, start: Pos, token: Token) -> Lexeme {
        let end = self.offset;
        Lexeme {
            token,
            text: self.text(start, end),
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Lexeme;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace_and_comments();

        if let Some(c) = self.next_char() {
            let start = self.pos();
            let tok = match c {
                '(' => Token::LParen,
                ')' => Token::RParen,
                ':' => {
                    if self.consume_ident(false) {
                        Token::Symbol
                    } else {
                        Token::Error
                    }
                }
                '#' => {
                    if self.consume_color() {
                        Token::Color
                    } else {
                        Token::Error
                    }
                }
                '"' => {
                    if self.consume_string() {
                        Token::String
                    } else {
                        Token::Error
                    }
                }

                '-' => {
                    self.consume_number();
                    Token::Number
                }

                _ if c.is_ascii_digit() => {
                    self.consume_number();
                    Token::Number
                }

                _ if c.is_ascii_alphabetic() => {
                    if self.consume_ident(true) {
                        Token::Ident
                    } else {
                        Token::Error
                    }
                }

                _ => Token::Error,
            };
            Some(self.lexeme(start, tok))
        } else {
            None
        }
    }
}

#[cfg(test)]
macro_rules! lexer_next {
    ($lexer:ident, $token:expr, $text:expr) => {
        let result = $lexer.next();
        assert!(result.is_some());

        let result = result.unwrap();
        assert_eq!($token, result.token);
        assert_eq!($text, result.text);
    };
}

#[test]
fn test_lex_basic() {
    let input = "(:symbol 0.1 #6600ff \"foo.\\\"bar\"))";
    let mut lexer = Lexer::new(input);
    lexer_next!(lexer, Token::LParen, "(");
    lexer_next!(lexer, Token::Symbol, ":symbol");
    lexer_next!(lexer, Token::Number, "0.1");
    lexer_next!(lexer, Token::Color, "#6600ff");
    lexer_next!(lexer, Token::String, "\"foo.\\\"bar\"");
    lexer_next!(lexer, Token::RParen, ")");
    lexer_next!(lexer, Token::RParen, ")");
    assert!(lexer.next().is_none());
}

#[test]
fn test_lex_leading_space() {
    let input = "         :symbol1 :symbol-2";
    let mut lexer = Lexer::new(input);
    lexer_next!(lexer, Token::Symbol, ":symbol1");
    lexer_next!(lexer, Token::Symbol, ":symbol-2");
}

#[test]
fn test_lex_trailing_space() {
    let input = ":symbol   ";
    let mut lexer = Lexer::new(input);
    lexer_next!(lexer, Token::Symbol, ":symbol");
}

#[test]
fn test_lex_leading_newline() {
    let input = "    \n     :symbol";
    let mut lexer = Lexer::new(input);
    lexer_next!(lexer, Token::Symbol, ":symbol");
}

#[test]
fn test_lex_leading_comment() {
    let input = "    ;; foo comment\n     :symbol";
    let mut lexer = Lexer::new(input);
    lexer_next!(lexer, Token::Symbol, ":symbol");
}
