use std::{
    io::Read,
    process::ExitCode
};
use anyhow::Result;
use clap::Parser as ClapParser;
use acwj_rs::{
    Scanner,
    cg_arm32::ArmV7Backend,
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

fn compile<T, B>(scanner: &Scanner<T>, symbols: &SymbolTable, code_gen: &mut CodeGenerator<B>) -> Result<()>
    where T: Read,
          B: CodeBackend,
{

    let parser = Parser::new(scanner, symbols, code_gen);

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
    code_gen.gen_postamble()?;

    Ok(())
}

fn main() -> ExitCode {
    let args = Cli::parse();

    let file = match std::fs::File::open(args.file_name) {
        Ok(fd) => fd,
        Err(st) => {
            eprintln!("{}", st);
            return ExitCode::FAILURE
        }
    };
    let output = match std::fs::File::create(args.output) {
        Ok(fd) => fd,
        Err(st) => {
            eprintln!("{}", st);
            return ExitCode::FAILURE
        }
    };

    let scanner = Scanner::new(file);
    // Generate symbol table populated with predefined symbols
    let symbols = SymbolTable::default();

    let arch = args.arch.as_str();

    if arch == "?" {
        println!("Valid target architectures:\n");
        println!("  x86_64 -> Intel/AMD 64-bit x86");
        println!("  arm32  -> armv7 (Raspberry Pi 3/4)");

        ExitCode::SUCCESS
    } else {
        let res = match arch {
            "arm32" => {
                let mut code_gen = CodeGenerator::new(ArmV7Backend::new(output), &symbols);
                compile(&scanner, &symbols, &mut code_gen)
            },
            _ => {
                if arch != DEFAULT_ARCH {
                    println!("Unknown architecture: '{}'. Using the default '{}'", arch, DEFAULT_ARCH);
                }
                let mut code_gen = CodeGenerator::new(X86_64Backend::new(output), &symbols);
                compile(&scanner, &symbols, &mut code_gen)
            }
        };

        match res {
            Ok(_) => ExitCode::SUCCESS,
            Err(st) => {
                eprintln!("{}", st);
                ExitCode::FAILURE
            },
        }
    }

}
