// Given an AST, generate code recursively

use anyhow::Result;
use crate::ast::{AstNode, Identifier};

pub trait CodeBackend {
    type Reg: Copy;

    fn free_all_registers(&mut self) -> Result<()>;
    fn preamble(&mut self) -> Result<()>;
    fn func_postamble(&mut self) -> Result<()>;
    fn func_preamble(&mut self, ident: &str) -> Result<()>;
    fn load_int(&mut self, val: i64) -> Result<Self::Reg>;
    fn load_glob(&mut self, ident: &str) -> Result<Self::Reg>;
    fn store_glob(&mut self, r: Self::Reg, ident: &str) -> Result<Self::Reg>;
    fn glob_sym(&mut self, sym: &str) -> Result<()>;
    fn add(&mut self, r1: Self::Reg, r2: Self::Reg) -> Result<Option<Self::Reg>>;
    fn sub(&mut self, r1: Self::Reg, r2: Self::Reg) -> Result<Option<Self::Reg>>;
    fn mul(&mut self, r1: Self::Reg, r2: Self::Reg) -> Result<Option<Self::Reg>>;
    fn div(&mut self, r1: Self::Reg, r2: Self::Reg) -> Result<Option<Self::Reg>>;
    fn compare_and_set(&mut self, op: &AstNode, r1: Self::Reg, r2: Self::Reg) -> Result<Option<Self::Reg>>;
    fn compare_and_jump(&mut self, op: &AstNode, r1: Self::Reg, r2: Self::Reg, label_num: usize) -> Result<Option<Self::Reg>>;
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

    fn binary<F>(&mut self, op: &AstNode, left: &AstNode, right: &AstNode, func: F) -> Result<Option<B::Reg>>
        where F: FnOnce(&mut B, B::Reg, B::Reg) -> Result<Option<B::Reg>>,
        {
            let left_reg = self.gen_ast(left, None, Some(op), 0)?;
            let right_reg = self.gen_ast(right, left_reg, Some(op), 0)?;
            match (left_reg, right_reg) {
                (Some(lr), Some(rr)) => {
                    func(&mut self.backend, lr, rr)
                }
                _ => unreachable!("Critical: binary expression subtree returned NoReg")
            }
        }

    pub fn gen_if(&mut self, tree: &AstNode) -> Result<Option<B::Reg>> {
        if let AstNode::If { cond, branch_t, branch_f } = tree {
            let l_false = self.label();
            let l_end = branch_f.as_ref().map(|_| self.label());

            // Generate the condition code. It will include a conditional jump to the
            // false branch
            self.gen_ast(cond, None, Some(tree), l_false)?;
            self.backend.free_all_registers()?;

            // Generate the code for the true branch
            self.gen_ast(branch_t, None, Some(tree), 0)?;
            self.backend.free_all_registers()?;

            // If we generated an "end" clause, now it's the time to jump
            if let Some(label) = l_end { self.backend.jump(label)?; }

            // Now, the false label, which will serve as "end" if there's no false branch,
            self.backend.label(l_false)?;

            if let Some(branch_f) = branch_f {
                self.gen_ast(branch_f, None, Some(tree), 0)?;
                self.backend.free_all_registers()?;
                // We're sure that l_end was generated, so we're safe just unwrapping
                self.backend.label(l_end.unwrap())?;
            }
        } else {
            unreachable!("Invalid input for gen_if!")
        }

        Ok(None)
    }

    pub fn gen_while(&mut self, tree: &AstNode) -> Result<Option<B::Reg>> {
        if let AstNode::While { cond, body } = tree {
            let l_start = self.label();
            let l_end = self.label();

            self.backend.label(l_start)?;

            // Generate the condition code, with a jump to the "end" label when the condition fails
            self.gen_ast(cond, None, Some(tree), l_end)?;
            self.backend.free_all_registers()?;

            // Generate the body
            self.gen_ast(body, None, Some(tree), 0)?;
            self.backend.free_all_registers()?;
            // And back to start to test the condition again
            self.backend.jump(l_start)?;

            self.backend.label(l_end)?;
        } else {
            unreachable!("Invalid input for gen_while!")
        }

        Ok(None)
    }

