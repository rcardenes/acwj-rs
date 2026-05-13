use std::{
    collections::HashMap,
    fmt,
    io::Write,
    sync::LazyLock,
};
use anyhow::Result;
use crate::{
    ast::Ast,
    cgen::CodeBackend
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

\t.globl\tmain
\t.type\tmain, @function
main:
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

static CMP_TO_JMP_INSTRUCTIONS: LazyLock<HashMap<Ast, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        (Ast::Equal, "jne"),
        (Ast::NotEqual, "je"),
        (Ast::LessThan, "jge"),
        (Ast::LessThanOrEqual, "jg"),
        (Ast::GreaterThan, "jle"),
        (Ast::GreaterThanOrEqual, "jl"),
    ])
});

static CMP_TO_SET_INSTRUCTIONS: LazyLock<HashMap<Ast, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        (Ast::Equal, "sete"),
        (Ast::NotEqual, "setne"),
        (Ast::LessThan, "setl"),
        (Ast::LessThanOrEqual, "setle"),
        (Ast::GreaterThan, "setg"),
        (Ast::GreaterThanOrEqual, "setge"),
    ])
});

fn ast_to_jmp_op(op: &Ast) -> &'static str {
    match CMP_TO_JMP_INSTRUCTIONS.get(op) {
        Some(instr) => instr,
        None => panic!("Not a comparison operator: {:?}", op)
    }
}

fn ast_to_set_op(op: &Ast) -> &'static str {
    match CMP_TO_SET_INSTRUCTIONS.get(op) {
        Some(instr) => instr,
        None => panic!("Not a comparison operator: {:?}", op)
    }
}

pub struct X864_64Backend<T>
    where T: Write
{
    output: T,
    reg_status: HashMap<X86_64Reg, bool>,
}

impl<T> X864_64Backend<T>
    where T: Write,
{

    pub fn new(output: T) -> Self {
        X864_64Backend {
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

impl<T> CodeBackend for X864_64Backend<T>
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

    fn postamble(&mut self) -> Result<()> {
        write!(self.output, "{}", POSTAMBLE)?;
        Ok(())
    }

    fn load_int(&mut self, val: i64) -> Result<Self::Reg> {
        let reg = self.alloc_register();
        writeln!(self.output, "\tmovq\t${}, {}", val, reg)?;

        Ok(reg)
    }

    fn load_glob(&mut self, ident: &str) -> Result<Self::Reg> {
        // Get a new register
        let reg = self.alloc_register();

        writeln!(self.output, "\tmovq\t{}(%rip), {}", ident, reg)?;

        Ok(reg)
    }

    fn store_glob(&mut self, r: Self::Reg, ident: &str) -> Result<Self::Reg> {
        writeln!(self.output, "\tmovq\t{}, {}(%rip)", r, ident)?;

        Ok(r)
    }

    fn glob_sym(&mut self, sym: &str) -> Result<()> {
        writeln!(self.output, "\t.comm\t{},8,8", sym)?;

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

    fn compare_and_set(&mut self, op: &Ast, r1: Self::Reg, r2: Self::Reg) -> Result<Option<Self::Reg>> {
        let r2_8b = r2.as_8b();

        writeln!(self.output, "\tcmpq\t{}, {}", r2, r1)?;
        writeln!(self.output, "\t{}\t{}", ast_to_set_op(op), r2_8b)?;
        writeln!(self.output, "\tmovzbq\t{}, {}", r2_8b, r2)?;
        self.free_register(r1);

        Ok(Some(r2))
    }

    fn compare_and_jump(&mut self, op: &Ast, r1: Self::Reg, r2: Self::Reg, label_num: usize) -> Result<Option<Self::Reg>> {
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
