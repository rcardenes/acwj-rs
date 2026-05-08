use std::io::{BufReader, Read};

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
    buffer: BufReader<T>,
    putback: Option<char>,
    line: usize,
}

impl<T> Scanner<T>
where T: Read
{
    pub fn new(reader: T) -> Self {
        Scanner {
            buffer: BufReader::new(reader),
            putback: None,
            line: 1,
        }
    }

    pub fn scan(&mut self) -> Option<Token> {
        match self.skip()? {
            '+' => Some(Token::Plus),
            '-' => Some(Token::Minus),
            '*' => Some(Token::Star),
            '/' => Some(Token::Slash),
            c if c.is_digit(10) => {
                Some(Token::IntLit(self.scanint(c)))
            }
            c => {
                panic!("Unrecognised character '{}' on line {}", c, self.line);
            }
        }
    }

    fn putback(&mut self, c: char) {
        self.putback = Some(c);
    } 

    fn next(&mut self) -> Option<char> {
        if let Some(c) = self.putback.take() {
            Some(c)
        } else {
            let mut buf = [0; 1];

            match self.buffer.read_exact(&mut buf) {
                Ok(_) => {
                    if buf[0] == b'\n' {
                        self.line += 1;
                    } else {
                    }
                    Some(buf[0] as char)
                }
                Err(_) => None,
            }
        }
    }

    fn skip(&mut self) -> Option<char> {
        while let Some(c) = self.next() {
            if !c.is_whitespace() {
                return Some(c);
            }
        }

        None
    }

    fn scanint(&mut self, first: char) -> i64 {
        let mut value = first.to_digit(10).unwrap() as i64;

        while let Some(c) = self.next() {
            if c.is_digit(10) {
                value = value * 10 + c.to_digit(10).unwrap() as i64;
            } else {
                self.putback(c);
                break;
            }
        }

        value
    }
}

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

#[cfg(test)]

mod tests {
    use std::path::PathBuf;
    use rstest::rstest;
        // assert_eq!(tokens, expected);

    use super::*;

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
    #[case::input04(get_test_file(TEST_DATA_FILES[3]), EXPECTED_TOKENS[3])]
    fn test_scanner(#[case] file: PathBuf, #[case] expected: &[Token]) {
        let file = std::fs::File::open(file).expect("Failed to open test file");
        let scanner = Scanner::new(file);

        let tokens: Vec<Token> = ScannerIter::new(scanner).into_iter().collect();

        assert_eq!(tokens, expected.to_vec());
    }
}
