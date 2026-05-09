use std::{
    collections::HashMap,
    fmt,
    io::Write
};
use anyhow::Result;

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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Reg {
    R8,
    R9,
    R10,
    R11,
}

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reg::R8 => write!(f, "%r8"),
            Reg::R9 => write!(f, "%r9"),
            Reg::R10 => write!(f, "%r10"),
            Reg::R11 => write!(f, "%r11"),
        }
    }
}

pub struct CodeGenerator<T>
    where T: Write
{
    output: T,
    reg_status: HashMap<Reg, bool>,
}

impl<T> CodeGenerator<T>
    where T: Write,
{

    pub fn new(output: T) -> Self {
        CodeGenerator {
            output,
            reg_status: HashMap::from([
                           (Reg::R8, true),
                           (Reg::R9, true),
                           (Reg::R10, true),
                           (Reg::R11, true)]),
        }
    }

    pub fn free_all_registers(&mut self) {
        for val in self.reg_status.values_mut() {
            *val = true;
        }
    }

    fn free_register(&mut self, reg: Reg) {
        self.reg_status.entry(reg).insert_entry(true);
    }

    fn alloc_register(&mut self) -> Reg {
        for (reg, free) in self.reg_status.iter_mut() {
            if *free {
                *free = false;
                return *reg
            }
        }
        panic!("Out of registers!")
    }

    pub fn preamble(&mut self) -> Result<()> {
        self.free_all_registers();
        write!(self.output, "{}", PREAMBLE)?;
        Ok(())
    }

    pub fn postamble(&mut self) -> Result<()> {
        write!(self.output, "{}", POSTAMBLE)?;
        Ok(())
    }

    pub fn load(&mut self, val: i64) -> Result<Reg> {
        let reg = self.alloc_register();
        writeln!(self.output, "\tmovq\t${}, {}", val, reg)?;

        Ok(reg)
    }

    // Add two registers together and return
    // the number of the register with the result
    pub fn add(&mut self, r1: Reg, r2: Reg) -> Result<Reg> {
        writeln!(self.output, "\taddq\t{}, {}", r1, r2)?;
        self.free_register(r1);

        Ok(r2)
    }

    // Subtract the second register from the first and
    // return the number of the register with the result
    pub fn sub(&mut self, r1: Reg, r2: Reg) -> Result<Reg> {
        writeln!(self.output, "\tsubq\t{}, {}", r2, r1)?;
        self.free_register(r2);

        Ok(r1)
    }

    // Multiply two registers together and return
    // the number of the register with the result
    pub fn mul(&mut self, r1: Reg, r2: Reg) -> Result<Reg> {
        writeln!(self.output, "\timulq\t{}, {}", r1, r2)?;
        self.free_register(r1);

        Ok(r2)
    }

    // Divide the first register by the second and
    // regurn the number of the register with the result
    pub fn div(&mut self, r1: Reg, r2: Reg) -> Result<Reg> {
        writeln!(self.output, "\tmovq\t{r1}, %rax")?;
        writeln!(self.output, "\tcqo")?;
        writeln!(self.output, "\tidivq\t{r2}")?;
        writeln!(self.output, "\tmovq\t%rax, {r1}")?;
        self.free_register(r2);

        Ok(r1)
    }

    pub fn print_int(&mut self, r: Reg) -> Result<()> {
        writeln!(self.output, "\tmovq\t{r}, %rdi")?;
        writeln!(self.output, "\tcall\tprintint")?;
        self.free_register(r);

        Ok(())
    }
}
