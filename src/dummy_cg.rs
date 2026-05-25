use anyhow::Result;
use crate::{
    cgen::CodeBackend,
    sym::{PrimType, SymFilteredIterator},
};

#[derive(Clone, Copy)]
pub enum DummyReg {
    Acc,
}

pub struct DummyBackend {}

impl std::io::Write for DummyBackend {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Ok(0)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl CodeBackend for DummyBackend {
    type Reg = DummyReg;

    fn alignment() -> usize { 0 }

    fn free_all_registers(&mut self) -> Result<()> {
        unimplemented!()
    }

    fn preamble(&mut self) -> Result<()> {
        unimplemented!()
    }

    fn postamble(&mut self, _: SymFilteredIterator) -> Result<()> {
        unimplemented!()
    }

    fn type_size(&self, dtype: PrimType) -> usize {
        match dtype {
            PrimType::Long
            | PrimType::CharPtr
            | PrimType::IntPtr
            | PrimType::LongPtr
            | PrimType::VoidPtr => 8,
            PrimType::Int => 4,
            PrimType::Char => 1,
            PrimType::Void => 0,
        }
    }

    fn func_preamble(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn func_postamble(&mut self, _: usize) -> Result<()> {
        Ok(())
    }

    fn load_int(&mut self, _: i64) -> Result<Self::Reg> {
        Ok(DummyReg::Acc)
    }

    fn load_glob(&mut self, _: &str, _: PrimType) -> Result<Self::Reg> {
        Ok(DummyReg::Acc)
    }

    fn store_glob(&mut self, _: Self::Reg, _: &str, _: PrimType) -> Result<Self::Reg> {
        unimplemented!()
    }

    fn glob_sym(&mut self, _: &str, _: PrimType) -> Result<()> {
        unimplemented!()
    }

    fn add(&mut self, _: Self::Reg, _: Self::Reg) -> Result<Option<Self::Reg>> {
        unimplemented!()
    }

    fn sub(&mut self, _: Self::Reg, _: Self::Reg) -> Result<Option<Self::Reg>> {
        unimplemented!()
    }

    fn mul(&mut self, _: Self::Reg, _: Self::Reg) -> Result<Option<Self::Reg>> {
        unimplemented!()
    }

    fn div(&mut self, _: Self::Reg, _: Self::Reg) -> Result<Option<Self::Reg>> {
        unimplemented!()
    }

    fn compare_and_set(&mut self, _: &crate::ast::AstNode, _: Self::Reg, _: Self::Reg) -> Result<Option<Self::Reg>> {
        unimplemented!()
    }

    fn compare_and_jump(&mut self, _: &crate::ast::AstNode, _: Self::Reg, _: Self::Reg, _: usize) -> Result<Option<Self::Reg>> {
        unimplemented!()
    }

    fn label(&mut self, _: usize) -> Result<()> {
        unimplemented!()
    }

    fn jump(&mut self, _: usize) -> Result<()> {
        unimplemented!()
    }

    fn call(&mut self, _: Self::Reg, _: &str) -> Result<Option<Self::Reg>> {
        unimplemented!()
    }

    fn ret(&mut self, _: Self::Reg, _: PrimType, _: usize) -> Result<()> {
        unimplemented!()
    }

    fn address(&mut self, _: &str) -> Result<Option<Self::Reg>> {
        unimplemented!()
    }

    fn deref(&mut self, _: Self::Reg, _: PrimType) -> Result<Option<Self::Reg>> {
        unimplemented!()
    }

}
