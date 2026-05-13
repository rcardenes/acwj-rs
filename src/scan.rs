use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    io::{BufReader, Read},
    ops::AddAssign,
};

pub static TEXTLEN: usize = 512; // Maximum lenght of symbols in input

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
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
    Ident(String), // identifier
    LeftBrace,     // {
    RightBrace,    // }
    LeftParen,     // (
    RightParen,    // )
    Semi,          // ;
    Else,          // else
    If,            // if
    Int,           // int
    Print,         // print
    While,         // while
    IntLit(i64),   // Integer literal
}


impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

static KEYWORDS: &[(&str, Token)] = &[
    ("else", Token::Else),
    ("if", Token::If),
    ("int", Token::Int),
    ("print", Token::Print),
    ("while", Token::While),
];

fn is_valid_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[derive(Debug)]
pub struct Scanner<T> {
    buffer: RefCell<BufReader<T>>,
    putback_char: RefCell<Option<char>>,
    putback_token: RefCell<Option<Token>>,
    keyword_map: HashMap<&'static str, Token>,
    line: RefCell<usize>,
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
            keyword_map: KEYWORDS.iter().cloned().collect(),
        }
    }

    pub fn scan(&self) -> Option<Token> {
        if let Some(t) = self.putback_token.borrow_mut().take() {
            Some(t)
        } else {
            match self.skip()? {
                '(' => Some(Token::LeftParen),
                ')' => Some(Token::RightParen),
                '{' => Some(Token::LeftBrace),
                '}' => Some(Token::RightBrace),
                '+' => Some(Token::Plus),
                '-' => Some(Token::Minus),
                '*' => Some(Token::Star),
                '/' => Some(Token::Slash),
                ';' => Some(Token::Semi),
                '=' => {
                    if let Some('=') = self.peek() {
                        self.clear_putback();
                        Some(Token::EQ)
                    } else {
                        Some(Token::Assign)
                    }
                },
                '<' => {
                    if let Some('=') = self.peek() {
                        self.clear_putback();
                        Some(Token::LE)
                    } else {
                        Some(Token::LT)
                    }
                },
                '>' => {
                    if let Some('=') = self.peek() {
                        self.clear_putback();
                        Some(Token::GE)
                    } else {
                        Some(Token::GT)
                    }
                },
                '!' => {
                    match self.peek() {
                        Some('=') => {
                            self.clear_putback();
                            Some(Token::NE)
                        },
                        Some(c) => self.fatal_extra("Unrecognised character", c),
                        None => self.fatal("Found EOF while parsing expression")
                    }
                },
                c if c.is_ascii_digit() => {
                    Some(Token::IntLit(self.scan_int(c)))
                }
                c if c.is_ascii_alphabetic() || c == '_' => {
                    self.putback_char(c);
                    let ident = self.scan_ident(TEXTLEN);

                    if let Some(t) = self.keyword(&ident) {
                        Some(t)
                    } else {
                        Some(Token::Ident(ident))
                    }
                }
                c => {
                    panic!("Unrecognised character '{}' on line {}", c, self.line.borrow());
                }
            }
        }
    }

    pub fn get_line(&self) -> usize {
        *self.line.borrow()
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
                        self.line.borrow_mut().add_assign(1)
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

    fn clear_putback(&self) {
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
                    panic!("identifier too long in line {}", self.line.borrow());
                }

                buffer.push(c);
            } else {
                self.putback_char(c);
                break;
            }
        }

        buffer
    }

    fn keyword(&self, s: &str) -> Option<Token> {
        self.keyword_map.get(s).cloned()
    }

    // Consume a token. If it's the end of the file, return 'false'.
    // If a token was found and matches the expected one, return 'true'.
    // Else panic
    pub fn if_not_eof_matches<F>(&self, expected: F, expected_string: &str) -> Option<Token>
        where F: Fn(&Token) -> bool
    {
        if let Some(tok) = self.scan() {
            if expected(&tok) {
                Some(tok)
            } else {
                panic!("Expected {} on line {}", expected_string, self.line.borrow());
            }
        } else {
            None
        }
    }

    // Consume a token. If it matches the expected one. Else panic
    pub fn matches(&self, expected: Token, expected_string: &str) -> Token {
        if let Some(tok) = self.if_not_eof_matches(|tok| *tok == expected, expected_string) {
            tok
        } else {
            panic!("End of input while expecting {}", expected_string);
        }
    }

    pub fn ident(&self) -> String {
        let res = self.if_not_eof_matches(|tok| matches!(tok, &Token::Ident(_)), "identifier");
        match res {
            Some(Token::Ident(name)) => name,
            None => panic!("End of input while expecting an identifier"),
            _ => unreachable!()
        }
    }

    pub fn maybe_token(&self, token: Token) -> bool {
        match self.scan() {
            Some(tok) => {
                if tok == token {
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
        self.matches(Token::Semi, ";")
    }

    pub fn lbrace(&self) -> Token {
        self.matches(Token::LeftBrace, "{")
    }

    pub fn lparen(&self) -> Token {
        self.matches(Token::LeftParen, "(")
    }

    pub fn rparen(&self) -> Token {
        self.matches(Token::RightParen, ")")
    }

    pub fn fatal(&self, error_msg: &str) -> ! {
        panic!("{} on line {}", error_msg, self.get_line())
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
        assert_eq!(scan_all_mem("+"), vec![Token::Plus]);
    }

    #[test]
    fn scan_minus() {
        assert_eq!(scan_all_mem("-"), vec![Token::Minus]);
    }

    #[test]
    fn scan_star() {
        assert_eq!(scan_all_mem("*"), vec![Token::Star]);
    }

    #[test]
    fn scan_slash() {
        assert_eq!(scan_all_mem("/"), vec![Token::Slash]);
    }

    #[test]
    fn scan_single_digit_intlit() {
        assert_eq!(scan_all_mem("7"), vec![Token::IntLit(7)]);
    }

    #[test]
    fn scan_multidigit_intlit() {
        assert_eq!(scan_all_mem("1234"), vec![Token::IntLit(1234)]);
    }

    #[test]
    fn scan_skips_leading_and_trailing_whitespace() {
        assert_eq!(scan_all_mem("  +  "), vec![Token::Plus]);
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

    #[test]
    #[should_panic]
    fn scan_panics_on_unknown_character() {
        scan_all_mem("@");
    }

    #[test]
    fn scan_sequence_of_mixed_tokens() {
        assert_eq!(
            scan_all_mem("1 + 2"),
            vec![Token::IntLit(1), Token::Plus, Token::IntLit(2)],
        );
    }

    #[test]
    fn scan_int_keyword() {
        assert_eq!(scan_all_mem("int"), vec![Token::Int]);
    }

    #[test]
    fn scan_identifier() {
        assert_eq!(scan_all_mem("x"), vec![Token::Ident("x".to_string())]);
        assert_eq!(scan_all_mem("foo"), vec![Token::Ident("foo".to_string())]);
        assert_eq!(scan_all_mem("_bar"), vec![Token::Ident("_bar".to_string())]);
    }

    #[test]
    fn scan_equals() {
        assert_eq!(scan_all_mem("="), vec![Token::Assign]);
    }

    static TEST_DATA_FILES: &[&str] = &[
        "input01",
        "input02",
        "input03",
        "input04",
        // "input05",
    ];

    static EXPECTED_TOKENS: &[&[Token]] = &[
        &[Token::IntLit(2), Token::Plus, Token::IntLit(3), Token::Star, Token::IntLit(5), Token::Minus, Token::IntLit(8), Token::Slash, Token::IntLit(3)],
        &[Token::IntLit(13), Token::Minus, Token::IntLit(6), Token::Plus, Token::IntLit(4), Token::Star, Token::IntLit(5), Token::Plus, Token::IntLit(8), Token::Slash, Token::IntLit(3)],
        &[Token::IntLit(12), Token::IntLit(34), Token::Plus, Token::Minus, Token::IntLit(56), Token::Star, Token::Slash, Token::Minus, Token::Minus, Token::IntLit(8), Token::Plus, Token::Star, Token::IntLit(2)],
        &[Token::IntLit(23), Token::Plus, Token::IntLit(18), Token::Minus, Token::IntLit(45)],
    ];

    fn get_test_file(filename: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/test/scanner")
                                                 .join(filename)
    }

    #[rstest]
    #[case::input01(get_test_file(TEST_DATA_FILES[0]), EXPECTED_TOKENS[0])]
    #[case::input02(get_test_file(TEST_DATA_FILES[1]), EXPECTED_TOKENS[1])]
    #[case::input03(get_test_file(TEST_DATA_FILES[2]), EXPECTED_TOKENS[2])]
    fn test_scanner_success(#[case] file: PathBuf, #[case] expected: &[Token]) {
        let file = std::fs::File::open(file).expect("Failed to open test file");
        let scanner = Scanner::new(file);

        let tokens: Vec<Token> = ScannerIter::new(scanner).collect();
        eprintln!("{:?}", tokens);

        assert_eq!(tokens, expected.to_vec());
    }

    #[rstest]
    #[case::input04(get_test_file(TEST_DATA_FILES[3]))]
    #[should_panic]
    fn test_scanner_fails(#[case] file: PathBuf) {
        let file = std::fs::File::open(file).expect("Failed to open test file");
        let scanner = Scanner::new(file);

        let _: Vec<Token> = ScannerIter::new(scanner).collect();
    }
}
