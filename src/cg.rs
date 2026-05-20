use std::{
    collections::HashMap,
    fmt,
    io::Write,
};
use anyhow::Result;
use crate::{
    ast::AstNode,
    cgen::CodeBackend,
    sym::PrimType,
};

static PREAMBLE: &str = "
\t.text
.LC0:
\t.string\t\"%d\\n\"
printint:
\tpushq\t%rbp
\tmovq\t%rsp, %rbp
\tsubq\t$16, %rsp
\tmovl\t%edi, -4(%rbp)
\tmovl\t-4(%rbp), %eax
\tmovl\t%eax, %esi
\tleaq\t.LC0(%rip), %rdi
\tmovl\t$0, %eax
\tcall\tprintf@PLT
\tnop
\tleave
\tret
";

static FUNC_PREAMBLE: &str = "
\t.text
\t.globl\t{name}
\t.type\t{name} @function
{name}:
\tpushq\t%rbp
\tmovq\t%rsp, %rbp
";

static POSTAMBLE: &str = "
\tmovl\t$0, %eax
\tpopq\t%rbp
\tret
";

enum X86_64Reg8b {
    R8,
    R9,
    R10,
    R11,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum X86_64Reg {
    R8,
    R9,
    R10,
    R11,
}

impl X86_64Reg {
    fn as_8b(&self) -> X86_64Reg8b {
        match self {
            X86_64Reg::R8 => X86_64Reg8b::R8,
            X86_64Reg::R9 => X86_64Reg8b::R9,
            X86_64Reg::R10 => X86_64Reg8b::R10,
            X86_64Reg::R11 => X86_64Reg8b::R11,
        }
    }
}

impl fmt::Display for X86_64Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            X86_64Reg::R8 => write!(f, "%r8"),
            X86_64Reg::R9 => write!(f, "%r9"),
            X86_64Reg::R10 => write!(f, "%r10"),
            X86_64Reg::R11 => write!(f, "%r11"),
        }
    }
}

impl fmt::Display for X86_64Reg8b {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            X86_64Reg8b::R8 => write!(f, "%r8b"),
            X86_64Reg8b::R9 => write!(f, "%r9b"),
            X86_64Reg8b::R10 => write!(f, "%r10b"),
            X86_64Reg8b::R11 => write!(f, "%r11b"),
        }
    }
}

fn ast_to_jmp_op(op: &AstNode) -> &'static str {
    match op {
        AstNode::Equal {..} => "jne",
        AstNode::NotEqual {..} => "je",
        AstNode::LessThan {..} => "jge",
        AstNode::LessThanOrEqual {..} => "jg",
        AstNode::GreaterThan {..} => "jle",
        AstNode::GreaterThanOrEqual {..} => "jl",
        _ => panic!("Not a comparison operator: {:?}", op)
    }
}

fn ast_to_set_op(op: &AstNode) -> &'static str {
    match op {
        AstNode::Equal {..} => "sete",
        AstNode::NotEqual {..} => "setne",
        AstNode::LessThan {..} => "setl",
        AstNode::LessThanOrEqual {..} => "setle",
        AstNode::GreaterThan {..} => "setg",
        AstNode::GreaterThanOrEqual {..} => "setge",
        _ => panic!("Not a comparison operator: {:?}", op)
    }
}

pub struct X86_64Backend<T>
    where T: Write
{
    pub(crate) output: T,
    pub(crate) reg_status: HashMap<X86_64Reg, bool>,
}

impl<T> X86_64Backend<T>
    where T: Write,
{

    pub fn new(output: T) -> Self {
        X86_64Backend {
            output,
            reg_status: HashMap::from([
                           (X86_64Reg::R8, true),
                           (X86_64Reg::R9, true),
                           (X86_64Reg::R10, true),
                           (X86_64Reg::R11, true)]),
        }
    }

    fn free_register(&mut self, reg: X86_64Reg) {
        self.reg_status.entry(reg).insert_entry(true);
    }

    fn alloc_register(&mut self) -> X86_64Reg {
        for (reg, free) in self.reg_status.iter_mut() {
            if *free {
                *free = false;
                return *reg
            }
        }
        panic!("Out of registers!")
    }
}

