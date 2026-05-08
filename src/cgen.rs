// Code Generation

use anyhow::Result;
use crate::{
    cg::{CodeGenerator, Reg},
    ast::{Ast, AstNode},
    tree::{IndexableNode, Tree},
};

fn gen_ast<T>(tree: &Tree<AstNode>, node_index: Option<usize>, code_gen: &mut CodeGenerator<T>) -> Result<Reg>
    where T: std::io::Write,
{
    fn binary<F, T>(tree: &Tree<AstNode>, node: &AstNode, code_gen: &mut CodeGenerator<T>, func: F) -> Result<Reg>
        where F: Fn(&mut CodeGenerator<T>, Reg, Reg) -> Result<Reg>,
              T: std::io::Write,
    {
        let left_reg = gen_ast(tree, node.get_left_index(), code_gen)?;
        let right_reg = gen_ast(tree, node.get_right_index(), code_gen)?;
        func(code_gen, left_reg, right_reg)
    }

    let root = tree.get_root_or_node(node_index).unwrap();

    match root.op {
        Ast::Add => binary(tree, root, code_gen, |cg, r1, r2| cg.add(r1, r2)),
        Ast::Subtract => binary(tree, root, code_gen, |cg, r1, r2| cg.sub(r1, r2)),
        Ast::Multiply => binary(tree, root, code_gen, |cg, r1, r2| cg.mul(r1, r2)),
        Ast::Divide => binary(tree, root, code_gen, |cg, r1, r2| cg.div(r1, r2)),
        Ast::IntLit(val) => code_gen.load(val)
    }
}

pub fn generate_code<T>(tree: &Tree<AstNode>, code_gen: &mut CodeGenerator<T>) -> Result<()>
    where T: std::io::Write,
{
    code_gen.preamble()?;
    let reg = gen_ast(tree, None, code_gen)?;
    code_gen.print_int(reg)?;
    code_gen.postamble()?;

    Ok(())
}
