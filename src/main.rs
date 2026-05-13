use anyhow::Result;
use clap::Parser as ClapParser;
use acwj_rs::{
    Scanner,
    cg::X864_64Backend,
    cgen::CodeGenerator,
    sym::SymbolTable,
    pars::Parser,
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
    let mut code_gen = CodeGenerator::new(X864_64Backend::new(output));

    let mut symbols = SymbolTable::new();
    let mut parser = Parser::new(&scanner, &mut symbols);
    // Generate code and AST on the fly
    // Traverse the AST to generate code
    code_gen.gen_preamble()?;

    while let Some(tree) = parser.function_declaration()? {
        code_gen.gen_ast(&tree, None, None, None, 0)?;

    }

    Ok(())
}
