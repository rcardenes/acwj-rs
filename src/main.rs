use anyhow::Result;
use clap::Parser as ClapParser;
use acwj_rs::{
    Scanner,
    cg::X86_64Backend,
    cgen::CodeGenerator,
    pars::Parser,
    sym::SymbolTable,
};

#[derive(ClapParser, Debug)]
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

    // Generate symbol table populated with predefined symbols
    let symbols = SymbolTable::default();

    let mut code_gen = CodeGenerator::new(X86_64Backend::new(output), &symbols);
    let parser = Parser::new(&scanner, &symbols, &code_gen);

    // Generate AST
    let mut functions = vec![];

    while let Some(tree) = parser.function_declaration()? {
        functions.push(tree);
    }

    // Traverse the AST to generate code
    code_gen.gen_preamble()?;
    for tree in functions {
        code_gen.gen_ast(&tree, None, None, 0)?;
    }

    Ok(())
}
