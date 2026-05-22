use anyhow::Result;
use crate::{
    cgen::CodeBackend,
    sym::PrimType,
};

#[derive(Clone, Copy)]
pub enum DummyReg {
    Acc,
}

pub struct DummyBackend {}

impl CodeBackend for DummyBackend {
    type Reg = DummyReg;

    fn free_all_registers(&mut self) -> Result<()> {
        todo!()
    }

    fn preamble(&mut self) -> Result<()> {
        todo!()
    }

    fn type_size(&self, dtype: crate::sym::PrimType) -> usize {
        match dtype {
            PrimType::Long => 8,
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

    fn load_glob(&mut self, _: &str, _: crate::sym::PrimType) -> Result<Self::Reg> {
        Ok(DummyReg::Acc)
    }

    fn store_glob(&mut self, _: Self::Reg, _: &str, _: crate::sym::PrimType) -> Result<Self::Reg> {
        todo!()
    }

    fn glob_sym(&mut self, _: &str, _: crate::sym::PrimType) -> Result<()> {
        todo!()
    }

    fn add(&mut self, _: Self::Reg, _: Self::Reg) -> Result<Option<Self::Reg>> {
        todo!()
    }

    fn sub(&mut self, _: Self::Reg, _: Self::Reg) -> Result<Option<Self::Reg>> {
        todo!()
    }

    fn mul(&mut self, _: Self::Reg, _: Self::Reg) -> Result<Option<Self::Reg>> {
        todo!()
    }

    fn div(&mut self, _: Self::Reg, _: Self::Reg) -> Result<Option<Self::Reg>> {
        todo!()
    }

    fn compare_and_set(&mut self, _: &crate::ast::AstNode, _: Self::Reg, _: Self::Reg) -> Result<Option<Self::Reg>> {
        todo!()
    }

    fn compare_and_jump(&mut self, _: &crate::ast::AstNode, _: Self::Reg, _: Self::Reg, _: usize) -> Result<Option<Self::Reg>> {
        todo!()
    }

    fn label(&mut self, _: usize) -> Result<()> {
        todo!()
    }

    fn jump(&mut self, _: usize) -> Result<()> {
        todo!()
    }

    fn print_int(&mut self, _: Self::Reg) -> Result<()> {
        todo!()
    }

    fn call(&mut self, _: Self::Reg, _: &str) -> Result<Option<Self::Reg>> {
        todo!()
    }

    fn ret(&mut self, _: Self::Reg, _: crate::sym::PrimType, _: usize) -> Result<()> {
        todo!()
    }
}
