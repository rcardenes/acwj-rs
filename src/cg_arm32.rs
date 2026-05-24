use std::{
    collections::HashMap,
    fmt,
    io::Write,
};

use anyhow::Result;

use crate::{
    ast::AstNode,
    cgen::CodeBackend,
    sym::PrimType
};

static FUNC_PREAMBLE: &str = "
.text
\t.globl\t{name}
\t.type\t{name}, %function
{name}:
\tpush\t{fp, lr}
\tadd\tfp, sp, #4
\tsub\tsp, sp, #8
\tstr\tr0, [fp, #-8]
";

static POSTAMBLE: &str = "
\tsub\tsp, fp, #4
\tpop\t{fp, pc}
";

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ArmV7Reg {
    R4,
    R5,
    R6,
    R7,
}

impl fmt::Display for ArmV7Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArmV7Reg::R4 => write!(f, "r4"),
            ArmV7Reg::R5 => write!(f, "r5"),
            ArmV7Reg::R6 => write!(f, "r6"),
            ArmV7Reg::R7 => write!(f, "r7"),
        }
    }
}

// Returns the appropriate branching instruction for the operator
fn ast_to_jmp_op(op: &AstNode) -> &'static str {
    match op {
        AstNode::Equal {..} => "bne",
        AstNode::NotEqual {..} => "beq",
        AstNode::LessThan {..} => "bge",
        AstNode::LessThanOrEqual {..} => "bgt",
        AstNode::GreaterThan {..} => "ble",
        AstNode::GreaterThanOrEqual {..} => "blt",
        _ => panic!("Not a comparison operator: {:?}", op)
    }
}

// Returns two test instructions to be used consecutively
// when performing logic tests over two operands (==, !=, <, <=, >, >=)
fn ast_to_set_op(op: &AstNode) -> (&'static str, &'static str) {
    match op {
        AstNode::Equal {..} => ("moveq", "movne"),
        AstNode::NotEqual {..} => ("movne", "moveq"),
        AstNode::LessThan {..} => ("movlt", "movge"),
        AstNode::LessThanOrEqual {..} => ("movle", "movgt"),
        AstNode::GreaterThan {..} => ("movgt", "movle"),
        AstNode::GreaterThanOrEqual {..} => ("movge", "movlt"),
        _ => panic!("Not a comparison operator: {:?}", op)
    }
}

pub struct ArmV7Backend<T>
    where T: Write,
{
    pub(crate) output: T,
    pub(crate) reg_status: HashMap<ArmV7Reg, bool>,
}

impl<T> ArmV7Backend<T>
    where T: Write,
{
    pub fn new(output: T) -> Self {
        ArmV7Backend {
            output,
            reg_status: HashMap::from([
                           (ArmV7Reg::R4, true),
                           (ArmV7Reg::R5, true),
                           (ArmV7Reg::R6, true),
                           (ArmV7Reg::R7, true)]),
        }
    }

    fn free_register(&mut self, reg: ArmV7Reg) {
        self.reg_status.entry(reg).insert_entry(true);
    }

    fn alloc_register(&mut self) -> ArmV7Reg {
        for (reg, free) in self.reg_status.iter_mut() {
            if *free {
                *free = false;
                return *reg
            }
        }
        panic!("Out of registers!")
    }
}

impl<T> Write for ArmV7Backend<T>
    where T: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.output.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.output.flush()
    }
}

