use std::{
    cell::{Ref, RefCell},
    iter::Iterator,
};
use crate::scan::Token;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructuralType {
    Function { end_label: Option<usize> },
    Variable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimType {
    Char,
    Int,
    Long,
    Void,
}

impl From<Token> for PrimType {
    fn from(tok: Token) -> Self {
        match tok {
            Token::Char => PrimType::Char,
            Token::Int => PrimType::Int,
            Token::Long => PrimType::Long,
            Token::Void => PrimType::Void,
            // It's bothersome to list them all, but at this point I want to catch new types

            Token::Plus|Token::Minus|Token::Star|Token::Slash
                |Token::EQ|Token::NE|Token::LT|Token::GT|Token::LE|Token::GE
                |Token::Assign|Token::Ident(_)|Token::IntLit(_)
                |Token::LeftBrace|Token::RightBrace|Token::LeftParen|Token::RightParen 
                |Token::If|Token::Else|Token::For|Token::While
                |Token::Semi|Token::Return
                // TODO: This needs to provide at least a line number
                => panic!("No type for token {:?}", tok)
        }
    }
}

#[derive(Clone)]
pub struct SymbolEntry {
    pub name: String,
    pub dtype: PrimType,
    pub stype: StructuralType,
}

pub struct SymFilteredIterator<'a> {
    borrow: Option<Ref<'a, [SymbolEntry]>>,
    test: fn (&SymbolEntry) -> bool,
}

impl<'a> SymFilteredIterator<'a> {
    pub fn new(cell: &'a RefCell<Vec<SymbolEntry>>, test: fn(&SymbolEntry) -> bool) -> Self {
        let borrow = cell.borrow();
        let slice_borrow = Ref::map(borrow, |b| b.as_slice());
        SymFilteredIterator { borrow: Some(slice_borrow), test }
    }
}

impl<'a> Iterator for SymFilteredIterator<'a> {
    type Item = Ref<'a, SymbolEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let current_borrow = self.borrow.take()?;

            if current_borrow.is_empty() {
                break
            }

            // Split the Ref guard. We keep the tail and consume the head
            let (head, tail) = Ref::map_split(current_borrow, |slice| {
                slice.split_first().unwrap()
            });

            self.borrow = Some(tail);

            if (self.test)(&head) {
                return Some(head)
            }
        }

        None
    }
}

pub struct SymbolTableBuilder {
    globals: Vec<SymbolEntry>,
}

#[allow(clippy::new_without_default)]
impl SymbolTableBuilder {
    pub fn new() -> Self {
        SymbolTableBuilder { globals: vec![] }
    }

    pub fn add_glob(mut self, name: &str, dtype: PrimType, stype: StructuralType) -> Self {
        self.globals.push(SymbolEntry { name: name.into(), dtype, stype });
        self
    }

    pub fn add_glob_fn(mut self, name: &str, dtype: PrimType) -> Self {
        let stype = StructuralType::Function { end_label: None };
        self.globals.push(SymbolEntry { name: name.into(), dtype, stype });
        self
    }

    pub fn build(self) -> SymbolTable {
        SymbolTable { globals: self.globals.into() }
    }
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

    pub fn add_glob(&self, name: &str, dtype: PrimType, stype: StructuralType) {
        self.globals.borrow_mut().push(SymbolEntry { name: name.into(), dtype, stype });
    }

    pub fn add_glob_fn(&self, name: &str, dtype: PrimType) {
        let stype = StructuralType::Function { end_label: None };
        self.globals.borrow_mut().push(SymbolEntry { name: name.into(), dtype, stype });
    }

    fn find_pos(&self, name: &str) -> Option<usize> {
        let borrow = self.globals.borrow();
        borrow.iter().position(|e| e.name == name)
    }

