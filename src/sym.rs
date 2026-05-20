use std::cell::{Ref, RefCell};
use crate::scan::Token;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructuralType {
    Function,
    Variable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    Int,
    Char,
    Void,
}

impl From<Token> for DataType {
    fn from(tok: Token) -> Self {
        match tok {
            Token::Char => DataType::Char,
            Token::Int => DataType::Int,
            Token::Void => DataType::Void,
            // TODO: This needs to provide at least a line number
            _ => panic!("Illegal type, token {}", tok),
        }
    }
}

pub struct SymbolEntry {
    name: String,
    pub dtype: DataType,
    pub stype: StructuralType,
}

pub struct SymbolTable {
    globals: RefCell<Vec<SymbolEntry>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        SymbolTable {
            globals: vec![].into(),
        }
    }

    pub fn add_glob(&self, name: &str, dtype: DataType, stype: StructuralType) {
        self.globals.borrow_mut().push(SymbolEntry { name: name.into(), dtype, stype });
    }

    pub fn find_glob(&self, name: &str) -> Option<Ref<'_, SymbolEntry>> {
        let borrow = self.globals.borrow();
        let res = borrow.iter().position(|e| e.name == name);
        res.map(|i| Ref::map(borrow, |vec| &vec[i]))
    }

    pub fn is_empty(&self) -> bool {
        self.globals.borrow().is_empty()
    }

    pub fn len(&self) -> usize {
        self.globals.borrow().len()
    }

    pub fn has_global(&self, name: &str) -> bool {
        self.globals.borrow().iter().position(|e| e.name == name).is_some()
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

#[test]
    fn test_add_glob() {
        let symbols = SymbolTable::new();
        symbols.add_glob("x", DataType::Int, StructuralType::Variable);
        symbols.add_glob("y", DataType::Int, StructuralType::Variable);

        assert_eq!(symbols.len(), 2);
        assert!(symbols.has_global("x"));
        assert!(symbols.has_global("y"));
    }

#[test]
    fn test_find_glob_found() {
        let symbols = SymbolTable::new();
        symbols.add_glob("x", DataType::Int, StructuralType::Variable);
        symbols.add_glob("y", DataType::Int, StructuralType::Variable);

        let result = symbols.find_glob("x");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "x");

        let result = symbols.find_glob("y");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "y");
    }

#[test]
    fn test_find_glob_not_found() {
        let symbols = SymbolTable::new();
        symbols.add_glob("x", DataType::Int, StructuralType::Variable);
        symbols.add_glob("y", DataType::Int, StructuralType::Variable);

        let result = symbols.find_glob("z");
        assert!(result.is_none());
    }
}
