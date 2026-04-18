use std::io::{BufReader, Read};

#[derive(Debug)]
pub enum Token {
    Plus,
    Minus,
    Star,
    Slash,
    IntLiteral(i64),
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
                Some(Token::IntLiteral(self.scanint(c)))
            }
            _ => None,
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
