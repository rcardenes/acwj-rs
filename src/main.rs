use std::{
    io::Write,
    process
};
use anyhow::Result;
use clap::Parser as ClapParser;
use acwj_rs::{
    Scanner,
    cg_x86_64::X86_64Backend,
    cgen::{CodeBackend, CodeGenerator},
    pars::Parser,
    sym::SymbolTable,
};

static DEFAULT_ARCH: &str = "x86_64";

#[derive(ClapParser, Debug)]
struct Cli {
    /// Architecture selector. Use '?' for a list of valid arches.
    #[clap(long, default_value_t = String::from("x86_64"))]
    arch: String,

    /// Path to the input file
    file_name: String,

    /// Path to the output file
    #[clap(short, default_value_t = String::from("out.s"))]
    output: String,
}

fn select_backend<T>(arch: &str, output: T) -> impl CodeBackend
    where T: Write,
{
    match arch {
        "?" => {
            println!("Valid target architectures:\n");
            println!("  x86_64 -> Intel/AMD 64-bit x86");
            println!("  arm32  -> armv7 (Raspberry Pi 3/4)");

            process::exit(0)
        },
        "x86_64" => X86_64Backend::new(output),
        "arm32" => unimplemented!(),
        _ => {
            println!("Unknown architecture: '{}'. Using the default '{}'", arch, DEFAULT_ARCH);
            X86_64Backend::new(output)
        }
    }
}

fn main() -> Result<()> {
    let args = Cli::parse();

    let file = std::fs::File::open(args.file_name).expect("Failed to open file");
    let output = std::fs::File::create(args.output)?;

    let scanner = Scanner::new(file);
    let backend = select_backend(&args.arch, output);

    // Generate symbol table populated with predefined symbols
    let symbols = SymbolTable::default();

    let mut code_gen = CodeGenerator::new(backend, &symbols);
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