    pub fn gen_print(&mut self, tree: &AstNode) -> Result<Option<B::Reg>> {
        if let AstNode::Print { expr } = tree {
            match self.gen_ast(expr, None, None, 0)? {
                Some(reg) => {
                    self.backend.print_int(reg)?;
                    self.backend.free_all_registers()?;
                }
                None => unreachable!("gen_print: the expression returned no register")
            }
        } else {
            unreachable!("Invalid input for gen_print!")
        }

        Ok(None)
    }

    pub fn gen_glue_ast(&mut self, tree: &AstNode) -> Result<Option<B::Reg>> {
        if let AstNode::Glue { left, right } = tree {
            let _ = self.gen_ast(left, None, Some(tree), 0)?;
            self.backend.free_all_registers()?;
            let _ = self.gen_ast(right, None, Some(tree), 0)?;
            self.backend.free_all_registers()?;
        } else {
            unreachable!("Invalid input for gen_glue_ast!")
        }

        Ok(None)
    }

    pub fn gen_function(&mut self, tree: &AstNode) -> Result<Option<B::Reg>> {
        if let AstNode::Function { name, body } = tree {
            match &**name {
                AstNode::Ident(id) => {
                    self.backend.func_preamble(&id.name)?;
                },
                _ => unreachable!("Function with invalid id: {:?}", name)
            };
            self.gen_ast(body, None, Some(tree), 0)?;
            self.backend.func_postamble()?;
        } else {
            unreachable!("Generating a function without a root Ast::Function should be impossible")
        }
        Ok(None)
    }


    pub fn gen_ast(&mut self, tree: &AstNode, r: Option<B::Reg>, parent_op: Option<&AstNode>, label: usize) -> Result<Option<B::Reg>> {
        match tree {
            AstNode::Empty => Ok(None), // We shouldn't see empty statements, but simply skip it
            AstNode::Function {..} => self.gen_function(tree),
            AstNode::If {..} => self.gen_if(tree),
            AstNode::While {..} => self.gen_while(tree),
            AstNode::Print {..} => self.gen_print(tree),
            AstNode::Glue {..} => self.gen_glue_ast(tree),
            AstNode::Add { left, right } => self.binary(tree, left, right, |cg, r1, r2| cg.add(r1, r2)),
            AstNode::Subtract { left, right } => self.binary(tree, left, right, |cg, r1, r2| cg.sub(r1, r2)),
            AstNode::Multiply { left, right } => self.binary(tree, left, right, |cg, r1, r2| cg.mul(r1, r2)),
            AstNode::Divide { left, right } => self.binary(tree, left, right, |cg, r1, r2| cg.div(r1, r2)),
            AstNode::Equal { left, right }|AstNode::NotEqual { left, right }
                |AstNode::LessThan { left, right }|AstNode::GreaterThan { left, right }
                |AstNode::LessThanOrEqual { left, right }
                |AstNode::GreaterThanOrEqual { left, right } => {
                    if parent_op.is_some_and(AstNode::is_branching_stmt) {
                        self.binary(tree, left, right,
                            |cg: &mut B, r1, r2| cg.compare_and_jump(tree, r1, r2, label))
                    } else {
                        self.binary(tree, left, right,
                            |cg: &mut B, r1, r2| cg.compare_and_set(tree, r1, r2))
                    }
                },
            AstNode::IntLit(val) => self.backend.load_int(*val).map(Some),
            AstNode::Ident(Identifier { name }) => self.backend.load_glob(&name).map(Some),
            AstNode::LvIdent(Identifier { name }) => self.backend.store_glob(r.unwrap(), &name).map(Some),
            // For Assign, all the work is done by the code generation down its branches.
            // We need only to return the right-branch register
            AstNode::Assign { id, expr } => self.binary(tree, expr, id, |_, _, r2| Ok(Some(r2))),
            AstNode::GlobalDec { id: Identifier { name }} => {
                self.backend.glob_sym(&name).map(|_| None)
            }
        }
    }

    pub fn gen_preamble(&mut self) -> Result<()> {
        self.backend.preamble()
    }

    pub fn gen_freeregs(&mut self) -> Result<()> {
        self.backend.free_all_registers()
    }

    pub fn gen_globsym(&mut self, name: &str) -> Result<()> {
        self.backend.glob_sym(name)
    }
}

