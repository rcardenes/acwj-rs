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
    fn add(&mut self, r1: Self::Reg, r2: Self::Reg) -> Result<Option<Self::Reg>>;
    fn sub(&mut self, r1: Self::Reg, r2: Self::Reg) -> Result<Option<Self::Reg>>;
    fn mul(&mut self, r1: Self::Reg, r2: Self::Reg) -> Result<Option<Self::Reg>>;
    fn div(&mut self, r1: Self::Reg, r2: Self::Reg) -> Result<Option<Self::Reg>>;
    fn compare_and_set(&mut self, op: &Ast, r1: Self::Reg, r2: Self::Reg) -> Result<Option<Self::Reg>>;
    fn compare_and_jump(&mut self, op: &Ast, r1: Self::Reg, r2: Self::Reg, label_num: usize) -> Result<Option<Self::Reg>>;
    fn label(&mut self, label_num: usize) -> Result<()>;
    fn jump(&mut self, label_num: usize) -> Result<()>;
    fn print_int(&mut self, r: Self::Reg) -> Result<()>;
}

pub struct CodeGenerator<B>
where
    B: CodeBackend,
{
    backend: B,
    last_label: usize,
}

impl<B> CodeGenerator<B>
where
    B: CodeBackend,
{
    pub fn new(backend: B) -> Self {
        Self { backend, last_label: 0 }
    }

    fn label(&mut self) -> usize {
        self.last_label += 1;
        self.last_label
    }

    fn binary<F>(&mut self, tree: &Tree<AstNode>, node: &AstNode, func: F) -> Result<Option<B::Reg>>
        where F: FnOnce(&mut B, B::Reg, B::Reg) -> Result<Option<B::Reg>>,
        {
            let left_reg = self.gen_ast(tree, node.get_left_index(), None, Some(&node.op), 0)?;
            let right_reg = self.gen_ast(tree, node.get_right_index(), left_reg, Some(&node.op), 0)?;
            match (left_reg, right_reg) {
                (Some(lr), Some(rr)) => {
                    func(&mut self.backend, lr, rr)
                }
                _ => unreachable!("Critical: binary expression subtree returned NoReg")
            }
        }

    pub fn gen_if(&mut self, tree: &Tree<AstNode>, node: &AstNode) -> Result<Option<B::Reg>> {
        let l_false = self.label();
        let right_idx = node.get_right_index();
        let l_end = right_idx.map(|_| self.label());

        // Generate the condition code. It will include a conditional jump to the
        // false branch
        self.gen_ast(tree, node.get_left_index(), None, Some(&node.op), l_false)?;
        self.backend.free_all_registers()?;

        // Generate the code for the true branch
        self.gen_ast(tree, node.get_mid_index(), None, Some(&node.op), 0)?;
        self.backend.free_all_registers()?;
        // If we generated an "end" clause, now it's the time to jump
        if let Some(label) = l_end { self.backend.jump(label)?; }

        // Now, the false label, which will be serve as "end" if there's no false branch,
        self.backend.label(l_false)?;

        if let Some(idx) = right_idx {
            self.gen_ast(tree, Some(idx), None, Some(&node.op), 0)?;
            self.backend.free_all_registers()?;
            // We're sure that l_end was generated, so we're safe just unwrapping
            self.backend.label(l_end.unwrap())?;
        }

        Ok(None)
    }

    pub fn gen_while(&mut self, tree: &Tree<AstNode>, node: &AstNode) -> Result<Option<B::Reg>> {
        let l_start = self.label();
        let l_end = self.label();

        self.backend.label(l_start)?;

        // Generate the condition code, with a jump to the "end" label when the condition fails
        self.gen_ast(tree, node.get_left_index(), None, Some(&node.op), l_end)?;
        self.backend.free_all_registers()?;

        // Generate the body
        self.gen_ast(tree, node.get_right_index(), None, Some(&node.op), 0)?;
        self.backend.free_all_registers()?;
        // And back to start to test the condition again
        self.backend.jump(l_start)?;

        self.backend.label(l_end)?;

        Ok(None)
    }

    pub fn gen_print(&mut self, tree: &Tree<AstNode>, node: &AstNode) -> Result<Option<B::Reg>> {
        match self.gen_ast(tree, node.get_left_index(), None, None, 0)? {
            Some(reg) => {
                self.backend.print_int(reg)?;
                self.backend.free_all_registers()?;
            }
            None => unreachable!("gen_print: the expression returned no register")
        }
        Ok(None)
    }

    pub fn gen_glue_ast(&mut self, tree: &Tree<AstNode>, node: &AstNode) -> Result<Option<B::Reg>> {
        let _ = self.gen_ast(tree, node.get_left_index(), None, Some(&Ast::Glue), 0)?;
        self.backend.free_all_registers()?;
        let _ = self.gen_ast(tree, node.get_right_index(), None, Some(&Ast::Glue), 0)?;
        self.backend.free_all_registers()?;
        Ok(None)
    }


    pub fn gen_ast(&mut self, tree: &Tree<AstNode>, node_index: Option<usize>, r: Option<B::Reg>, parent_op: Option<&Ast>, label: usize) -> Result<Option<B::Reg>> {
        let root = tree.get_root_or_node(node_index).unwrap();

        // Special handling for statements
        match &root.op {
            Ast::If => self.gen_if(tree, root),
            Ast::While => self.gen_while(tree, root),
            Ast::Print => self.gen_print(tree, root),
            Ast::Glue => self.gen_glue_ast(tree, root),
            Ast::Add => self.binary(tree, root, |cg, r1, r2| cg.add(r1, r2)),
            Ast::Subtract => self.binary(tree, root, |cg, r1, r2| cg.sub(r1, r2)),
            Ast::Multiply => self.binary(tree, root, |cg, r1, r2| cg.mul(r1, r2)),
            Ast::Divide => self.binary(tree, root, |cg, r1, r2| cg.div(r1, r2)),
            Ast::Equal|Ast::NotEqual
                |Ast::LessThan|Ast::GreaterThan
                |Ast::LessThanOrEqual
                |Ast::GreaterThanOrEqual => {
                    if parent_op.is_some_and(Ast::is_loop_with_comparison) {
                        self.binary(tree, root, 
                            |cg: &mut B, r1, r2| cg.compare_and_jump(&root.op, r1, r2, label))
                    } else {
                        self.binary(tree, root, 
                            |cg: &mut B, r1, r2| cg.compare_and_set(&root.op, r1, r2))
                    }
                },
            Ast::IntLit(val) => self.backend.load_int(*val).map(Some),
            Ast::Ident(id) => self.backend.load_glob(id).map(Some),
            Ast::LvIdent(id) => self.backend.store_glob(r.unwrap(), id).map(Some),
            // For Assign, all the work is done by the code generation down its branches.
            // We need only to return the right-branch register
            Ast::Assign => self.binary(tree, root, |_, _, r2| Ok(Some(r2))),
            Ast::GlobalDec(id) => {
                self.backend.glob_sym(id).map(|_| None)
            }
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

    pub fn gen_globsym(&mut self, name: &str) -> Result<()> {
        self.backend.glob_sym(name)
    }
}

