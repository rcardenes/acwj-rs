// Given an AST, generate code recursively

use std::cell::RefCell;
use anyhow::Result;
use crate::{
    ast::{AstNode, Identifier},
    sym::{PrimType, SymbolTable},
};

pub trait CodeBackend {
    type Reg: Copy;

    fn free_all_registers(&mut self) -> Result<()>;
    fn preamble(&mut self) -> Result<()>;
    fn type_size(&self, dtype: PrimType) -> usize;
    fn func_preamble(&mut self, ident: &str) -> Result<()>;
    fn func_postamble(&mut self, label_num: usize) -> Result<()>;
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
    fn call(&mut self, r: Self::Reg, ident: &str) -> Result<Option<Self::Reg>>;
    fn ret(&mut self, r: Self::Reg, dtype: PrimType, label_num: usize) -> Result<()>;
}

pub struct CodeGenerator<'a, B>
where
    B: CodeBackend,
{
    backend: B,
    symbols: &'a SymbolTable,
    last_label: RefCell<usize>,
}

impl<'a, B> CodeGenerator<'a, B>
where
    B: CodeBackend,
{
    pub fn new(backend: B, symbols: &'a SymbolTable) -> Self {
        Self { backend, symbols, last_label: 0.into() }
    }

    pub fn label(&self) -> usize {
        let mut borrow = self.last_label.borrow_mut();
        *borrow += 1;
        *borrow
    }

    pub fn get_last_label(&self) -> usize {
        *self.last_label.borrow()
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

    fn gen_func_call(&mut self, tree: &AstNode) -> Result<Option<B::Reg>> {
        let AstNode::FuncCall { id, left, .. } = tree else { unreachable!("We know this is always FuncCall") };
        let ident = &id.name;
        let Some(reg) = self.gen_ast(left, None, Some(tree), 0)? else { unreachable!("Call to {ident} without an expression") };
        self.backend.call(reg, ident)
    }

    fn gen_return(&mut self, tree: &AstNode) -> Result<Option<B::Reg>> {
        let AstNode::Return { id: Identifier { name }, expr } = tree else {
            unreachable!("We know this is always Return")
        };

        let label_num = self.symbols.get_fn_end_label(name);
        let dtype = self.symbols.get_fn_dtype(name);

        let Some(reg) = self.gen_ast(expr, None, Some(tree), 0)? else {
            unreachable!("Return always gets an expression")
        };

        self.backend.ret(reg, dtype, label_num).map(|_| None)
    }

    pub fn gen_if(&mut self, tree: &AstNode) -> Result<Option<B::Reg>> {
        let AstNode::If { cond, branch_t, branch_f } = tree else {
            unreachable!("Invalid input for gen_if!")
        };

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
        if let AstNode::Function { name, body} = tree {
            let ident = match &**name {
                AstNode::Ident { id,  .. } => id.name.as_str(),
                _ => unreachable!("Function with invalid id: {:?}", name)
            };

            self.backend.func_preamble(ident)?;
            let end_label = self.label();
            self.symbols.set_fn_end_label(ident, end_label);

            self.gen_ast(body, None, Some(tree), 0)?;

            self.backend.func_postamble(end_label)?;
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
            // AstNode::GlobalDec { id: Identifier { name }, dtype } => {
            //     self.backend.glob_sym(name, *dtype).map(|_| None)
            // },
            AstNode::GlobalDec { ..} => {
                // Done elsewhere to ensure all globals are at the top of the output, instead of
                // interspersed with the function code
                Ok(None)
            }
            AstNode::FuncCall { .. } => self.gen_func_call(tree),
            AstNode::Return { .. } => self.gen_return(tree),
        }
    }

    pub fn gen_preamble(&mut self) -> Result<()> {
        self.backend.preamble()?;

        for symbol in self.symbols.iter_global_vars() {
            self.backend.glob_sym(&symbol.name, symbol.dtype)?;
        }

        Ok(())
    }

    pub fn gen_freeregs(&mut self) -> Result<()> {
        self.backend.free_all_registers()
    }

    pub fn type_size(&self, dtype: PrimType) -> usize {
        self.backend.type_size(dtype)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};
    use crate::{
        cg::X86_64Backend,
        ast::AstNode,
        sym::SymbolTableBuilder,
        scan::Token,
    };

    #[fixture]
    fn symbols() -> SymbolTable {
        SymbolTableBuilder::new()
            .add_glob_fn("println", PrimType::Void)
            .build()
    }

    fn new_generator<'a>(symbols: &'a SymbolTable) -> CodeGenerator<'a, X86_64Backend<Vec<u8>>> {
        CodeGenerator::new(X86_64Backend::new(Vec::new()), symbols)
    }

    fn output_string(cg: &CodeGenerator<X86_64Backend<Vec<u8>>>) -> String {
        String::from_utf8(cg.backend.output.clone()).unwrap()
    }

    // === Construction ===

    #[rstest]
    fn new_starts_with_last_label_zero(symbols: SymbolTable) {
        let cg = new_generator(&symbols);
        assert_eq!(cg.get_last_label(), 0);
    }

    #[rstest]
    fn label_increments_counter(symbols: SymbolTable) {
        let cg = new_generator(&symbols);
        assert_eq!(cg.label(), 1);
        assert_eq!(cg.label(), 2);
        assert_eq!(cg.label(), 3);
    }

    // === gen_ast: literals and identifiers ===

    #[rstest]
    fn gen_ast_intlit_loads_value(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let node = AstNode::make_intlit(42, PrimType::Int);
        let result = cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(result.is_some());
        assert!(output_string(&cg).contains("movq\t$42,"));
    }

    #[rstest]
    fn gen_ast_intlit_char_range(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let node = AstNode::make_intlit(255, PrimType::Char);
        let result = cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(result.is_some());
        assert!(output_string(&cg).contains("movq\t$255,"));
    }

    #[rstest]
    fn gen_ast_ident_loads_glob(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let node = AstNode::make_ident("x", PrimType::Long);
        let result = cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(result.is_some());
        assert!(output_string(&cg).contains("movq\tx(%rip),"));
    }

    #[rstest]
    fn gen_ast_ident_loads_char_glob(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let node = AstNode::make_ident("c", PrimType::Char);
        let result = cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(result.is_some());
        assert!(output_string(&cg).contains("movzbq\tc(%rip),"));
    }

    #[rstest]
    fn gen_ast_empty_returns_none(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let result = cg.gen_ast(&AstNode::Empty, None, None, 0).unwrap();
        assert!(result.is_none());
        assert!(output_string(&cg).is_empty());
    }

    // === gen_ast: arithmetic ===

    #[rstest]
    fn gen_ast_add_emits_addq(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
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

    #[rstest]
    fn gen_ast_sub_emits_subq(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let node = AstNode::make_binary(
            Token::Minus,
            AstNode::make_intlit(5, PrimType::Int),
            AstNode::make_intlit(3, PrimType::Int),
            PrimType::Int,
        );
        cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(output_string(&cg).contains("subq"));
    }

    #[rstest]
    fn gen_ast_mul_emits_imulq(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let node = AstNode::make_binary(
            Token::Star,
            AstNode::make_intlit(2, PrimType::Int),
            AstNode::make_intlit(4, PrimType::Int),
            PrimType::Int,
        );
        cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(output_string(&cg).contains("imulq"));
    }

    #[rstest]
    fn gen_ast_div_emits_idivq(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
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

    #[rstest]
    fn gen_ast_eq_emits_sete(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let node = AstNode::make_binary(Token::EQ, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("cmpq"));
        assert!(output.contains("sete"));
    }

    #[rstest]
    fn gen_ast_lt_emits_setl(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let node = AstNode::make_binary(Token::LT, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(output_string(&cg).contains("setl"));
    }

    // === gen_glue_ast ===

    #[rstest]
    fn gen_glue_ast_processes_left_then_right(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let node = AstNode::make_glue(
            AstNode::make_function_call("printint", AstNode::make_intlit(1, PrimType::Long), PrimType::Long),
            AstNode::make_function_call("printint", AstNode::make_intlit(2, PrimType::Long), PrimType::Long),
        );
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        let first = output.find("movq\t$1,").unwrap();
        let second = output.find("movq\t$2,").unwrap();
        assert!(first < second, "left print should precede right print");
    }

    // === gen_if (branching context → compare_and_jump) ===

    #[rstest]
    fn gen_if_without_else(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let cond = AstNode::make_binary(Token::LT, AstNode::make_intlit(1, PrimType::Char), AstNode::make_intlit(2, PrimType::Char), PrimType::Char);
        let body = AstNode::make_function_call("printint", AstNode::make_intlit(42, PrimType::Int), PrimType::Int);
        let node = AstNode::make_if(cond, body, None);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("call\tprintint"));
        assert!(output.contains("L1:"));
    }

    #[rstest]
    fn gen_if_with_else(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let cond = AstNode::make_binary(Token::EQ, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        let true_branch = AstNode::make_function_call("printint", AstNode::make_intlit(10, PrimType::Int), PrimType::Int);
        let false_branch = AstNode::make_function_call("printint", AstNode::make_intlit(20, PrimType::Int), PrimType::Int);
        let node = AstNode::make_if(cond, true_branch, Some(false_branch));
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("jne"));
        assert!(output.contains("jmp\tL2"));
        assert!(output.contains("L1:"));
        assert!(output.contains("L2:"));
    }

    #[rstest]
    fn gen_if_nested(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let inner_if = AstNode::make_if(
            AstNode::make_binary(Token::GT, AstNode::make_intlit(3, PrimType::Int), AstNode::make_intlit(1, PrimType::Int), PrimType::Int),
            AstNode::make_function_call("printint", AstNode::make_intlit(100, PrimType::Int), PrimType::Int),
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

    #[rstest]
    fn gen_while_emits_loop(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let cond = AstNode::make_binary(Token::NE, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(1, PrimType::Int), PrimType::Int);
        let body = AstNode::make_function_call("printint", AstNode::make_intlit(7, PrimType::Int), PrimType::Int);
        let node = AstNode::make_while(cond, body);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("L1:"));
        assert!(output.contains("je"));
        assert!(output.contains("call\tprintint"));
        assert!(output.contains("jmp\tL1"));
        assert!(output.contains("L2:"));
    }

    #[rstest]
    fn gen_while_empty_body(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let cond = AstNode::make_binary(Token::EQ, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(1, PrimType::Int), PrimType::Int);
        let node = AstNode::make_while(cond, AstNode::Empty);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("L1:"));
        assert!(output.contains("L2:"));
    }

    // === gen_ast: Assign ===

    #[rstest]
    fn gen_ast_assign_stores_int(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let id = AstNode::make_lvident("x", PrimType::Int);
        let expr = AstNode::make_intlit(42, PrimType::Int);
        let node = AstNode::make_assign(id, expr);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("movq\t$42,"));
        assert!(output.contains("x(%rip)"));
    }

    #[rstest]
    fn gen_ast_assign_stores_char_byte(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let id = AstNode::make_lvident("c", PrimType::Char);
        let expr = AstNode::make_intlit(7, PrimType::Char);
        let node = AstNode::make_assign(id, expr);
        cg.gen_ast(&node, None, None, 0).unwrap();
        assert!(output_string(&cg).contains("movb"));
    }

    #[rstest]
    fn gen_ast_assign_with_expression(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        let sum = AstNode::make_binary(Token::Plus, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        let id = AstNode::make_lvident("x", PrimType::Int);
        let node = AstNode::make_assign(id, sum);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("addq"));
        assert!(output.contains("x(%rip)"));
    }

    // === gen_function ===

    #[rstest]
    fn gen_function_emits_complete_function(symbols: SymbolTable) {
        symbols.add_glob_fn("main", PrimType::Int);
        let mut cg = new_generator(&symbols);
        cg.gen_preamble().unwrap();
        let name = AstNode::make_ident("main", PrimType::Void);
        let body = AstNode::make_function_call("printint", AstNode::make_intlit(42, PrimType::Int), PrimType::Int);
        let node = AstNode::make_function(name, body);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("main:"));
        assert!(output.contains("call\tprintint"));
        assert!(output.contains("ret"));
    }

    #[rstest]
    fn gen_function_with_glue_body(symbols: SymbolTable) {
        symbols.add_glob_fn("myfunc", PrimType::Void);
        let mut cg = new_generator(&symbols);
        let name = AstNode::make_ident("myfunc", PrimType::Void);
        let body = AstNode::make_glue(
            AstNode::make_global_declaration("x", PrimType::Long),
            AstNode::make_assign(
                AstNode::make_lvident("x", PrimType::Long),
                AstNode::make_intlit(99, PrimType::Long),
            ),
        );
        let node = AstNode::make_function(name, body);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("myfunc:"));
        // Global declarations no longer produce code on the spot
        assert!(!output.contains(".comm\tx,8,8"));
        assert!(output.contains("movq\t$99,"));
        assert!(output.contains("ret"));
    }

    // === gen_preamble / gen_freeregs ===

    #[rstest]
    fn gen_freeregs_does_not_panic(symbols: SymbolTable) {
        let mut cg = new_generator(&symbols);
        cg.gen_ast(&AstNode::make_intlit(1, PrimType::Int), None, None, 0).unwrap();
        cg.gen_freeregs().unwrap();
    }

    // === Integration: function with print and assignment ===

    #[rstest]
    fn function_with_var_decl_and_assign_and_print(symbols: SymbolTable) {
        symbols.add_glob_fn("main", PrimType::Int);
        let mut cg = new_generator(&symbols);
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
            AstNode::make_function_call("printint", AstNode::make_ident("x", PrimType::Int), PrimType::Int),
        );
        let node = AstNode::make_function(name, body);
        cg.gen_ast(&node, None, None, 0).unwrap();
        let output = output_string(&cg);
        assert!(output.contains("main:"));
        // Global declarations no longer generate output on the spot
        assert!(!output.contains(".comm\tx,4,4"));
        assert!(output.contains("movq\t$100,"));
        assert!(output.contains("movzbl\tx(%rip),"));
        assert!(output.contains("call\tprintint"));
        assert!(output.contains("ret"));
    }
}

