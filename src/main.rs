use anyhow::Result;
use clap::Parser;
use acwj_rs::{
    Scanner,
    binexpr,
    interpret_ast,
};

#[derive(Parser, Debug)]
struct Args {
    file_name: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let file = std::fs::File::open(args.file_name).expect("Failed to open file");
    let scanner = Scanner::new(file);

    // Call binexpr with minimal previos precedence
    let ast_tree = binexpr(&scanner, 0)?;
    match interpret_ast(&ast_tree, None) {
        Ok(res) => println!("{}", res),
        Err(err) => eprintln!("{}", err),
    };

    Ok(())
}
