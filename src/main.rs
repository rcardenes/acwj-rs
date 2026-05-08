use clap::Parser;
use acwj_rs::{Scanner, Token};

#[derive(Parser, Debug)]
struct Args {
    file_name: String,
}

fn scanfile(file_name: &str) {
    let file = std::fs::File::open(file_name).expect("Failed to open file");
    let mut scanner = Scanner::new(file);

    while let Some(token) = scanner.scan() {
        let tok = match token {
            Token::Plus => "+".to_string(),
            Token::Minus => "-".to_string(),
            Token::Star => "*".to_string(),
            Token::Slash => "/".to_string(),
            Token::IntLit(val) => format!("intlit, value {}", val),
        };

        println!("Token {tok}");
    }
}

fn main() {
    let args = Args::parse();
    scanfile(&args.file_name);
}