    pub fn find_glob(&self, name: &str) -> Option<Ref<'_, SymbolEntry>> {
        let index = self.find_pos(name);
        let borrow = self.globals.borrow();
        index.map(|i| Ref::map(borrow, |vec| &vec[i]))
    }

    pub fn set_fn_end_label(&self, name: &str, label: usize) {
        if let Some(index) = self.find_pos(name) {
            let mut borrowed = self.globals.borrow_mut();
            let old_entry = &(*borrowed)[index];
            let new_entry = match old_entry {
                SymbolEntry { dtype, stype: StructuralType::Function { end_label: None }, .. } => {
                    SymbolEntry { name: name.into(), dtype: *dtype, stype: StructuralType::Function { end_label: Some(label) } }
                },
                SymbolEntry { stype: StructuralType::Function { end_label: Some(_) }, .. } => {
                    panic!("sym.rs: Trying to set end label for function '{}' which already has one", name)
                },
                _ => panic!("sym.rs: Trying to set end label for a non-function symbol"),
            };
            (*borrowed)[index] = new_entry;
        } else {
            panic!("sym.rs: Trying to set end label for undefined symbol '{}'", name)
        }
    }

    pub fn get_fn_end_label(&self, name: &str) -> usize {
        if let Some(index) = self.find_pos(name) {
            let borrowed = self.globals.borrow();
            match &(*borrowed)[index] {
                SymbolEntry { stype: StructuralType::Function { end_label: Some(label) }, .. } => *label,
                SymbolEntry { stype: StructuralType::Function { end_label: None }, .. } => {
                    panic!("sym.rs: Trying to get end label for function '{}' which has none", name)
                },
                _ => panic!("sym.rs: Trying to get end label for a non-function symbol"),
            }
        } else {
            panic!("sym.rs: Trying to get end label for undefined symbol '{}'", name)
        }
    }

    pub fn get_fn_dtype(&self, name: &str) -> PrimType {
        if let Some(index) = self.find_pos(name) {
            let borrowed = self.globals.borrow();
            match &(*borrowed)[index] {
                &SymbolEntry { dtype, stype: StructuralType::Function { .. }, .. } => dtype,
                _ => panic!("sym.rs: Trying to get type for a non-function symbol"),
            }
        } else {
            panic!("sym.rs: Trying to get type for undefined symbol '{}'", name)
        }
    }

    pub fn iter_global_vars<'a>(&'a self) -> SymFilteredIterator<'a> {
        SymFilteredIterator::new(&self.globals, |se| matches!(se.stype, StructuralType::Variable))
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
        SymbolTableBuilder::new()
            .add_glob_fn("printint", PrimType::Void)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_glob() {
        let symbols = SymbolTable::new();
        symbols.add_glob("x", PrimType::Int, StructuralType::Variable);
        symbols.add_glob("y", PrimType::Int, StructuralType::Variable);

        assert_eq!(symbols.len(), 2);
        assert!(symbols.has_global("x"));
        assert!(symbols.has_global("y"));
    }

    #[test]
    fn test_find_glob_found() {
        let symbols = SymbolTable::new();
        symbols.add_glob("x", PrimType::Int, StructuralType::Variable);
        symbols.add_glob("y", PrimType::Int, StructuralType::Variable);

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
        symbols.add_glob("x", PrimType::Int, StructuralType::Variable);
        symbols.add_glob("y", PrimType::Int, StructuralType::Variable);

        let result = symbols.find_glob("z");
        assert!(result.is_none());
    }

    #[test]
    fn test_global_vars_iterator_only_returns_vars() {
        let symbols = SymbolTable::default();

        // Adds "main" to the default symbols, to ensure there's at least one function
        symbols.add_glob_fn("main", PrimType::Int);
        symbols.add_glob("foo", PrimType::Char, StructuralType::Variable);
        symbols.add_glob("bar", PrimType::Int, StructuralType::Variable);

        let names = symbols.iter_global_vars().map(|se| se.name.clone()).collect::<Vec<_>>();
        assert_eq!(names, vec!["foo", "bar"]);
    }
}
