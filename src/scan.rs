use std::{
    cell::RefCell,
    io::{BufReader, Read}, ops::AddAssign
};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Plus,
    Minus,
    Star,
    Slash,
    IntLit(i64),
}

#[derive(Debug)]
pub struct Scanner<T> {
    buffer: RefCell<BufReader<T>>,
    putback_char: RefCell<Option<char>>,
    putback_token: RefCell<Option<Token>>,
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
        }
    }

    pub fn scan(&self) -> Option<Token> {
        if let Some(t) = self.putback_token.borrow_mut().take() {
            Some(t)
        } else {
            match self.skip()? {
                '+' => Some(Token::Plus),
                '-' => Some(Token::Minus),
                '*' => Some(Token::Star),
                '/' => Some(Token::Slash),
                c if c.is_ascii_digit() => {
                    Some(Token::IntLit(self.scanint(c)))
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

    fn skip(&self) -> Option<char> {
        while let Some(c) = self.next() {
            if !c.is_whitespace() {
                return Some(c);
            }
        }

        None
    }

    fn scanint(&self, first: char) -> i64 {
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
