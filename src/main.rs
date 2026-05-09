use anyhow::Result;
use clap::Parser;
use acwj_rs::{
    Scanner,
    cg::CodeGenerator,
    cgen::*,
    stmt::statements,
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

    let output = std::fs::File::create(args.output)?;
    let mut code_gen = CodeGenerator::new(output);

    // Generate code
    gen_preamble(&mut code_gen)?;
    // generate_code(&ast_tree, &mut code_gen)?;
    statements(&scanner, &mut code_gen)?;
    gen_postamble(&mut code_gen)?;

    Ok(())
}
