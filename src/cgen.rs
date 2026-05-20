// Given an AST, generate code recursively

use anyhow::Result;
use crate::{
    ast::{AstNode, Identifier},
    sym::PrimType,
};

pub trait CodeBackend {
    type Reg: Copy;

    fn free_all_registers(&mut self) -> Result<()>;
    fn preamble(&mut self) -> Result<()>;
    fn func_postamble(&mut self) -> Result<()>;
    fn func_preamble(&mut self, ident: &str) -> Result<()>;
    fn load_int(&mut self, val: i64) -> Result<Self::Reg>;
    fn load_glob(&mut self, ident: &str, dtype: PrimType) -> Result<Self::Reg>;
    fn store_glob(&mut self, r: Self::Reg, ident: &str, dtype: PrimType) -> Result<Self::Reg>;
    fn glob_sym(&mut self, sym: &str, dtype: PrimType) -> Result<()>;
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
                AstNode::Ident { id,  .. } => {
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
            AstNode::Add { left, right, .. } => self.binary(tree, left, right, |cg, r1, r2| cg.add(r1, r2)),
            AstNode::Subtract { left, right, .. } => self.binary(tree, left, right, |cg, r1, r2| cg.sub(r1, r2)),
            AstNode::Multiply { left, right, .. } => self.binary(tree, left, right, |cg, r1, r2| cg.mul(r1, r2)),
            AstNode::Divide { left, right, .. } => self.binary(tree, left, right, |cg, r1, r2| cg.div(r1, r2)),
            AstNode::Equal { left, right, .. }|AstNode::NotEqual { left, right, .. }
                |AstNode::LessThan { left, right, .. }|AstNode::GreaterThan { left, right, .. }
                |AstNode::LessThanOrEqual { left, right, .. }
                |AstNode::GreaterThanOrEqual { left, right, .. } => {
                    if parent_op.is_some_and(AstNode::is_branching_stmt) {
                        self.binary(tree, left, right,
                            |cg: &mut B, r1, r2| cg.compare_and_jump(tree, r1, r2, label))
                    } else {
                        self.binary(tree, left, right,
                            |cg: &mut B, r1, r2| cg.compare_and_set(tree, r1, r2))
                    }
                },
            AstNode::IntLit { val, .. } => self.backend.load_int(*val).map(Some),
            AstNode::Ident{ id: Identifier { name }, dtype } => self.backend.load_glob(name, *dtype).map(Some),
            AstNode::LvIdent{ id: Identifier { name }, dtype } => self.backend.store_glob(r.unwrap(), name, *dtype).map(Some),
            // For Assign, all the work is done by the code generation down its branches.
            // We need only to return the right-branch register
            AstNode::Assign { id, expr } => self.binary(tree, expr, id, |_, _, r2| Ok(Some(r2))),
            AstNode::GlobalDec { id: Identifier { name }, dtype } => {
                self.backend.glob_sym(name, *dtype).map(|_| None)
            }
        }
    }

    pub fn gen_preamble(&mut self) -> Result<()> {
        self.backend.preamble()
    }

    pub fn gen_freeregs(&mut self) -> Result<()> {
        self.backend.free_all_registers()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cg::X86_64Backend;
    use crate::ast::AstNode;
    use crate::sym::PrimType;
    use crate::scan::Token;

    fn new_generator() -> CodeGenerator<X86_64Backend<Vec<u8>>> {
        CodeGenerator::new(X86_64Backend::new(Vec::new()))
    }

    fn output_string(cg: &CodeGenerator<X86_64Backend<Vec<u8>>>) -> String {
        String::from_utf8(cg.backend.output.clone()).unwrap()
    }

    // === Construction ===

    #[test]
    fn new_starts_with_last_label_zero() {
        let cg = new_generator();
        assert_eq!(cg.last_label, 0);
    }

    #[test]
    fn label_increments_counter() {
        let mut cg = new_generator();
        assert_eq!(cg.label(), 1);
        assert_eq!(cg.label(), 2);
        assert_eq!(cg.label(), 3);
    }

    // === gen_ast: literals and identifiers ===

    #[test]
    fn gen_ast_intlit_loads_value() {
        let mut cg = new_generator();
        let node = AstNode::make_intlit(42, PrimType::Int);
        let result = cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(result.is_some());
        assert!(output_string(&cg).contains("movq\t$42,"));
    }

    #[test]
    fn gen_ast_intlit_char_range() {
        let mut cg = new_generator();
        let node = AstNode::make_intlit(255, PrimType::Char);
        let result = cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(result.is_some());
        assert!(output_string(&cg).contains("movq\t$255,"));
    }

    #[test]
    fn gen_ast_ident_loads_glob() {
        let mut cg = new_generator();
        let node = AstNode::make_ident("x", PrimType::Int);
        let result = cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(result.is_some());
        assert!(output_string(&cg).contains("movq\tx(%rip),"));
    }

    #[test]
    fn gen_ast_ident_loads_char_glob() {
        let mut cg = new_generator();
        let node = AstNode::make_ident("c", PrimType::Char);
        let result = cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(result.is_some());
        assert!(output_string(&cg).contains("movzbq\tc(%rip),"));
    }

    #[test]
    fn gen_ast_empty_returns_none() {
        let mut cg = new_generator();
        let result = cg.gen_ast(&AstNode::Empty, None, None, 0).unwrap();
        assert!(result.is_none());
        assert!(output_string(&cg).is_empty());
    }

    #[test]
    fn gen_ast_global_dec_calls_glob_sym() {
        let mut cg = new_generator();
        let node = AstNode::make_global_declaration("x", PrimType::Int);
        let result = cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(result.is_none());
        assert_eq!(output_string(&cg), "\t.comm\tx,8,8\n");
    }

    // === gen_ast: arithmetic ===

    #[test]
    fn gen_ast_add_emits_addq() {
        let mut cg = new_generator();
        let node = AstNode::make_binary(
            Token::Plus,
            AstNode::make_intlit(1, PrimType::Char),
            AstNode::make_intlit(2, PrimType::Char),
            PrimType::Char,
        );
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("movq\t$1,"));
        assert!(output.contains("movq\t$2,"));
        assert!(output.contains("addq"));
    }

    #[test]
    fn gen_ast_sub_emits_subq() {
        let mut cg = new_generator();
        let node = AstNode::make_binary(
            Token::Minus,
            AstNode::make_intlit(5, PrimType::Int),
            AstNode::make_intlit(3, PrimType::Int),
            PrimType::Int,
        );
        cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(output_string(&cg).contains("subq"));
    }

    #[test]
    fn gen_ast_mul_emits_imulq() {
        let mut cg = new_generator();
        let node = AstNode::make_binary(
            Token::Star,
            AstNode::make_intlit(2, PrimType::Int),
            AstNode::make_intlit(4, PrimType::Int),
            PrimType::Int,
        );
        cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(output_string(&cg).contains("imulq"));
    }

    #[test]
    fn gen_ast_div_emits_idivq() {
        let mut cg = new_generator();
        let node = AstNode::make_binary(
            Token::Slash,
            AstNode::make_intlit(10, PrimType::Int),
            AstNode::make_intlit(2, PrimType::Int),
            PrimType::Int,
        );
        cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(output_string(&cg).contains("idivq"));
    }

    // === gen_ast: comparison (non-branching → compare_and_set) ===

    #[test]
    fn gen_ast_eq_emits_sete() {
        let mut cg = new_generator();
        let node = AstNode::make_binary(Token::EQ, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("cmpq"));
        assert!(output.contains("sete"));
    }

    #[test]
    fn gen_ast_lt_emits_setl() {
        let mut cg = new_generator();
        let node = AstNode::make_binary(Token::LT, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(output_string(&cg).contains("setl"));
    }

    // === gen_print ===

    #[test]
    fn gen_print_emits_printint_call() {
        let mut cg = new_generator();
        let node = AstNode::make_print(AstNode::make_intlit(42, PrimType::Int));
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("movq"));
        assert!(output.contains("call\tprintint"));
    }

    // === gen_glue_ast ===

    #[test]
    fn gen_glue_ast_processes_left_then_right() {
        let mut cg = new_generator();
        let node = AstNode::make_glue(
            AstNode::make_print(AstNode::make_intlit(1, PrimType::Int)),
            AstNode::make_print(AstNode::make_intlit(2, PrimType::Int)),
        );
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        let first = output.find("movq\t$1,").unwrap();
        let second = output.find("movq\t$2,").unwrap();
        assert!(first < second, "left print should precede right print");
    }

    // === gen_if (branching context → compare_and_jump) ===

    #[test]
    fn gen_if_without_else() {
        let mut cg = new_generator();
        let cond = AstNode::make_binary(Token::LT, AstNode::make_intlit(1, PrimType::Char), AstNode::make_intlit(2, PrimType::Char), PrimType::Char);
        let body = AstNode::make_print(AstNode::make_intlit(42, PrimType::Int));
        let node = AstNode::make_if(cond, body, None);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("call\tprintint"));
        assert!(output.contains("L1:"));
    }

    #[test]
    fn gen_if_with_else() {
        let mut cg = new_generator();
        let cond = AstNode::make_binary(Token::EQ, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        let true_branch = AstNode::make_print(AstNode::make_intlit(10, PrimType::Int));
        let false_branch = AstNode::make_print(AstNode::make_intlit(20, PrimType::Int));
        let node = AstNode::make_if(cond, true_branch, Some(false_branch));
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("jne"));
        assert!(output.contains("jmp\tL2"));
        assert!(output.contains("L1:"));
        assert!(output.contains("L2:"));
    }

    #[test]
    fn gen_if_nested() {
        let mut cg = new_generator();
        let inner_if = AstNode::make_if(
            AstNode::make_binary(Token::GT, AstNode::make_intlit(3, PrimType::Int), AstNode::make_intlit(1, PrimType::Int), PrimType::Int),
            AstNode::make_print(AstNode::make_intlit(100, PrimType::Int)),
            None,
        );
        let outer_cond = AstNode::make_binary(Token::EQ, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(1, PrimType::Int), PrimType::Int);
        let node = AstNode::make_if(outer_cond, inner_if, None);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        // Should have two sets of labels
        assert!(output.contains("L1:"));
        assert!(output.contains("L2:"));
        assert!(output.contains("printint"));
    }

    // === gen_while ===

    #[test]
    fn gen_while_emits_loop() {
        let mut cg = new_generator();
        let cond = AstNode::make_binary(Token::NE, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(1, PrimType::Int), PrimType::Int);
        let body = AstNode::make_print(AstNode::make_intlit(7, PrimType::Int));
        let node = AstNode::make_while(cond, body);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("L1:"));
        assert!(output.contains("je"));
        assert!(output.contains("call\tprintint"));
        assert!(output.contains("jmp\tL1"));
        assert!(output.contains("L2:"));
    }

    #[test]
    fn gen_while_empty_body() {
        let mut cg = new_generator();
        let cond = AstNode::make_binary(Token::EQ, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(1, PrimType::Int), PrimType::Int);
        let node = AstNode::make_while(cond, AstNode::Empty);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("L1:"));
        assert!(output.contains("L2:"));
    }

    // === gen_ast: Assign ===

    #[test]
    fn gen_ast_assign_stores_int() {
        let mut cg = new_generator();
        let id = AstNode::make_lvident("x", PrimType::Int);
        let expr = AstNode::make_intlit(42, PrimType::Int);
        let node = AstNode::make_assign(id, expr);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("movq\t$42,"));
        assert!(output.contains("x(%rip)"));
    }

    #[test]
    fn gen_ast_assign_stores_char_byte() {
        let mut cg = new_generator();
        let id = AstNode::make_lvident("c", PrimType::Char);
        let expr = AstNode::make_intlit(7, PrimType::Char);
        let node = AstNode::make_assign(id, expr);
        cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(output_string(&cg).contains("movb"));
    }

    #[test]
    fn gen_ast_assign_with_expression() {
        let mut cg = new_generator();
        let sum = AstNode::make_binary(Token::Plus, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        let id = AstNode::make_lvident("x", PrimType::Int);
        let node = AstNode::make_assign(id, sum);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("addq"));
        assert!(output.contains("x(%rip)"));
    }

    // === gen_function ===

    #[test]
    fn gen_function_emits_complete_function() {
        let mut cg = new_generator();
        cg.gen_preamble().unwrap();
        let name = AstNode::make_ident("main", PrimType::Void);
        let body = AstNode::make_print(AstNode::make_intlit(42, PrimType::Int));
        let node = AstNode::make_function(name, body);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("main:"));
        assert!(output.contains("call\tprintint"));
        assert!(output.contains("ret"));
    }

    #[test]
    fn gen_function_with_glue_body() {
        let mut cg = new_generator();
        let name = AstNode::make_ident("myfunc", PrimType::Void);
        let body = AstNode::make_glue(
            AstNode::make_global_declaration("x", PrimType::Int),
            AstNode::make_assign(
                AstNode::make_lvident("x", PrimType::Int),
                AstNode::make_intlit(99, PrimType::Int),
            ),
        );
        let node = AstNode::make_function(name, body);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("myfunc:"));
        assert!(output.contains(".comm\tx,8,8"));
        assert!(output.contains("movq\t$99,"));
        assert!(output.contains("ret"));
    }

    // === gen_preamble / gen_freeregs ===

    #[test]
    fn gen_preamble_writes_printint() {
        let mut cg = new_generator();
        cg.gen_preamble().unwrap();
        assert!(output_string(&cg).contains("printint:"));
    }

    #[test]
    fn gen_freeregs_does_not_panic() {
        let mut cg = new_generator();
        cg.gen_ast(&AstNode::make_intlit(1, PrimType::Int), None, None, 0).unwrap();
        cg.gen_freeregs().unwrap();
    }

    // === Integration: function with print and assignment ===

    #[test]
    fn function_with_var_decl_and_assign_and_print() {
        let mut cg = new_generator();
        cg.gen_preamble().unwrap();
        let name = AstNode::make_ident("main", PrimType::Void);
        let body = AstNode::make_glue(
            AstNode::make_glue(
                AstNode::make_global_declaration("x", PrimType::Int),
                AstNode::make_assign(
                    AstNode::make_lvident("x", PrimType::Int),
                    AstNode::make_intlit(100, PrimType::Int),
                ),
            ),
            AstNode::make_print(AstNode::make_ident("x", PrimType::Int)),
        );
        let node = AstNode::make_function(name, body);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("main:"));
        assert!(output.contains(".comm\tx,8,8"));
        assert!(output.contains("movq\t$100,"));
        assert!(output.contains("movq\tx(%rip),"));
        assert!(output.contains("call\tprintint"));
        assert!(output.contains("ret"));
    }
}

