use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    io::{BufReader, Read},
    ops::AddAssign,
};

pub static TEXTLEN: usize = 512; // Maximum lenght of symbols in input

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    Amper,         // &
    Plus,          // +
    Minus,         // -
    Star,          // *
    Slash,         // /
    Assign,        // =
    EQ,            // ==
    NE,            // !=
    LT,            // <
    GT,            // >
    LE,            // <=
    GE,            // >=
    LogAnd,        // &&
    Ident(String), // identifier
    LeftBrace,     // {
    RightBrace,    // }
    LeftParen,     // (
    RightParen,    // )
    Semi,          // ;

    // Keywords
    Else,          // else
    For,           // for
    If,            // if
    Char,          // char
    Int,           // int
    Long,          // long
    Return,        // return
    Void,          // void
    While,         // while

    // Literals
    IntLit(i64),   // Integer literal
}

impl TokenType {
    pub fn is_type(&self) -> bool {
        matches!(self, TokenType::Void|TokenType::Char|TokenType::Int|TokenType::Long)
    }
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

static KEYWORDS: &[(&str, TokenType)] = &[
    ("char", TokenType::Char),
    ("else", TokenType::Else),
    ("for", TokenType::For),
    ("if", TokenType::If),
    ("int", TokenType::Int),
    ("long", TokenType::Long),
    ("return", TokenType::Return),
    ("void", TokenType::Void),
    ("while", TokenType::While),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub(crate) ttype: TokenType,
    pub(crate) line: usize,
    pub(crate) col: usize,
}

impl Token {
    pub fn is_type(&self) -> bool {
        self.ttype.is_type()
    }
}

fn is_valid_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[derive(Debug)]
pub struct Scanner<T> {
    buffer: RefCell<BufReader<T>>,
    putback_char: RefCell<Option<char>>,
    putback_token: RefCell<Option<Token>>,
    keyword_map: HashMap<&'static str, TokenType>,
    line: RefCell<usize>,
    col: RefCell<usize>,
}

impl<T> Scanner<T>
where T: Read
{
    pub fn new(reader: T) -> Self {
        Scanner {
            buffer: BufReader::new(reader).into(),
            putback_char: None.into(),
            putback_token: None.into(),
            line: 1.into(),
            col: 1.into(),
            keyword_map: KEYWORDS.iter().cloned().collect(),
        }
    }

    pub fn is_eof(&self) -> bool {
        if self.putback_token.borrow().is_some() {
            false
        } else if let Some(c) = self.skip() {
            self.putback_char(c);
            false
        } else {
            true
        }
    }

    pub fn scan(&self) -> Option<Token> {
        if let Some(t) = self.putback_token.borrow_mut().take() {
            Some(t)
        } else {
            let (line, col) = (self.get_line(), self.get_col());
            let ttype = match self.skip()? {
                '(' => TokenType::LeftParen,
                ')' => TokenType::RightParen,
                '{' => TokenType::LeftBrace,
                '}' => TokenType::RightBrace,
                '+' => TokenType::Plus,
                '-' => TokenType::Minus,
                '*' => TokenType::Star,
                '/' => TokenType::Slash,
                ';' => TokenType::Semi,
                '=' => {
                    if let Some('=') = self.peek() {
                        self.clear_putback_char();
                        TokenType::EQ
                    } else {
                        TokenType::Assign
                    }
                },
                '<' => {
                    if let Some('=') = self.peek() {
                        self.clear_putback_char();
                        TokenType::LE
                    } else {
                        TokenType::LT
                    }
                },
                '>' => {
                    if let Some('=') = self.peek() {
                        self.clear_putback_char();
                        TokenType::GE
                    } else {
                        TokenType::GT
                    }
                },
                '!' => {
                    match self.peek() {
                        Some('=') => {
                            self.clear_putback_char();
                            TokenType::NE
                        },
                        Some(c) => self.fatal_extra("Unrecognised character", c),
                        None => self.fatal("Found EOF while parsing expression")
                    }
                },
                '&' => {
                    if let Some('&') = self.peek() {
                        self.clear_putback_char();
                        TokenType::LogAnd
                    } else {
                        TokenType::Amper
                    }
                },
                c if c.is_ascii_digit() => {
                    TokenType::IntLit(self.scan_int(c))
                }
                c if c.is_ascii_alphabetic() || c == '_' => {
                    self.putback_char(c);
                    let ident = self.scan_ident(TEXTLEN);

                    if let Some(t) = self.keyword(&ident) {
                        t
                    } else {
                        TokenType::Ident(ident)
                    }
                }
                c => {
                    panic!("Unrecognised character '{}' on line {}", c, self.line.borrow());
                }
            };

            Some(Token { ttype, line, col })
        }
    }

    pub fn get_line(&self) -> usize {
        *self.line.borrow()
    }

    pub fn get_col(&self) -> usize {
        *self.col.borrow()
    }

    // PUTBACK FUNCTIONS
    //
    //  Characters are never seen outside the scanner. Only the
    //  scanner itself can put chars back, the function is not
    //  public.
    fn putback_char(&self, c: char) {
        self.putback_char.borrow_mut().replace(c);
    } 

    //  Tokens can be put back by users of the Scanner struct,
    //  thus this needs to be public.
    pub fn putback_token(&self, t: Token) {
        self.putback_token.borrow_mut().replace(t);
    } 

    fn next(&self) -> Option<char> {
        if let Some(c) = self.putback_char.borrow_mut().take() {
            Some(c)
        } else {
            let mut buf = [0; 1];

            match self.buffer.borrow_mut().read_exact(&mut buf) {
                Ok(_) => {
                    if buf[0] == b'\n' {
                        self.line.borrow_mut().add_assign(1);
                        *self.col.borrow_mut() = 1;
                    } else {
                        self.col.borrow_mut().add_assign(1);
                    }
                    Some(buf[0] as char)
                }
                Err(_) => None,
            }
        }
    }

    fn peek(&self) -> Option<char> {
        if let Some(c) = self.next() {
            self.putback_char(c);
            Some(c)
        } else {
            None
        }
    }

    fn clear_putback_char(&self) {
        _ = self.putback_char.take();
    }

    fn skip(&self) -> Option<char> {
        while let Some(c) = self.next() {
            if !c.is_whitespace() {
                return Some(c);
            }
        }

        None
    }

    fn scan_int(&self, first: char) -> i64 {
        let mut value = first.to_digit(10).unwrap() as i64;

        while let Some(c) = self.next() {
            if c.is_ascii_digit() {
                value = value * 10 + c.to_digit(10).unwrap() as i64;
            } else {
                self.putback_char(c);
                break;
            }
        }

        value
    }

    fn scan_ident(&self, lim: usize) -> String {
        let mut buffer = String::new();

        while let Some(c) = self.next() {
            if is_valid_ident_char(c) {
                if lim == buffer.len() {
                    panic!("identifier too long in line {}", self.get_line());
                }

                buffer.push(c);
            } else {
                self.putback_char(c);
                break;
            }
        }

        buffer
    }

    fn keyword(&self, s: &str) -> Option<TokenType> {
        self.keyword_map.get(s).cloned()
    }

    // Consume a token. If it's the end of the file, return 'false'.
    // If a token was found and matches the expected one, return 'true'.
    // Else panic
    pub fn if_not_eof_matches<F>(&self, expected: F, expected_string: &str) -> Option<Token>
        where F: Fn(&TokenType) -> bool
    {
        if let Some(tok) = self.scan() {
            if expected(&tok.ttype) {
                Some(tok)
            } else {
                panic!("Expected {} on line {}; found {} instead", expected_string, self.line.borrow(), tok.ttype);
            }
        } else {
            None
        }
    }

    // Consume a token. If it matches the expected one. Else panic
    pub fn matches(&self, expected: TokenType, expected_string: &str) -> Token {
        if let Some(tok) = self.if_not_eof_matches(|tok| *tok == expected, expected_string) {
            tok
        } else {
            panic!("End of input while expecting {}", expected_string);
        }
    }

    pub fn ident(&self, expected_string: Option<&str>) -> String {
        let exp = expected_string.unwrap_or("identifier");
        let res = self.if_not_eof_matches(|tok| matches!(tok, &TokenType::Ident(_)), exp);
        match res {
            Some(Token { ttype: TokenType::Ident(name), .. }) => name,
            None => panic!("End of input while expecting an identifier"),
            _ => unreachable!()
        }
    }

    pub fn maybe_token(&self, token: TokenType) -> bool {
        match self.scan() {
            Some(tok) => {
                if tok.ttype == token {
                    true
                } else {
                    self.putback_token(tok);
                    false
                }
            },
            None => panic!("End of input while expecting {token}"),
        }
    }

    pub fn semi(&self) -> Token {
        self.matches(TokenType::Semi, ";")
    }

    pub fn lbrace(&self) -> Token {
        self.matches(TokenType::LeftBrace, "{")
    }

    pub fn lparen(&self) -> Token {
        self.matches(TokenType::LeftParen, "(")
    }

    pub fn rparen(&self) -> Token {
        self.matches(TokenType::RightParen, ")")
    }

    pub fn fatal(&self, error_msg: &str) -> ! {
        panic!("{} at {},{}", error_msg, self.get_line(), self.get_col())
    }

    pub fn fatal_extra<D>(&self, error_msg: &str, value: D) -> !
        where D: std::fmt::Display,
    {
        panic!("{}:{} on line {}", error_msg, value, self.get_line())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::io::Cursor;
    use rstest::rstest;

    use super::*;

    struct ScannerIter<T> {
        scanner: Scanner<T>,
    }

    impl<T> ScannerIter<T> {
        fn new(scanner: Scanner<T>) -> Self {
            ScannerIter { scanner }
        }
    }

    impl <T> Iterator for ScannerIter<T>
    where T: Read
    {
        type Item = Token;

        fn next(&mut self) -> Option<Self::Item> {
            self.scanner.scan()
        }
    }

    fn to_vec_tokentype(v: Vec<Token>) -> Vec<TokenType> {
        v.into_iter().map(|t| t.ttype).collect()
    }

    fn scan_all_mem(input: &str) -> Vec<Token> {
        let scanner = Scanner::new(Cursor::new(input.as_bytes().to_vec()));
        let mut tokens = vec![];
        while let Some(tok) = scanner.scan() {
            tokens.push(tok);
        }
        tokens
    }

    // --- in-memory unit tests ---

    #[test]
    fn scan_plus() {
        assert_eq!(to_vec_tokentype(scan_all_mem("+")), vec![TokenType::Plus]);
    }

    #[test]
    fn scan_minus() {
        assert_eq!(to_vec_tokentype(scan_all_mem("-")), vec![TokenType::Minus]);
    }

    #[test]
    fn scan_star() {
        assert_eq!(to_vec_tokentype(scan_all_mem("*")), vec![TokenType::Star]);
    }

    #[test]
    fn scan_slash() {
        assert_eq!(to_vec_tokentype(scan_all_mem("/")), vec![TokenType::Slash]);
    }

    #[test]
    fn scan_single_digit_intlit() {
        assert_eq!(to_vec_tokentype(scan_all_mem("7")), vec![TokenType::IntLit(7)]);
    }

    #[test]
    fn scan_multidigit_intlit() {
        assert_eq!(to_vec_tokentype(scan_all_mem("1234")), vec![TokenType::IntLit(1234)]);
    }

    #[test]
    fn scan_skips_leading_and_trailing_whitespace() {
        assert_eq!(to_vec_tokentype(scan_all_mem("  +  ")), vec![TokenType::Plus]);
    }

    #[test]
    fn scan_returns_none_at_eof() {
        let scanner = Scanner::new(Cursor::new(b"".to_vec()));
        assert!(scanner.scan().is_none());
    }

    #[test]
    fn scan_tracks_newlines() {
        let scanner = Scanner::new(Cursor::new(b"1\n2".to_vec()));
        scanner.scan(); // reads '1', then '\n' (line → 2), putback_chars '\n'
        assert_eq!(scanner.get_line(), 2);
        scanner.scan(); // reads putback_char '\n', then '2'
        assert_eq!(scanner.get_line(), 2);
    }

    // --- edge cases / panic tests ---

    #[test]
    #[should_panic(expected = "Unrecognised character")]
    fn scan_bang_not_followed_by_equal() {
        scan_all_mem("!x");
    }

    #[test]
    #[should_panic(expected = "Found EOF")]
    fn scan_bang_at_eof() {
        scan_all_mem("!");
    }

    #[test]
    #[should_panic(expected = "identifier too long")]
    fn scan_identifier_too_long() {
        let long_ident = "a".repeat(TEXTLEN + 1);
        scan_all_mem(&long_ident);
    }

    // ...

    #[test]
    fn scan_newline_then_eof() {
        let scanner = Scanner::new(Cursor::new(b"1\n".to_vec()));
        assert_eq!(scanner.scan().map(|t| t.ttype), Some(TokenType::IntLit(1)));
        assert_eq!(scanner.get_line(), 2);
        assert!(scanner.scan().is_none());
    }

    #[test]
    #[should_panic]
    fn scan_panics_on_unknown_character() {
        scan_all_mem("@");
    }

    #[test]
    fn scan_sequence_of_mixed_tokens() {
        assert_eq!(
            to_vec_tokentype(scan_all_mem("1 + 2")),
            vec![TokenType::IntLit(1), TokenType::Plus, TokenType::IntLit(2)],
        );
    }

    #[test]
    fn scan_int_keyword() {
        assert_eq!(to_vec_tokentype(scan_all_mem("int")), vec![TokenType::Int]);
    }

    #[test]
    fn scan_identifier() {
        assert_eq!(to_vec_tokentype(scan_all_mem("x")), vec![TokenType::Ident("x".to_string())]);
        assert_eq!(to_vec_tokentype(scan_all_mem("foo")), vec![TokenType::Ident("foo".to_string())]);
        assert_eq!(to_vec_tokentype(scan_all_mem("_bar")), vec![TokenType::Ident("_bar".to_string())]);
    }

    #[test]
    fn scan_equals() {
        assert_eq!(to_vec_tokentype(scan_all_mem("=")), vec![TokenType::Assign]);
    }

    static TEST_DATA_FILES: &[&str] = &[
        "input01",
        "input02",
        "input03",
        "input04",
        // "input05",
    ];

    static EXPECTED_TOKENS: &[&[TokenType]] = &[
        &[TokenType::IntLit(2), TokenType::Plus, TokenType::IntLit(3), TokenType::Star, TokenType::IntLit(5), TokenType::Minus, TokenType::IntLit(8), TokenType::Slash, TokenType::IntLit(3)],
        &[TokenType::IntLit(13), TokenType::Minus, TokenType::IntLit(6), TokenType::Plus, TokenType::IntLit(4), TokenType::Star, TokenType::IntLit(5), TokenType::Plus, TokenType::IntLit(8), TokenType::Slash, TokenType::IntLit(3)],
        &[TokenType::IntLit(12), TokenType::IntLit(34), TokenType::Plus, TokenType::Minus, TokenType::IntLit(56), TokenType::Star, TokenType::Slash, TokenType::Minus, TokenType::Minus, TokenType::IntLit(8), TokenType::Plus, TokenType::Star, TokenType::IntLit(2)],
        &[TokenType::IntLit(23), TokenType::Plus, TokenType::IntLit(18), TokenType::Minus, TokenType::IntLit(45)],
    ];

    fn get_test_file(filename: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/test/scanner")
                                                 .join(filename)
    }

    #[rstest]
    #[case::input01(get_test_file(TEST_DATA_FILES[0]), EXPECTED_TOKENS[0])]
    #[case::input02(get_test_file(TEST_DATA_FILES[1]), EXPECTED_TOKENS[1])]
    #[case::input03(get_test_file(TEST_DATA_FILES[2]), EXPECTED_TOKENS[2])]
    fn test_scanner_success(#[case] file: PathBuf, #[case] expected: &[TokenType]) {
        let file = std::fs::File::open(file).expect("Failed to open test file");
        let scanner = Scanner::new(file);

        let tokens: Vec<TokenType> = ScannerIter::new(scanner).map(|t| t.ttype).collect();

        assert_eq!(tokens, expected.to_vec());
    }

    #[rstest]
    #[case::input04(get_test_file(TEST_DATA_FILES[3]))]
    #[should_panic]
    fn test_scanner_fails(#[case] file: PathBuf) {
        let file = std::fs::File::open(file).expect("Failed to open test file");
        let scanner = Scanner::new(file);

        let _: Vec<TokenType> = ScannerIter::new(scanner).map(|t| t.ttype).collect();
    }

    // --- multi-char comparison operators ---

    #[test]
    fn scan_double_eq() {
        assert_eq!(to_vec_tokentype(scan_all_mem("==")), vec![TokenType::EQ]);
    }

    #[test]
    fn scan_ne() {
        assert_eq!(to_vec_tokentype(scan_all_mem("!=")), vec![TokenType::NE]);
    }

    #[test]
    fn scan_le() {
        assert_eq!(to_vec_tokentype(scan_all_mem("<=")), vec![TokenType::LE]);
    }

    #[test]
    fn scan_ge() {
        assert_eq!(to_vec_tokentype(scan_all_mem(">=")), vec![TokenType::GE]);
    }

    #[test]
    fn scan_single_char_lt_and_gt() {
        assert_eq!(to_vec_tokentype(scan_all_mem("<")), vec![TokenType::LT]);
        assert_eq!(to_vec_tokentype(scan_all_mem(">")), vec![TokenType::GT]);
    }

    // --- braces and parens ---

    #[test]
    fn scan_braces_and_parens() {
        assert_eq!(to_vec_tokentype(scan_all_mem("{")), vec![TokenType::LeftBrace]);
        assert_eq!(to_vec_tokentype(scan_all_mem("}")), vec![TokenType::RightBrace]);
        assert_eq!(to_vec_tokentype(scan_all_mem("(")), vec![TokenType::LeftParen]);
        assert_eq!(to_vec_tokentype(scan_all_mem(")")), vec![TokenType::RightParen]);
    }

    // --- keywords ---

    #[test]
    fn scan_keywords() {
        assert_eq!(to_vec_tokentype(scan_all_mem("char")), vec![TokenType::Char]);
        assert_eq!(to_vec_tokentype(scan_all_mem("else")), vec![TokenType::Else]);
        assert_eq!(to_vec_tokentype(scan_all_mem("for")), vec![TokenType::For]);
        assert_eq!(to_vec_tokentype(scan_all_mem("if")), vec![TokenType::If]);
        assert_eq!(to_vec_tokentype(scan_all_mem("int")), vec![TokenType::Int]);
        assert_eq!(to_vec_tokentype(scan_all_mem("long")), vec![TokenType::Long]);
        assert_eq!(to_vec_tokentype(scan_all_mem("return")), vec![TokenType::Return]);
        assert_eq!(to_vec_tokentype(scan_all_mem("void")), vec![TokenType::Void]);
        assert_eq!(to_vec_tokentype(scan_all_mem("while")), vec![TokenType::While]);
    }

    // --- Scanner helper methods ---

    #[test]
    fn matches_succeeds_on_correct_token() {
        let scanner = Scanner::new(Cursor::new(b"+".to_vec()));
        let tok = scanner.matches(TokenType::Plus, "+");
        assert_eq!(tok.ttype, TokenType::Plus);
    }

    #[test]
    #[should_panic]
    fn matches_panics_on_wrong_token() {
        let scanner = Scanner::new(Cursor::new(b"-".to_vec()));
        scanner.matches(TokenType::Plus, "+");
    }

    #[test]
    fn ident_returns_identifier_name() {
        let scanner = Scanner::new(Cursor::new(b"foo".to_vec()));
        assert_eq!(scanner.ident(None), "foo");
    }

    #[test]
    fn maybe_token_true_consumes_token() {
        let scanner = Scanner::new(Cursor::new(b"+".to_vec()));
        assert!(scanner.maybe_token(TokenType::Plus));
        assert!(scanner.scan().is_none());
    }

    #[test]
    fn maybe_token_false_puts_back_token() {
        let scanner = Scanner::new(Cursor::new(b"+".to_vec()));
        assert!(!scanner.maybe_token(TokenType::Minus));
        assert_eq!(scanner.scan().map(|t| t.ttype), Some(TokenType::Plus));
    }

    #[test]
    fn is_eof_false_with_putback_token() {
        let scanner = Scanner::new(Cursor::new(b"".to_vec()));
        scanner.putback_token(Token { line: 1, col: 1, ttype: TokenType::Semi });
        assert!(!scanner.is_eof());
    }

    #[test]
    fn is_eof_true_on_empty_input() {
        let scanner = Scanner::new(Cursor::new(b"".to_vec()));
        assert!(scanner.is_eof());
    }

    #[test]
    fn is_eof_false_on_non_empty_input() {
        let scanner = Scanner::new(Cursor::new(b"+".to_vec()));
        assert!(!scanner.is_eof());
    }
}
