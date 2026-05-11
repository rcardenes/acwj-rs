// Given an AST, generate code recursively

use anyhow::Result;
use crate::{
    ast::{Ast, AstNode},
    tree::{IndexableNode, Tree},
};

pub trait CodeBackend {
    type Reg: Copy;

    fn free_all_registers(&mut self) -> Result<()>;
    fn preamble(&mut self) -> Result<()>;
    fn postamble(&mut self) -> Result<()>;
    fn load_int(&mut self, val: i64) -> Result<Self::Reg>;
    fn load_glob(&mut self, ident: &str) -> Result<Self::Reg>;
    fn store_glob(&mut self, r: Self::Reg, ident: &str) -> Result<Self::Reg>;
    fn glob_sym(&mut self, sym: &str) -> Result<()>;
    fn add(&mut self, r1: Self::Reg, r2: Self::Reg) -> Result<Self::Reg>;
    fn sub(&mut self, r1: Self::Reg, r2: Self::Reg) -> Result<Self::Reg>;
    fn mul(&mut self, r1: Self::Reg, r2: Self::Reg) -> Result<Self::Reg>;
    fn div(&mut self, r1: Self::Reg, r2: Self::Reg) -> Result<Self::Reg>;
    fn print_int(&mut self, r: Self::Reg) -> Result<()>;
}

pub struct CodeGenerator<B>
where
    B: CodeBackend,
{
    backend: B,
}

impl<B> CodeGenerator<B>
where
    B: CodeBackend,
{
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    fn binary<F>(&mut self, tree: &Tree<AstNode>, node: &AstNode, func: F) -> Result<B::Reg>
        where F: Fn(&mut B, B::Reg, B::Reg) -> Result<B::Reg>,
        {
            let left_reg = self.gen_ast(tree, node.get_left_index(), None)?;
            let right_reg = self.gen_ast(tree, node.get_right_index(), Some(left_reg))?;
            func(&mut self.backend, left_reg, right_reg)
        }

    pub fn gen_ast(&mut self, tree: &Tree<AstNode>, node_index: Option<usize>, r: Option<B::Reg>) -> Result<B::Reg>
    {

        let root = tree.get_root_or_node(node_index).unwrap();

        match &root.op {
            Ast::Add => self.binary(tree, root, |cg, r1, r2| cg.add(r1, r2)),
            Ast::Subtract => self.binary(tree, root, |cg, r1, r2| cg.sub(r1, r2)),
            Ast::Multiply => self.binary(tree, root, |cg, r1, r2| cg.mul(r1, r2)),
            Ast::Divide => self.binary(tree, root, |cg, r1, r2| cg.div(r1, r2)),
            Ast::IntLit(val) => self.backend.load_int(*val),
            Ast::Ident(id) => self.backend.load_glob(id),
            Ast::LvIdent(id) => self.backend.store_glob(r.unwrap(), id),
            // For Assign, all the work is done by the code generation down its branches.
            // We need only to return the right-branch register
            Ast::Assign => self.binary(tree, root, |_, _, r2| Ok(r2)),
        }
    }

    pub fn gen_preamble(&mut self) -> Result<()> {
        self.backend.preamble()
    }

    pub fn gen_postamble(&mut self) -> Result<()> {
        self.backend.postamble()
    }

    pub fn gen_freeregs(&mut self) -> Result<()> {
        self.backend.free_all_registers()
    }

    pub fn gen_printint(&mut self, reg: B::Reg) -> Result<()> {
        self.backend.print_int(reg)
    }

    pub fn gen_globsym(&mut self, name: &str) -> Result<()> {
        self.backend.glob_sym(name)
    }

    pub fn generate_code(&mut self, tree: &Tree<AstNode>) -> Result<()> {
        self.backend.preamble()?;
        let reg = self.gen_ast(tree, None, None)?;
        self.backend.print_int(reg)?;
        self.backend.postamble()?;

        Ok(())
    }
}

