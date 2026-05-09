use anyhow::Result;
use crate::{
    cg::CodeGenerator,
    cgen::{gen_ast, gen_printint, gen_freeregs},
    expr::binexpr,
    scan::{Scanner, Token},
};

pub fn statements<T1, T2>(scanner: &Scanner<T1>, code_gen: &mut CodeGenerator<T2>) -> Result<()>
    where T1: std::io::Read,
          T2: std::io::Write,
{
    while scanner.if_not_eof_matches(Token::Print, "print") {
        let tree = binexpr(scanner, 0)?;
        let reg = gen_ast(&tree, None, code_gen)?;
        gen_printint(code_gen, reg)?;
        gen_freeregs(code_gen);
        scanner.semi();
    }

    Ok(())
}
