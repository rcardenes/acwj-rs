use anyhow::Result;
use clap::Parser;
use acwj_rs::{
    Scanner,
    cg::CodeGenerator,
    cgen::generate_code,
    binexpr,
    interpret_ast,
};

#[derive(Parser, Debug)]
struct Cli {
    file_name: String,
    #[clap(short, default_value_t = String::from("out.s"))]
    output: String,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let file = std::fs::File::open(args.file_name).expect("Failed to open file");
    let scanner = Scanner::new(file);

    // Call binexpr with minimal previos precedence
    let ast_tree = binexpr(&scanner, 0)?;

    // Calculate the final result
    match interpret_ast(&ast_tree, None) {
        Ok(res) => println!("{}", res),
        Err(err) => eprintln!("{}", err),
    };

    let output = std::fs::File::create(args.output)?;
    let mut code_gen = CodeGenerator::new(output);

    // Generate code
    generate_code(&ast_tree, &mut code_gen)?;

    Ok(())
}
