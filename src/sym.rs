pub struct SymbolEntry {
    name: String,
}

pub struct SymbolTable {
    globals: Vec<SymbolEntry>,
}

impl SymbolTable {
    pub fn new() -> Self {
        SymbolTable {
            globals: vec![],
        }
    }

    pub fn add_glob(&mut self, name: &str) {
        self.globals.push(SymbolEntry { name: name.into() });
    }

    pub fn find_glob(&self, name: &str) -> Option<&SymbolEntry> {
        self.globals.iter().find(|e| e.name == name)
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
        let mut symbols = SymbolTable::new();
        symbols.add_glob("x");
        symbols.add_glob("y");

        assert_eq!(symbols.globals.len(), 2);
        assert_eq!(symbols.globals[0].name, "x");
        assert_eq!(symbols.globals[1].name, "y");
    }

#[test]
    fn test_find_glob_found() {
        let mut symbols = SymbolTable::new();
        symbols.add_glob("x");
        symbols.add_glob("y");

        let result = symbols.find_glob("x");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "x");

        let result = symbols.find_glob("y");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "y");
    }

#[test]
    fn test_find_glob_not_found() {
        let mut symbols = SymbolTable::new();
        symbols.add_glob("x");
        symbols.add_glob("y");

        let result = symbols.find_glob("z");
        assert!(result.is_none());
    }
}
