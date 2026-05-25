use std::{
    cell::{Ref, RefCell},
    iter::Iterator,
};
use crate::scan::TokenType;

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
    CharPtr,
    IntPtr,
    LongPtr,
    VoidPtr,
}

impl PrimType {
    pub fn pointer_to(&self) -> PrimType {
        match self {
            PrimType::Char => PrimType::CharPtr,
            PrimType::Int => PrimType::IntPtr,
            PrimType::Long => PrimType::LongPtr,
            PrimType::Void => PrimType::VoidPtr,
            _ => panic!("Unrecognized in pointer_to: {self:?}")
        }
    }

    pub fn value_at(&self) -> PrimType {
        match self {
            PrimType::CharPtr => PrimType::Char,
            PrimType::IntPtr => PrimType::Int,
            PrimType::LongPtr => PrimType::Long,
            PrimType::VoidPtr => PrimType::Void,
            _ => panic!("Unrecognized in value_at: {self:?}")
        }
    }
}

impl From<TokenType> for PrimType {
    fn from(tok: TokenType) -> Self {
        match tok {
            TokenType::Char => PrimType::Char,
            TokenType::Int => PrimType::Int,
            TokenType::Long => PrimType::Long,
            TokenType::Void => PrimType::Void,
            // It's bothersome to list them all, but at this point I want to catch new types

            TokenType::Plus|TokenType::Minus|TokenType::Star|TokenType::Slash
                |TokenType::EQ|TokenType::NE|TokenType::LT|TokenType::GT|TokenType::LE|TokenType::GE
                |TokenType::Assign|TokenType::Ident(_)|TokenType::IntLit(_)
                |TokenType::LeftBrace|TokenType::RightBrace|TokenType::LeftParen|TokenType::RightParen 
                |TokenType::If|TokenType::Else|TokenType::For|TokenType::While
                |TokenType::Semi|TokenType::Return|TokenType::Amper|TokenType::LogAnd
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
        self.globals.borrow().iter().any(|e| e.name == name)
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

    // --- PrimType::pointer_to ---

    #[test]
    fn pointer_to_char_returns_charptr() {
        assert_eq!(PrimType::Char.pointer_to(), PrimType::CharPtr);
    }

    #[test]
    fn pointer_to_int_returns_intptr() {
        assert_eq!(PrimType::Int.pointer_to(), PrimType::IntPtr);
    }

    #[test]
    fn pointer_to_long_returns_longptr() {
        assert_eq!(PrimType::Long.pointer_to(), PrimType::LongPtr);
    }

    #[test]
    fn pointer_to_void_returns_voidptr() {
        assert_eq!(PrimType::Void.pointer_to(), PrimType::VoidPtr);
    }

    #[test]
    #[should_panic(expected = "Unrecognized in pointer_to")]
    fn pointer_to_pointer_panics() {
        PrimType::CharPtr.pointer_to();
    }

    // --- PrimType::value_at ---

    #[test]
    fn value_at_charptr_returns_char() {
        assert_eq!(PrimType::CharPtr.value_at(), PrimType::Char);
    }

    #[test]
    fn value_at_intptr_returns_int() {
        assert_eq!(PrimType::IntPtr.value_at(), PrimType::Int);
    }

    #[test]
    fn value_at_longptr_returns_long() {
        assert_eq!(PrimType::LongPtr.value_at(), PrimType::Long);
    }

    #[test]
    fn value_at_voidptr_returns_void() {
        assert_eq!(PrimType::VoidPtr.value_at(), PrimType::Void);
    }

    #[test]
    #[should_panic(expected = "Unrecognized in value_at")]
    fn value_at_non_pointer_panics() {
        PrimType::Char.value_at();
    }

    // --- PrimType::from(TokenType) ---

    #[test]
    fn prim_from_char_token() {
        assert_eq!(PrimType::from(TokenType::Char), PrimType::Char);
    }

    #[test]
    fn prim_from_int_token() {
        assert_eq!(PrimType::from(TokenType::Int), PrimType::Int);
    }

    #[test]
    fn prim_from_long_token() {
        assert_eq!(PrimType::from(TokenType::Long), PrimType::Long);
    }

    #[test]
    fn prim_from_void_token() {
        assert_eq!(PrimType::from(TokenType::Void), PrimType::Void);
    }

    #[test]
    #[should_panic(expected = "No type for token")]
    fn prim_from_non_type_token_panics() {
        let _: PrimType = TokenType::Semi.into();
    }

    // --- SymbolTable function label/type operations ---

    #[test]
    fn set_and_get_fn_end_label() {
        let symbols = SymbolTable::new();
        symbols.add_glob_fn("foo", PrimType::Int);
        symbols.set_fn_end_label("foo", 42);
        assert_eq!(symbols.get_fn_end_label("foo"), 42);
    }

    #[test]
    #[should_panic(expected = "undefined symbol")]
    fn set_fn_end_label_undefined_panics() {
        let symbols = SymbolTable::new();
        symbols.set_fn_end_label("foo", 1);
    }

    #[test]
    #[should_panic(expected = "already has one")]
    fn set_fn_end_label_already_set_panics() {
        let symbols = SymbolTable::new();
        symbols.add_glob_fn("foo", PrimType::Int);
        symbols.set_fn_end_label("foo", 1);
        symbols.set_fn_end_label("foo", 2);
    }

    #[test]
    #[should_panic(expected = "non-function symbol")]
    fn set_fn_end_label_non_function_panics() {
        let symbols = SymbolTable::new();
        symbols.add_glob("x", PrimType::Int, StructuralType::Variable);
        symbols.set_fn_end_label("x", 1);
    }

    #[test]
    #[should_panic(expected = "which has none")]
    fn get_fn_end_label_none_panics() {
        let symbols = SymbolTable::new();
        symbols.add_glob_fn("foo", PrimType::Int);
        let _ = symbols.get_fn_end_label("foo");
    }

    #[test]
    #[should_panic(expected = "non-function symbol")]
    fn get_fn_end_label_non_function_panics() {
        let symbols = SymbolTable::new();
        symbols.add_glob("x", PrimType::Int, StructuralType::Variable);
        let _ = symbols.get_fn_end_label("x");
    }

    #[test]
    #[should_panic(expected = "undefined symbol")]
    fn get_fn_end_label_undefined_panics() {
        let symbols = SymbolTable::new();
        let _ = symbols.get_fn_end_label("foo");
    }

    #[test]
    fn get_fn_dtype_returns_type() {
        let symbols = SymbolTable::new();
        symbols.add_glob_fn("foo", PrimType::Int);
        assert_eq!(symbols.get_fn_dtype("foo"), PrimType::Int);
    }

    #[test]
    #[should_panic(expected = "non-function symbol")]
    fn get_fn_dtype_non_function_panics() {
        let symbols = SymbolTable::new();
        symbols.add_glob("x", PrimType::Int, StructuralType::Variable);
        let _ = symbols.get_fn_dtype("x");
    }

    #[test]
    #[should_panic(expected = "undefined symbol")]
    fn get_fn_dtype_undefined_panics() {
        let symbols = SymbolTable::new();
        let _ = symbols.get_fn_dtype("foo");
    }

    #[test]
    fn builder_add_glob_fn_creates_function() {
        let st = SymbolTableBuilder::new()
            .add_glob_fn("main", PrimType::Int)
            .build();
        assert_eq!(st.len(), 1);
        let entry = st.find_glob("main").unwrap();
        assert!(matches!(entry.stype, StructuralType::Function { .. }));
        assert_eq!(entry.dtype, PrimType::Int);
    }

    #[test]
    fn is_empty_on_new_table() {
        let symbols = SymbolTable::new();
        assert!(symbols.is_empty());
    }

    #[test]
    fn add_glob_fn_adds_function() {
        let symbols = SymbolTable::new();
        symbols.add_glob_fn("foo", PrimType::Void);
        assert!(symbols.has_global("foo"));
        assert_eq!(symbols.len(), 1);
    }
}
