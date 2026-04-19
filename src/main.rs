use clap::Parser;
use acwj_rs::Scanner;

#[derive(Parser, Debug)]
struct Args {
    file_name: String,
}

fn main() {
    let args = Args::parse();
    let file = std::fs::File::open(args.file_name).expect("Failed to open file");
    let mut scanner = Scanner::new(file);

    while let Some(token) = scanner.scan() {
        println!("{:?}", token);
    }
}