impl<T> CodeBackend for ArmV7Backend<T>
    where T: Write,
{
    type Reg = ArmV7Reg;

    fn alignment() -> usize { 2 }

    fn free_all_registers(&mut self) -> Result<()> {
        for val in self.reg_status.values_mut() {
            *val = true;
        }

        Ok(())
    }

    fn type_size(&self, dtype: crate::sym::PrimType) -> usize {
        match dtype {
            PrimType::Char => 1,
            PrimType::Int => 4,
            PrimType::Long => 4,
            PrimType::Void => 0,
        }
    }

    fn func_preamble(&mut self, ident: &str) -> Result<()> {
        let preamble = FUNC_PREAMBLE.replace("{name}", ident);
        write!(self, "{}", preamble)?;
        Ok(())
    }

    fn func_postamble(&mut self, label_num: usize) -> Result<()> {
        self.label(label_num)?;
        write!(self, "{}", POSTAMBLE)?;
        Ok(())
    }

    fn load_int(&mut self, val: i64) -> anyhow::Result<Self::Reg> {
    // The values are i64, but we're limited to 32 bits...
        let uval = val as u32;
        let reg = self.alloc_register();
        writeln!(self, "\tmovw\t{}, {:#x}", reg, uval & 0xFFFF)?;
        if uval > 0xFFFF {
            writeln!(self, "\tmovt\t{}, {:#x}", reg, uval >> 16)?;
        }

        Ok(reg)
    }

    fn load_glob(&mut self, ident: &str, _: crate::sym::PrimType) -> anyhow::Result<Self::Reg> {
        let reg = self.alloc_register();

        writeln!(self, "\tldr\tr3, ={}", ident)?;
        writeln!(self, "\tldr\t{}, [r3]", reg)?;

        Ok(reg)
    }

    fn store_glob(&mut self, r: Self::Reg, ident: &str, dtype: crate::sym::PrimType) -> anyhow::Result<Self::Reg> {
        writeln!(self, "\tldr\tr3, ={}", ident)?;
        match dtype {
            PrimType::Char =>
                writeln!(self, "\tstrb\t{}, [r3]", r)?,
            PrimType::Int|PrimType::Long =>
                writeln!(self, "\tstr\t{}, [r3]", r)?,
            PrimType::Void => unreachable!("Can't generate store_glob for void types!"),
        }

        Ok(r)
    }

    // Add two registers together and return
    // the number of the register with the result
    fn add(&mut self, r1: Self::Reg, r2: Self::Reg) -> anyhow::Result<Option<Self::Reg>> {
        writeln!(self, "\tadd\t{r2}, {r2}, {r1}")?;
        self.free_register(r1);

        Ok(Some(r2))
    }

    fn sub(&mut self, r1: Self::Reg, r2: Self::Reg) -> anyhow::Result<Option<Self::Reg>> {
        writeln!(self, "\tsub\t{r1}, {r1}, {r2}")?;
        self.free_register(r2);

        Ok(Some(r1))
    }

    // Multiply two registers together and return
    // the number of the register with the result
    fn mul(&mut self, r1: Self::Reg, r2: Self::Reg) -> anyhow::Result<Option<Self::Reg>> {
        writeln!(self, "\tmul\t{r2}, {r2}, {r1}")?;
        self.free_register(r1);

        Ok(Some(r2))
    }

    // Divide the first register by the second and
    // return the number of the register with the result
    fn div(&mut self, r1: Self::Reg, r2: Self::Reg) -> anyhow::Result<Option<Self::Reg>> {
        // We're not doing hardware division but calling a function provided by
        // libgcc. r1 will hold the divisor and r0 the dividend. The quotient is
        // returned in r0
        writeln!(self, "\tmov\tr0, {r1}")?;
        writeln!(self, "\tmov\tr1, {r2}")?;
        writeln!(self, "\tbl\t__aeabi_idiv")?;
        writeln!(self, "\tmov\t{r1}, r0")?;
        self.free_register(r2);

        Ok(Some(r1))
    }

    fn compare_and_set(&mut self, op: &crate::ast::AstNode, r1: Self::Reg, r2: Self::Reg) -> anyhow::Result<Option<Self::Reg>> {
        writeln!(self, "\tcmp\t{}, {}", r2, r1)?;
        let (op1, op2) = ast_to_set_op(op);
        writeln!(self, "\t{}\t{}, #1", op1, r2)?; // If comparison is true, set 1
        writeln!(self, "\t{}\t{}, #0", op2, r2)?; // Otherwise, set 0
        self.free_register(r1);

        Ok(Some(r2))
    }

    fn compare_and_jump(&mut self, op: &crate::ast::AstNode, r1: Self::Reg, r2: Self::Reg, label_num: usize) -> anyhow::Result<Option<Self::Reg>> {
        writeln!(self, "\tcmp\t{}, {}", r2, r1)?;
        writeln!(self, "\t{}\t.L{}", ast_to_jmp_op(op), label_num)?;
        self.free_all_registers()?;

        Ok(None)
    }

    fn jump(&mut self, label_num: usize) -> anyhow::Result<()> {
        writeln!(self, "\tb\t.L{}", label_num)?;

        Ok(())
    }

    fn call(&mut self, r: Self::Reg, ident: &str) -> anyhow::Result<Option<Self::Reg>> {
        writeln!(self, "\tmov\tr0, {}", r)?;
        writeln!(self, "\tbl\t{}", ident)?;
        writeln!(self, "\tmov\t{}, r0", r)?;

        Ok(Some(r))
    }

    fn ret(&mut self, r: Self::Reg, _: crate::sym::PrimType, label_num: usize) -> anyhow::Result<()> {
        writeln!(self, "\tmov\tr0, {}", r)?;

        self.jump(label_num)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};
    use crate::ast::AstNode;
    use crate::sym::PrimType;
    use crate::scan::Token;

    type Backend = ArmV7Backend<Vec<u8>>;

    #[fixture]
    fn backend() -> Backend {
        ArmV7Backend::new(Vec::new())
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
        let regs: std::collections::HashSet<ArmV7Reg> = (0..4).map(|_| backend.alloc_register()).collect();
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
        assert!(output.contains("push\t{fp, lr}"));
    }

    #[rstest]
    fn func_preamble_writes_underscore_name(mut backend: Backend) {
        backend.func_preamble("my_func").unwrap();
        let output = output_string(&backend);
        assert!(output.contains("my_func:"));
    }

    #[rstest]
    fn func_postamble_writes_return(mut backend: Backend) {
        backend.func_postamble(0).unwrap();
        let output = output_string(&backend);
        assert!(output.contains("pop\t{fp, pc}"));
        assert!(output.contains(".L0:"));
    }

    // === Load / Store ===

    #[rstest]
    fn load_int_small_writes_movw_only(mut backend: Backend) {
        let reg = backend.load_int(42).unwrap();
        assert!(!backend.reg_status[&reg]);
        let output = output_string(&backend);
        assert!(output.contains(&format!("\tmovw\t{}, {:#x}", reg, 42)));
        assert!(!output.contains("movt"));
    }

    #[rstest]
    fn load_int_uses_movw_movt_for_large_values(mut backend: Backend) {
        let val = 0x1_0000;
        let reg = backend.load_int(val).unwrap();
        let output = output_string(&backend);
        assert!(output.contains(&format!("\tmovw\t{}, {:#x}", reg, val & 0xFFFF)));
        assert!(output.contains(&format!("\tmovt\t{}, {:#x}", reg, val >> 16)));
    }

    #[rstest]
    fn load_int_small_avoids_movt(mut backend: Backend) {
        backend.load_int(0xFFFF).unwrap();
        let output = output_string(&backend);
        assert!(!output.contains("movt"));
    }

    #[rstest]
    fn load_int_large_triggers_movt(mut backend: Backend) {
        backend.load_int(0x1_0000).unwrap();
        let output = output_string(&backend);
        assert!(output.contains("movt"));
    }

    #[rstest]
    fn load_glob_loads_via_r3(mut backend: Backend) {
        backend.load_glob("x", PrimType::Int).unwrap();
        let output = output_string(&backend);
        assert!(output.contains("ldr\tr3, =x"));
        assert!(output.contains("ldr\t"));
    }

    #[rstest]
    fn store_glob_long(mut backend: Backend) {
        let reg = backend.alloc_register();
        let ret = backend.store_glob(reg, "l", PrimType::Long).unwrap();
        assert_eq!(ret, reg);
        let output = output_string(&backend);
        assert!(output.contains(&format!("\tstr\t{}, [r3]", reg)));
    }

    #[rstest]
    fn store_glob_int(mut backend: Backend) {
        let reg = backend.alloc_register();
        let ret = backend.store_glob(reg, "x", PrimType::Int).unwrap();
        assert_eq!(ret, reg);
        let output = output_string(&backend);
        assert!(output.contains(&format!("\tstr\t{}, [r3]", reg)));
    }

    #[rstest]
    fn store_glob_char(mut backend: Backend) {
        let reg = backend.alloc_register();
        let ret = backend.store_glob(reg, "c", PrimType::Char).unwrap();
        assert_eq!(ret, reg);
        let output = output_string(&backend);
        assert!(output.contains(&format!("\tstrb\t{}, [r3]", reg)));
    }

    #[rstest]
    #[should_panic(expected = "Can't generate store_glob")]
    fn store_glob_void_panics(mut backend: Backend) {
        let reg = backend.alloc_register();
        let _ = backend.store_glob(reg, "v", PrimType::Void);
    }

    // === Glob symbols ===

    #[rstest]
    fn glob_sym_long(mut backend: Backend) {
        backend.glob_sym("l", PrimType::Long).unwrap();
        assert_eq!(output_string(&backend), ".global l\nl:\n\t.zero 4\n");
    }

    #[rstest]
    fn glob_sym_int(mut backend: Backend) {
        backend.glob_sym("x", PrimType::Int).unwrap();
        assert_eq!(output_string(&backend), ".global x\nx:\n\t.zero 4\n");
    }

    #[rstest]
    fn glob_sym_char(mut backend: Backend) {
        backend.glob_sym("c", PrimType::Char).unwrap();
        assert_eq!(output_string(&backend), ".global c\nc:\n\t.zero 1\n");
    }

    #[rstest]
    #[should_panic]
    fn glob_sym_void_panics(mut backend: Backend) {
        backend.glob_sym("v", PrimType::Void).unwrap();
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
        assert!(output_string(&backend).contains(&format!("\tadd\t{}, {}, {}", r2, r2, r1)));
    }

    #[rstest]
    fn sub_frees_r2_returns_r1(mut backend: Backend) {
        let r1 = backend.alloc_register();
        let r2 = backend.alloc_register();
        let result = backend.sub(r1, r2).unwrap();
        assert_eq!(result, Some(r1));
        assert!(!backend.reg_status[&r1]);
        assert!(backend.reg_status[&r2]);
        assert!(output_string(&backend).contains(&format!("\tsub\t{}, {}, {}", r1, r1, r2)));
    }

    #[rstest]
    fn mul_frees_r1_returns_r2(mut backend: Backend) {
        let r1 = backend.alloc_register();
        let r2 = backend.alloc_register();
        let result = backend.mul(r1, r2).unwrap();
        assert_eq!(result, Some(r2));
        assert!(backend.reg_status[&r1]);
        assert!(!backend.reg_status[&r2]);
        assert!(output_string(&backend).contains(&format!("\tmul\t{}, {}, {}", r2, r2, r1)));
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
        assert!(output.contains("__aeabi_idiv"));
        assert!(output.contains(&format!("\tmov\tr0, {}", r1)));
        assert!(output.contains(&format!("\tmov\tr1, {}", r2)));
    }

    // === Labels and jumps ===

    #[rstest]
    fn label_writes_label(mut backend: Backend) {
        backend.label(42).unwrap();
        assert_eq!(output_string(&backend), ".L42:\n");
    }

    #[rstest]
    fn jump_writes_b(mut backend: Backend) {
        backend.jump(7).unwrap();
        assert_eq!(output_string(&backend), "\tb\t.L7\n");
    }

    // === Comparisons: compare_and_set ===

    #[test]
    fn compare_and_set_all_ops() {
        let cases: [(AstNode, (&str, &str)); 6] = [
            (AstNode::make_binary(Token::EQ, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), ("moveq", "movne")),
            (AstNode::make_binary(Token::NE, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), ("movne", "moveq")),
            (AstNode::make_binary(Token::LT, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), ("movlt", "movge")),
            (AstNode::make_binary(Token::GT, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), ("movgt", "movle")),
            (AstNode::make_binary(Token::LE, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), ("movle", "movgt")),
            (AstNode::make_binary(Token::GE, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), ("movge", "movlt")),
        ];
        for (op, (op1, op2)) in &cases {
            let mut backend = backend();
            let r1 = backend.alloc_register();
            let r2 = backend.alloc_register();
            let result = backend.compare_and_set(op, r1, r2).unwrap();
            assert_eq!(result, Some(r2));
            let output = output_string(&backend);
            assert!(output.contains("cmp"), "cmp missing for {op1}/{op2}");
            assert!(output.contains(op1), "missing {op1}");
            assert!(output.contains(op2), "missing {op2}");
            assert!(backend.reg_status[&r1], "r1 should be freed for {op1}/{op2}");
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
            (AstNode::make_binary(Token::EQ, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "bne"),
            (AstNode::make_binary(Token::NE, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "beq"),
            (AstNode::make_binary(Token::LT, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "bge"),
            (AstNode::make_binary(Token::GT, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "ble"),
            (AstNode::make_binary(Token::LE, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "bgt"),
            (AstNode::make_binary(Token::GE, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int), "blt"),
        ];
        for (op, expected_jmp) in &cases {
            let mut backend = backend();
            let r1 = backend.alloc_register();
            let r2 = backend.alloc_register();
            let result = backend.compare_and_jump(op, r1, r2, 1).unwrap();
            assert!(result.is_none());
            let output = output_string(&backend);
            assert!(output.contains("cmp"), "cmp missing for {expected_jmp}");
            assert!(output.contains(expected_jmp), "missing {expected_jmp}");
            assert!(output.contains(".L1"), "missing label L1 for {expected_jmp}");
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

    // === ast_to_jmp_op ===

    #[test]
    fn ast_to_jmp_op_maps_correctly() {
        let zero = || AstNode::make_intlit(0, PrimType::Int);
        let bin = |t| AstNode::make_binary(t, zero(), zero(), PrimType::Int);
        assert_eq!(ast_to_jmp_op(&bin(Token::EQ)), "bne");
        assert_eq!(ast_to_jmp_op(&bin(Token::NE)), "beq");
        assert_eq!(ast_to_jmp_op(&bin(Token::LT)), "bge");
        assert_eq!(ast_to_jmp_op(&bin(Token::GT)), "ble");
        assert_eq!(ast_to_jmp_op(&bin(Token::LE)), "bgt");
        assert_eq!(ast_to_jmp_op(&bin(Token::GE)), "blt");
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
        assert_eq!(ast_to_set_op(&bin(Token::EQ)), ("moveq", "movne"));
        assert_eq!(ast_to_set_op(&bin(Token::NE)), ("movne", "moveq"));
        assert_eq!(ast_to_set_op(&bin(Token::LT)), ("movlt", "movge"));
        assert_eq!(ast_to_set_op(&bin(Token::GT)), ("movgt", "movle"));
        assert_eq!(ast_to_set_op(&bin(Token::LE)), ("movle", "movgt"));
        assert_eq!(ast_to_set_op(&bin(Token::GE)), ("movge", "movlt"));
    }

    #[test]
    #[should_panic(expected = "Not a comparison operator")]
    fn ast_to_set_op_panics_on_non_comparison() {
        ast_to_set_op(&AstNode::make_intlit(0, PrimType::Int));
    }

    // === ArmV7Reg formatting ===

    #[test]
    fn reg_display_format() {
        assert_eq!(format!("{}", ArmV7Reg::R4), "r4");
        assert_eq!(format!("{}", ArmV7Reg::R5), "r5");
        assert_eq!(format!("{}", ArmV7Reg::R6), "r6");
        assert_eq!(format!("{}", ArmV7Reg::R7), "r7");
    }

    // === type_size ===

    #[rstest]
    fn type_size_char(backend: Backend) {
        assert_eq!(backend.type_size(PrimType::Char), 1);
    }

    #[rstest]
    fn type_size_int(backend: Backend) {
        assert_eq!(backend.type_size(PrimType::Int), 4);
    }

    #[rstest]
    fn type_size_long(backend: Backend) {
        assert_eq!(backend.type_size(PrimType::Long), 4);
    }

    #[rstest]
    fn type_size_void(backend: Backend) {
        assert_eq!(backend.type_size(PrimType::Void), 0);
    }

    // === Alignment ===

    #[test]
    fn alignment_returns_2() {
        assert_eq!(<ArmV7Backend<Vec<u8>> as CodeBackend>::alignment(), 2);
    }

    // === Call and Ret ===

    #[rstest]
    fn call_moves_r0_and_bl(mut backend: Backend) {
        let reg = backend.alloc_register();
        let result = backend.call(reg, "foo").unwrap();
        assert_eq!(result, Some(reg));
        let output = output_string(&backend);
        assert!(output.contains(&format!("\tmov\tr0, {}", reg)));
        assert!(output.contains("\tbl\tfoo"));
        assert!(output.contains(&format!("\tmov\t{}, r0", reg)));
    }

    #[rstest]
    fn ret_moves_r0_and_jumps(mut backend: Backend) {
        let reg = backend.alloc_register();
        backend.ret(reg, PrimType::Int, 3).unwrap();
        let output = output_string(&backend);
        assert!(output.contains(&format!("\tmov\tr0, {}", reg)));
        assert!(output.contains("\tb\t.L3"));
    }

    #[rstest]
    fn ret_ignores_dtype(mut backend: Backend) {
        let reg = backend.alloc_register();
        backend.ret(reg, PrimType::Char, 5).unwrap();
        let output = output_string(&backend);
        // ARM ret does not emit any type-specific instruction
        assert!(output.contains(&format!("\tmov\tr0, {}", reg)));
        assert!(output.contains("\tb\t.L5"));
    }
}