impl<T> CodeBackend for X86_64Backend<T>
    where T: Write
{
    type Reg = X86_64Reg;

    fn free_all_registers(&mut self) -> Result<()> {
        for val in self.reg_status.values_mut() {
            *val = true;
        }

        Ok(())
    }

    fn preamble(&mut self) -> Result<()> {
        self.free_all_registers()?;
        write!(self.output, "{}", PREAMBLE)?;
        Ok(())
    }

    fn func_preamble(&mut self, ident: &str) -> Result<()> {
        let preamble = FUNC_PREAMBLE.replace("{name}", ident);
        write!(self.output, "{}", preamble)?;
        Ok(())
    }

    fn func_postamble(&mut self) -> Result<()> {
        write!(self.output, "{}", POSTAMBLE)?;
        Ok(())
    }

    fn load_int(&mut self, val: i64) -> Result<Self::Reg> {
        let reg = self.alloc_register();
        writeln!(self.output, "\tmovq\t${}, {}", val, reg)?;

        Ok(reg)
    }

    fn load_glob(&mut self, ident: &str, dtype: PrimType) -> Result<Self::Reg> {
        // Get a new register
        let reg = self.alloc_register();

        match dtype {
            PrimType::Int => writeln!(self.output, "\tmovq\t{}(%rip), {}", ident, reg)?,
            PrimType::Char => writeln!(self.output, "\tmovzbq\t{}(%rip), {}", ident, reg)?,
            PrimType::Void => unreachable!("Can't generate load_glob for void types!"),
        }

        Ok(reg)
    }

    fn store_glob(&mut self, r: Self::Reg, ident: &str, dtype: PrimType) -> Result<Self::Reg> {
        match dtype {
            PrimType::Int => writeln!(self.output, "\tmovq\t{}, {}(%rip)", r, ident)?,
            PrimType::Char => writeln!(self.output, "\tmovb\t{}, {}(%rip)", r.as_8b(), ident)?,
            PrimType::Void => unreachable!("Can't generate store_glob for void types!"),
        }

        Ok(r)
    }

    fn glob_sym(&mut self, sym: &str, dtype: PrimType) -> Result<()> {
        match dtype {
            PrimType::Int => writeln!(self.output, "\t.comm\t{},8,8", sym)?,
            PrimType::Char => writeln!(self.output, "\t.comm\t{},1,1", sym)?,
            PrimType::Void => {}, // Node code generation for void type
        }

        Ok(())
    }

    // Add two registers together and return
    // the number of the register with the result
    fn add(&mut self, r1: Self::Reg, r2: Self::Reg) -> Result<Option<Self::Reg>> {
        writeln!(self.output, "\taddq\t{}, {}", r1, r2)?;
        self.free_register(r1);

        Ok(Some(r2))
    }

    // Subtract the second register from the first and
    // return the number of the register with the result
    fn sub(&mut self, r1: Self::Reg, r2: Self::Reg) -> Result<Option<Self::Reg>> {
        writeln!(self.output, "\tsubq\t{}, {}", r2, r1)?;
        self.free_register(r2);

        Ok(Some(r1))
    }

    // Multiply two registers together and return
    // the number of the register with the result
    fn mul(&mut self, r1: Self::Reg, r2: Self::Reg) -> Result<Option<Self::Reg>> {
        writeln!(self.output, "\timulq\t{}, {}", r1, r2)?;
        self.free_register(r1);

        Ok(Some(r2))
    }

    // Divide the first register by the second and
    // regurn the number of the register with the result
    fn div(&mut self, r1: Self::Reg, r2: Self::Reg) -> Result<Option<Self::Reg>> {
        writeln!(self.output, "\tmovq\t{r1}, %rax")?;
        writeln!(self.output, "\tcqo")?;
        writeln!(self.output, "\tidivq\t{r2}")?;
        writeln!(self.output, "\tmovq\t%rax, {r1}")?;
        self.free_register(r2);

        Ok(Some(r1))
    }

    fn label(&mut self, label_num: usize) -> Result<()> {
        writeln!(self.output, "L{}:", label_num)?;
        Ok(())
    }

    fn jump(&mut self, label_num: usize) -> Result<()> {
        writeln!(self.output, "\tjmp\tL{}", label_num)?;
        Ok(())
    }

    fn compare_and_set(&mut self, op: &AstNode, r1: Self::Reg, r2: Self::Reg) -> Result<Option<Self::Reg>> {
        let r2_8b = r2.as_8b();

        writeln!(self.output, "\tcmpq\t{}, {}", r2, r1)?;
        writeln!(self.output, "\t{}\t{}", ast_to_set_op(op), r2_8b)?;
        writeln!(self.output, "\tmovzbq\t{}, {}", r2_8b, r2)?;
        self.free_register(r1);

        Ok(Some(r2))
    }

    fn compare_and_jump(&mut self, op: &AstNode, r1: Self::Reg, r2: Self::Reg, label_num: usize) -> Result<Option<Self::Reg>> {
        writeln!(self.output, "\tcmpq\t{}, {}", r2, r1)?;
        writeln!(self.output, "\t{}\tL{}", ast_to_jmp_op(op), label_num)?;
        self.free_all_registers()?;

        Ok(None)
    }


    fn print_int(&mut self, r: Self::Reg) -> Result<()> {
        writeln!(self.output, "\tmovq\t{r}, %rdi")?;
        writeln!(self.output, "\tcall\tprintint")?;
        self.free_register(r);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};
    use crate::ast::AstNode;
    use crate::sym::PrimType;
    use crate::scan::Token;

    type Backend = X86_64Backend<Vec<u8>>;

    #[fixture]
    fn backend() -> Backend {
        X86_64Backend::new(Vec::new())
    }

    fn output_string(backend: &Backend) -> String {
        String::from_utf8(backend.output.clone()).unwrap()
    }

    // === Register management ===

    fn all_free(backend: &Backend) -> bool {
        backend.reg_status.values().all(|&x| x)
    }

    #[rstest]
    fn new_has_all_registers_free(backend: Backend) {
        assert_eq!(backend.reg_status.len(), 4);
        assert!(all_free(&backend));
    }

    #[rstest]
    fn alloc_register_marks_reg_unavailable(mut backend: Backend) {
        let reg = backend.alloc_register();
        assert!(!backend.reg_status[&reg]);
    }

    #[rstest]
    fn alloc_register_returns_unique_registers(mut backend: Backend) {
        let regs: std::collections::HashSet<X86_64Reg> = (0..4).map(|_| backend.alloc_register()).collect();
        assert_eq!(regs.len(), 4);
    }

    #[rstest]
    #[should_panic(expected = "Out of registers!")]
    fn alloc_register_panics_when_exhausted(mut backend: Backend) {
        for _ in 0..5 {
            backend.alloc_register();
        }
    }

    #[rstest]
    fn free_register_restores_availability(mut backend: Backend) {
        let reg = backend.alloc_register();
        assert!(!backend.reg_status[&reg]);
        backend.free_register(reg);
        assert!(backend.reg_status[&reg]);
    }

    #[rstest]
    fn free_all_registers_frees_all(mut backend: Backend) {
        let _ = backend.alloc_register();
        let _ = backend.alloc_register();
        backend.free_all_registers().unwrap();
        assert!(all_free(&backend));
    }

    // === Preamble / Postamble ===

    #[rstest]
    fn preamble_writes_printint_routine(mut backend: Backend) {
        backend.preamble().unwrap();
        let output = output_string(&backend);
        assert!(output.contains("printint:"));
        assert!(output.contains("printf@PLT"));
    }

    #[rstest]
    fn preamble_frees_all_registers(mut backend: Backend) {
        let _ = backend.alloc_register();
        backend.preamble().unwrap();
        assert!(all_free(&backend));
    }

    #[rstest]
    fn func_preamble_writes_label_with_name(mut backend: Backend) {
        backend.func_preamble("main").unwrap();
        let output = output_string(&backend);
        assert!(output.contains("main:"));
        assert!(output.contains(".globl\tmain"));
    }

    #[rstest]
    fn func_preamble_writes_underscore_name(mut backend: Backend) {
        backend.func_preamble("my_func").unwrap();
        let output = output_string(&backend);
        assert!(output.contains("my_func:"));
    }

    #[rstest]
    fn func_postamble_writes_return(mut backend: Backend) {
        backend.func_postamble().unwrap();
        let output = output_string(&backend);
        assert!(output.contains("popq"));
        assert!(output.contains("ret"));
    }

    // === Load / Store ===

    #[rstest]
    fn load_int_writes_movq(mut backend: Backend) {
        let reg = backend.load_int(42).unwrap();
        assert!(!backend.reg_status[&reg]);
        let output = output_string(&backend);
        assert!(output.contains("movq\t$42,"));
    }

    #[rstest]
    fn load_int_negative(mut backend: Backend) {
        backend.load_int(-7).unwrap();
        let output = output_string(&backend);
        assert!(output.contains("movq\t$-7,"));
    }

    #[rstest]
    fn load_int_large(mut backend: Backend) {
        backend.load_int(65536).unwrap();
        let output = output_string(&backend);
        assert!(output.contains("movq\t$65536,"));
    }

    #[rstest]
    fn load_glob_int(mut backend: Backend) {
        backend.load_glob("x", PrimType::Int).unwrap();
        let output = output_string(&backend);
        assert!(output.contains("movq\tx(%rip),"));
    }

    #[rstest]
    fn load_glob_char(mut backend: Backend) {
        backend.load_glob("c", PrimType::Char).unwrap();
        let output = output_string(&backend);
        assert!(output.contains("movzbq\tc(%rip),"));
    }

    #[rstest]
    #[should_panic(expected = "Can't generate load_glob")]
    fn load_glob_void_panics(mut backend: Backend) {
        let _ = backend.load_glob("v", PrimType::Void);
    }

    #[rstest]
    fn store_glob_int(mut backend: Backend) {
        let reg = backend.alloc_register();
        let ret = backend.store_glob(reg, "x", PrimType::Int).unwrap();
        assert_eq!(ret, reg);
        let output = output_string(&backend);
        assert!(output.contains(&format!("\tmovq\t{}, x(%rip)", reg)));
    }

    #[rstest]
    fn store_glob_char(mut backend: Backend) {
        let reg = backend.alloc_register();
        let ret = backend.store_glob(reg, "c", PrimType::Char).unwrap();
        assert_eq!(ret, reg);
        let output = output_string(&backend);
        assert!(output.contains(&format!("\tmovb\t{}, c(%rip)", reg.as_8b())));
    }

    #[rstest]
    #[should_panic(expected = "Can't generate store_glob")]
    fn store_glob_void_panics(mut backend: Backend) {
        let reg = backend.alloc_register();
        let _ = backend.store_glob(reg, "v", PrimType::Void);
    }

    // === Glob symbols ===

    #[rstest]
    fn glob_sym_int(mut backend: Backend) {
        backend.glob_sym("x", PrimType::Int).unwrap();
        assert_eq!(output_string(&backend), "\t.comm\tx,8,8\n");
    }

    #[rstest]
    fn glob_sym_char(mut backend: Backend) {
        backend.glob_sym("c", PrimType::Char).unwrap();
        assert_eq!(output_string(&backend), "\t.comm\tc,1,1\n");
    }

    #[rstest]
    fn glob_sym_void_emits_nothing(mut backend: Backend) {
        backend.glob_sym("v", PrimType::Void).unwrap();
        assert!(output_string(&backend).is_empty());
    }

    // === Arithmetic ===

    #[rstest]
    fn add_frees_r1_returns_r2(mut backend: Backend) {
        let r1 = backend.alloc_register();
        let r2 = backend.alloc_register();
        let result = backend.add(r1, r2).unwrap();
        assert_eq!(result, Some(r2));
        assert!(backend.reg_status[&r1]);
        assert!(!backend.reg_status[&r2]);
        assert!(output_string(&backend).contains(&format!("\taddq\t{}, {}", r1, r2)));
    }

    #[rstest]
    fn sub_frees_r2_returns_r1(mut backend: Backend) {
        let r1 = backend.alloc_register();
        let r2 = backend.alloc_register();
        let result = backend.sub(r1, r2).unwrap();
        assert_eq!(result, Some(r1));
        assert!(!backend.reg_status[&r1]);
        assert!(backend.reg_status[&r2]);
        assert!(output_string(&backend).contains(&format!("\tsubq\t{}, {}", r2, r1)));
    }

    #[rstest]
    fn mul_frees_r1_returns_r2(mut backend: Backend) {
        let r1 = backend.alloc_register();
        let r2 = backend.alloc_register();
        let result = backend.mul(r1, r2).unwrap();
        assert_eq!(result, Some(r2));
        assert!(backend.reg_status[&r1]);
        assert!(!backend.reg_status[&r2]);
        assert!(output_string(&backend).contains(&format!("\timulq\t{}, {}", r1, r2)));
    }

    #[rstest]
    fn div_frees_r2_returns_r1(mut backend: Backend) {
        let r1 = backend.alloc_register();
        let r2 = backend.alloc_register();
        let result = backend.div(r1, r2).unwrap();
        assert_eq!(result, Some(r1));
        assert!(!backend.reg_status[&r1]);
        assert!(backend.reg_status[&r2]);
        let output = output_string(&backend);
        assert!(output.contains("idivq"));
        assert!(output.contains("movq\t%rax,"));
    }

    // === Labels and jumps ===

    #[rstest]
    fn label_writes_label(mut backend: Backend) {
        backend.label(42).unwrap();
        assert_eq!(output_string(&backend), "L42:\n");
    }

    #[rstest]
    fn jump_writes_jmp(mut backend: Backend) {
        backend.jump(7).unwrap();
        assert_eq!(output_string(&backend), "\tjmp\tL7\n");
    }

    // === Comparisons: compare_and_set ===

    #[test]
    fn compare_and_set_all_ops() {
        let cases: [(AstNode, &str); 6] = [
            (AstNode::make_binary(Token::EQ, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "sete"),
            (AstNode::make_binary(Token::NE, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "setne"),
            (AstNode::make_binary(Token::LT, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "setl"),
            (AstNode::make_binary(Token::GT, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "setg"),
            (AstNode::make_binary(Token::LE, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "setle"),
            (AstNode::make_binary(Token::GE, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "setge"),
        ];
        for (op, expected_set) in &cases {
            let mut backend = backend();
            let r1 = backend.alloc_register();
            let r2 = backend.alloc_register();
            let result = backend.compare_and_set(op, r1, r2).unwrap();
            assert_eq!(result, Some(r2));
            let output = output_string(&backend);
            assert!(output.contains("cmpq"), "cmpq missing for {expected_set}");
            assert!(output.contains(expected_set), "missing {expected_set}");
            assert!(output.contains("movzbq"), "movzbq missing for {expected_set}");
            assert!(backend.reg_status[&r1], "r1 should be freed for {expected_set}");
        }
    }

    #[rstest]
    fn compare_and_set_frees_r1(mut backend: Backend) {
        let r1 = backend.alloc_register();
        let r2 = backend.alloc_register();
        let eq = AstNode::make_binary(Token::EQ, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int);
        backend.compare_and_set(&eq, r1, r2).unwrap();
        assert!(backend.reg_status[&r1]);
    }

    // === Comparisons: compare_and_jump ===

    #[test]
    fn compare_and_jump_all_ops() {
        let cases: [(AstNode, &str); 6] = [
            (AstNode::make_binary(Token::EQ, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "jne"),
            (AstNode::make_binary(Token::NE, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "je"),
            (AstNode::make_binary(Token::LT, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "jge"),
            (AstNode::make_binary(Token::GT, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "jle"),
            (AstNode::make_binary(Token::LE, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "jg"),
            (AstNode::make_binary(Token::GE, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "jl"),
        ];
        for (op, expected_jmp) in &cases {
            let mut backend = backend();
            let r1 = backend.alloc_register();
            let r2 = backend.alloc_register();
            let result = backend.compare_and_jump(op, r1, r2, 1).unwrap();
            assert!(result.is_none());
            let output = output_string(&backend);
            assert!(output.contains("cmpq"), "cmpq missing for {expected_jmp}");
            assert!(output.contains(expected_jmp), "missing {expected_jmp}");
            assert!(output.contains("L1"), "missing label L1 for {expected_jmp}");
        }
    }

    #[rstest]
    fn compare_and_jump_frees_all_regs(mut backend: Backend) {
        let r1 = backend.alloc_register();
        let r2 = backend.alloc_register();
        let eq = AstNode::make_binary(Token::EQ, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int);
        backend.compare_and_jump(&eq, r1, r2, 1).unwrap();
        assert!(all_free(&backend));
    }

    // === Print int ===

    #[rstest]
    fn print_int_moves_to_rdi_and_calls(mut backend: Backend) {
        let reg = backend.alloc_register();
        backend.print_int(reg).unwrap();
        let output = output_string(&backend);
        assert!(output.contains(&format!("\tmovq\t{}, %rdi", reg)));
        assert!(output.contains("\tcall\tprintint"));
        assert!(backend.reg_status[&reg]);
    }

    // === ast_to_jmp_op ===

    #[test]
    fn ast_to_jmp_op_maps_correctly() {
        let zero = || AstNode::make_intlit(0, PrimType::Int);
        let bin = |t| AstNode::make_binary(t, zero(), zero(), PrimType::Int);
        assert_eq!(ast_to_jmp_op(&bin(Token::EQ)), "jne");
        assert_eq!(ast_to_jmp_op(&bin(Token::NE)), "je");
        assert_eq!(ast_to_jmp_op(&bin(Token::LT)), "jge");
        assert_eq!(ast_to_jmp_op(&bin(Token::GT)), "jle");
        assert_eq!(ast_to_jmp_op(&bin(Token::LE)), "jg");
        assert_eq!(ast_to_jmp_op(&bin(Token::GE)), "jl");
    }

    #[test]
    #[should_panic(expected = "Not a comparison operator")]
    fn ast_to_jmp_op_panics_on_non_comparison() {
        ast_to_jmp_op(&AstNode::make_intlit(0, PrimType::Int));
    }

    // === ast_to_set_op ===

    #[test]
    fn ast_to_set_op_maps_correctly() {
        let zero = || AstNode::make_intlit(0, PrimType::Int);
        let bin = |t| AstNode::make_binary(t, zero(), zero(), PrimType::Int);
        assert_eq!(ast_to_set_op(&bin(Token::EQ)), "sete");
        assert_eq!(ast_to_set_op(&bin(Token::NE)), "setne");
        assert_eq!(ast_to_set_op(&bin(Token::LT)), "setl");
        assert_eq!(ast_to_set_op(&bin(Token::GT)), "setg");
        assert_eq!(ast_to_set_op(&bin(Token::LE)), "setle");
        assert_eq!(ast_to_set_op(&bin(Token::GE)), "setge");
    }

    #[test]
    #[should_panic(expected = "Not a comparison operator")]
    fn ast_to_set_op_panics_on_non_comparison() {
        ast_to_set_op(&AstNode::make_intlit(0, PrimType::Int));
    }

    // === X86_64Reg formatting ===

    #[test]
    fn reg_display_format() {
        assert_eq!(format!("{}", X86_64Reg::R8), "%r8");
        assert_eq!(format!("{}", X86_64Reg::R9), "%r9");
        assert_eq!(format!("{}", X86_64Reg::R10), "%r10");
        assert_eq!(format!("{}", X86_64Reg::R11), "%r11");
    }

    #[test]
    fn reg8b_display_format() {
        assert_eq!(format!("{}", X86_64Reg8b::R8), "%r8b");
        assert_eq!(format!("{}", X86_64Reg8b::R9), "%r9b");
        assert_eq!(format!("{}", X86_64Reg8b::R10), "%r10b");
        assert_eq!(format!("{}", X86_64Reg8b::R11), "%r11b");
    }

    #[test]
    fn reg_as_8b() {
        assert!(matches!(X86_64Reg::R8.as_8b(), X86_64Reg8b::R8));
        assert!(matches!(X86_64Reg::R9.as_8b(), X86_64Reg8b::R9));
        assert!(matches!(X86_64Reg::R10.as_8b(), X86_64Reg8b::R10));
        assert!(matches!(X86_64Reg::R11.as_8b(), X86_64Reg8b::R11));
    }
}
