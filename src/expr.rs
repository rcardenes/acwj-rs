use std::cell::RefCell;
use anyhow::{Result, bail};
use crate::{
    ast::AstNode,
    cgen::{CodeBackend, CodeGenerator},
    misc::{fatal_other, fatal_pos, fatal_tok},
    scan::{Scanner, Token, TokenType},
    sym::{PrimType, SymbolEntry, SymbolTable},
};

pub enum Compatibility {
    Incompatible,
    Compatible(PrimType), // Just carry a copy of the compatible type
    WidenLeft(PrimType), // Type to widen to
    WidenRight(PrimType), // Type to widen to
}

type Precedence = i16;

/// Return numeric precence for the different tokens, so that we
/// can use it in a Pratt-style parser.
fn op_precedence(line: usize, token: &TokenType) -> Result<Precedence> {
    Ok(match token {
        TokenType::Plus|TokenType::Minus => 10,
        TokenType::Star|TokenType::Slash => 20,
        TokenType::EQ|TokenType::NE => 30,
        TokenType::LT|TokenType::LE|TokenType::GT|TokenType::GE => 40,
        _  => {
            bail!("syntax error on line {}, token {:?}", line, token)
        }
    })
}

fn is_arithop(token: &TokenType) -> bool {
    matches!(token, TokenType::Plus
            |TokenType::Minus
            |TokenType::Star
            |TokenType::Slash
            |TokenType::EQ
            |TokenType::NE
            |TokenType::LT
            |TokenType::LE
            |TokenType::GT
            |TokenType::GE)
}

pub struct ExpressionGenerator<'a, T, B>
    where B: CodeBackend,
{
    scanner: &'a Scanner<T>,
    sym_table: &'a SymbolTable,
    code_gen: &'a CodeGenerator<'a, B>,
    in_function: RefCell<Vec<SymbolEntry>>,
}

impl<'a, T, B> ExpressionGenerator<'a, T, B>
    where T: std::io::Read,
          B: CodeBackend,
{
    pub fn new(scanner: &'a Scanner<T>, sym_table: &'a SymbolTable, code_gen: &'a CodeGenerator<B>) -> Self {
        ExpressionGenerator { scanner, sym_table, code_gen, in_function: vec![].into() }
    }

    // Return if two primitive types are compatible.
    // If only_right is true, then widening can happen only left to right
    pub fn type_compatibility(&self, left: PrimType, right: PrimType, only_right: bool) -> Compatibility {
        let (size_left, size_right) = (
            self.code_gen.type_size(left),
            self.code_gen.type_size(right)
            );
        if left == right {
            Compatibility::Compatible(left)
        } else if size_left == 0 || size_right == 0 {
            // One of the sides is likely void
            Compatibility::Incompatible
        } else if size_left < size_right {
            Compatibility::WidenLeft(right)
        } else if size_right < size_left {
            if only_right {
                Compatibility::Incompatible
            } else {
                Compatibility::WidenRight(left)
            }
        } else {
            // Same sizes
            Compatibility::Compatible(left)
        }
    }

    pub fn enter_function(&self, id: &str) {
        if let Some(sym) = self.sym_table.find_glob(id) {
            self.in_function.borrow_mut().push((*sym).clone());
        } else {
            // This can't happen under normal circumstances. enter_function is called
            // AFTER finding the identifier
            unreachable!("enter_function failed finding its symbol")
        }
    }

    pub fn current_function_type(&self) -> PrimType {
        if let Some(entry) = self.in_function.borrow().last() {
            entry.dtype
        } else {
            self.scanner.fatal("Can't return from outside a function")
        }
    }

    pub fn current_function_name(&self) -> String {
        if let Some(entry) = self.in_function.borrow().last() {
            entry.name.clone()
        } else {
            self.scanner.fatal("Can't return from outside a function")
        }
    }

    pub fn exit_function(&self) {
        if !self.in_function.borrow().is_empty() {
            let _ = self.in_function.borrow_mut().pop();
        } else {
            unreachable!("exit_function called but we're not inside a function")
        }
    }

    pub fn function_call(&self, token: Token) -> Result<AstNode> {
        // Check that the symbol has been declared.
        // TODO: Add structural type test
        let Token { ttype: TokenType::Ident(id), .. } = &token else { unreachable!("We only get here passing an ident") };

        if let Some(sym) = self.sym_table.find_glob(id) {
            let expr = self.binexpr(0)?;
            self.scanner.rparen();

            Ok(AstNode::make_function_call(id, expr, sym.dtype))
        } else {
            let id = id.clone();
            fatal_other("Undeclared function", token, id)
        }
    }

    pub fn primary(&self) -> Result<AstNode>
    {
        // For an INTLIT token, make a leaf AST node for it,
        // Otherwise, a syntax error for any other token type

        if let Some(token) = self.scanner.scan() {
            match &token.ttype {
                // For an IntLit token, make it a Char if it is within that type's range,
                // so that we don't have to narrow it later if needed. Widen the data is
                // always possible.
                TokenType::IntLit(val) => if *val >= 0 && *val < 256 {
                    Ok(AstNode::make_intlit(*val, PrimType::Char))
                } else {
                    Ok(AstNode::make_intlit(*val, PrimType::Int))
                },
                TokenType::Ident(id) => {
                    if self.scanner.maybe_token(TokenType::LeftParen) {
                        self.function_call(token)
                    } else if let Some(sym) = self.sym_table.find_glob(id) {
                        Ok(AstNode::make_ident(id, sym.dtype))
                    } else {
                        let id = id.clone();
                        fatal_other("Unknown variable", token, id)
                    }
                },
                _ => fatal_tok("Syntax error, token", token)
            }
        } else {
            panic!("EOF reached, expected an integer")
        }
    }

    fn prefix(&self) -> Result<AstNode> {
        if let Some(token) = self.scanner.scan() {
            match &token.ttype {
                TokenType::Amper => {
                    Ok(self.prefix()?.into_pointer("& operator must be followed by an identifier"))
                },
                TokenType::Star => {
                    Ok(self.prefix()?.make_deref("* operator must be followed by an identifier or *"))
                },
                _ => {
                    self.scanner.putback_token(token);
                    self.primary()
                }
            }
        } else {
            panic!("EOF reached, expected an expression")
        }
    }

    // Return an AST tree whose root is a binary operator.
    // ptp is the precedence of the previous token
    pub fn binexpr(&self, ptp: Precedence) -> Result<AstNode>
        where T: std::io::Read,
    {
        let mut left = self.prefix()?;

        while let Some(token) = self.scanner.scan() {
            if !is_arithop(&token.ttype) {
                self.scanner.putback_token(token);
                break;
            }

            let curr_prec = op_precedence(self.scanner.get_line(), &token.ttype)?;
            if curr_prec <= ptp {
                self.scanner.putback_token(token);
                break;
            }

            let mut right = self.binexpr(curr_prec)?;
            let compat = if let (Some(left_type), Some(right_type)) = (left.get_type(), right.get_type()) {
                self.type_compatibility(left_type, right_type, false)
            } else {
                Compatibility::Incompatible
            };

            let bin_type = match compat {
                Compatibility::Incompatible => fatal_pos("Incompatible types", token),
                Compatibility::WidenLeft(t) => {
                    // Widen the left branch
                    left = left.new_type(t);
                    t
                },
                Compatibility::WidenRight(t) => {
                    // Widen the right branch
                    right = right.new_type(t);
                    t
                },
                Compatibility::Compatible(t) => t, // Do nothing for full compatibility
            };

            left = match token.ttype {
                TokenType::Plus|TokenType::Minus|TokenType::Star|TokenType::Slash|TokenType::EQ|TokenType::NE|TokenType::LT|TokenType::LE|TokenType::GT|TokenType::GE => {
                    AstNode::make_binary(token.ttype, left, right, bin_type)
                }
                _ => unreachable!("This shouldn't be reachable after we tested the op to be arithmetic")
            };
        }

        Ok(left)
    }
}

#[cfg(test)]
mod tests {
    use rstest::{fixture, rstest};
    use std::io::Cursor;
    use super::*;
    use crate::dummy_cg::DummyBackend;
    use crate::sym::StructuralType;

    fn scanner_from(s: &str) -> Scanner<Cursor<Vec<u8>>> {
        Scanner::new(Cursor::new(s.as_bytes().to_vec()))
    }

    type TestInput = Cursor<Vec<u8>>;
    type CodeGen<'a> = CodeGenerator<'a, DummyBackend>;

    fn code_gen<'a>(symbols: &'a SymbolTable) -> CodeGen<'a> {
        CodeGenerator::new(DummyBackend{}, &symbols)
    }

    struct TestFramework
    {
        scanner: Scanner<TestInput>,
        sym_table: SymbolTable,
    }

    impl TestFramework {
        fn new(s: &str) -> Self {
            let scanner = Scanner::new(Cursor::new(s.as_bytes().to_vec()));
            let sym_table = SymbolTable::new();

            TestFramework { 
                scanner,
                sym_table,
            }
        }

        fn primary(&self) -> Result<AstNode> {
            let code_gen = code_gen(&self.sym_table);
            ExpressionGenerator::new(&self.scanner, &self.sym_table, &code_gen).primary()
        }

        fn binexpr(&self, ptp: Precedence) -> Result<AstNode> {
            let code_gen = code_gen(&self.sym_table);
            ExpressionGenerator::new(&self.scanner, &self.sym_table, &code_gen).binexpr(ptp)
        }
    }

    #[fixture]
    fn testfr(#[default("")] text: &str) -> TestFramework {
        TestFramework::new(text)
    }


    // --- primary ---

    #[rstest]
    fn primary_intlit_returns_leaf_node(#[with("42")] testfr: TestFramework) {
        let node = testfr.primary().unwrap();
        assert_eq!(node, AstNode::make_intlit(42, PrimType::Char));
    }

    #[rstest]
    fn primary_intlit_in_char_range(#[with("255")] testfr: TestFramework) {
        let node = testfr.primary().unwrap();
        assert_eq!(node, AstNode::make_intlit(255, PrimType::Char));
    }

    #[rstest]
    #[should_panic]
    fn primary_panics_on_operator_token(#[with("+")] testfr: TestFramework) {
        testfr.primary().unwrap();
    }

    #[rstest]
    #[should_panic]
    fn primary_panics_at_eof(#[with("")] testfr: TestFramework) {
        testfr.primary().unwrap();
    }

    // --- binexpr ---

    #[rstest]
    fn binexpr_single_integer_returns_intlit_root(#[with("7")] testfr: TestFramework) {
        let tree = testfr.binexpr(0).expect("Expected a clean parsing");
        assert_eq!(tree, AstNode::make_intlit(7, PrimType::Char));
    }

    #[rstest]
    fn binexpr_addition_builds_correct_tree(#[with("3 + 5")] testfr: TestFramework) {
        let tree = testfr.binexpr(0).expect("Expected a clean parsing");
        assert_eq!(tree, AstNode::make_binary(TokenType::Plus,
                                              AstNode::make_intlit(3, PrimType::Char),
                                              AstNode::make_intlit(5, PrimType::Char),
                                              PrimType::Char));
    }

    #[rstest]
    // "2 - 3 + 5" parses as Add(Subtract(2, 3), 5): last op is root, left subtree holds earlier ops
    fn binexpr_equal_precedence_is_left_associative(#[with("2 - 3 + 5")] testfr: TestFramework) {
        let tree = testfr.binexpr(0).expect("Expected a clean parsing");
        assert_eq!(tree,
            AstNode::make_binary(TokenType::Plus,
                AstNode::make_binary(TokenType::Minus,
                                     AstNode::make_intlit(2, PrimType::Char),
                                     AstNode::make_intlit(3, PrimType::Char),
                                     PrimType::Char),
                AstNode::make_intlit(5, PrimType::Char),
                PrimType::Char));
    }

    // --- op_precedence ---

    #[test]
    fn op_precedence_returns_correct_values() {
        let s = scanner_from("");
        let line = s.get_line();
        assert_eq!(op_precedence(line, &TokenType::Plus).unwrap(), 10);
        assert_eq!(op_precedence(line, &TokenType::Minus).unwrap(), 10);
        assert_eq!(op_precedence(line, &TokenType::Star).unwrap(), 20);
        assert_eq!(op_precedence(line, &TokenType::Slash).unwrap(), 20);
        assert_eq!(op_precedence(line, &TokenType::EQ).unwrap(), 30);
        assert_eq!(op_precedence(line, &TokenType::NE).unwrap(), 30);
        assert_eq!(op_precedence(line, &TokenType::LT).unwrap(), 40);
        assert_eq!(op_precedence(line, &TokenType::LE).unwrap(), 40);
        assert_eq!(op_precedence(line, &TokenType::GT).unwrap(), 40);
        assert_eq!(op_precedence(line, &TokenType::GE).unwrap(), 40);
    }

    #[test]
    fn op_precedence_fails_on_non_operator() {
        assert!(op_precedence(1, &TokenType::Semi).is_err());
        assert!(op_precedence(1, &TokenType::IntLit(1)).is_err());
    }

    // --- is_arithop ---

    #[test]
    fn is_arithop_true_for_all_operator_tokens() {
        for tok in &[TokenType::Plus, TokenType::Minus, TokenType::Star, TokenType::Slash,
                     TokenType::EQ, TokenType::NE, TokenType::LT, TokenType::LE, TokenType::GT, TokenType::GE] {
            assert!(is_arithop(tok), "{tok:?} should be an arith op");
        }
    }

    #[test]
    fn is_arithop_false_for_non_operators() {
        assert!(!is_arithop(&TokenType::IntLit(1)));
        assert!(!is_arithop(&TokenType::Semi));
        assert!(!is_arithop(&TokenType::Ident("x".into())));
    }

    // --- binexpr with comparisons ---

    #[rstest]
    fn binexpr_equality_comparison_builds_equal_root(#[with("3 == 5")] testfr: TestFramework) {
        let tree = testfr.binexpr(0).expect("Expected a clean parsing");

        assert_eq!(tree, AstNode::make_binary(TokenType::EQ,
                                              AstNode::make_intlit(3, PrimType::Char),
                                              AstNode::make_intlit(5, PrimType::Char),
                                              PrimType::Char));
    }

    // --- type_compatibility ---

    #[test]
    fn type_compatibility_equal_types() {
        let tf = TestFramework::new("1");
        let cg = code_gen(&tf.sym_table);
        let expr = ExpressionGenerator::new(&tf.scanner, &tf.sym_table, &cg);
        assert!(matches!(expr.type_compatibility(PrimType::Int, PrimType::Int, false),
                         Compatibility::Compatible(PrimType::Int)));
    }

    #[test]
    fn type_compatibility_widen_left() {
        let tf = TestFramework::new("1");
        let cg = code_gen(&tf.sym_table);
        let expr = ExpressionGenerator::new(&tf.scanner, &tf.sym_table, &cg);
        assert!(matches!(expr.type_compatibility(PrimType::Char, PrimType::Int, false),
                         Compatibility::WidenLeft(PrimType::Int)));
    }

    #[test]
    fn type_compatibility_widen_right() {
        let tf = TestFramework::new("1");
        let cg = code_gen(&tf.sym_table);
        let expr = ExpressionGenerator::new(&tf.scanner, &tf.sym_table, &cg);
        assert!(matches!(expr.type_compatibility(PrimType::Int, PrimType::Char, false),
                         Compatibility::WidenRight(PrimType::Int)));
    }

    #[test]
    fn type_compatibility_only_right_blocks_widen() {
        let tf = TestFramework::new("1");
        let cg = code_gen(&tf.sym_table);
        let expr = ExpressionGenerator::new(&tf.scanner, &tf.sym_table, &cg);
        assert!(matches!(expr.type_compatibility(PrimType::Int, PrimType::Char, true),
                         Compatibility::Incompatible));
    }

    #[test]
    fn type_compatibility_void_incompatible() {
        let tf = TestFramework::new("1");
        let cg = code_gen(&tf.sym_table);
        let expr = ExpressionGenerator::new(&tf.scanner, &tf.sym_table, &cg);
        assert!(matches!(expr.type_compatibility(PrimType::Void, PrimType::Int, false),
                         Compatibility::Incompatible));
    }

    #[test]
    fn type_compatibility_same_size() {
        let tf = TestFramework::new("1");
        let cg = code_gen(&tf.sym_table);
        let expr = ExpressionGenerator::new(&tf.scanner, &tf.sym_table, &cg);
        assert!(matches!(expr.type_compatibility(PrimType::Long, PrimType::LongPtr, false),
                         Compatibility::Compatible(PrimType::Long)));
    }

    // --- enter/exit function ---

    #[test]
    fn enter_exit_function_tracks_context() {
        let scanner = scanner_from("1");
        let sym_table = SymbolTable::new();
        sym_table.add_glob_fn("main", PrimType::Int);
        let cg = code_gen(&sym_table);
        let expr = ExpressionGenerator::new(&scanner, &sym_table, &cg);
        expr.enter_function("main");
        assert_eq!(expr.current_function_type(), PrimType::Int);
        assert_eq!(expr.current_function_name(), "main");
        expr.exit_function();
    }

    // --- primary with function call ---

    #[test]
    fn primary_function_call() {
        let scanner = scanner_from("foo(42)");
        let sym_table = SymbolTable::new();
        sym_table.add_glob_fn("foo", PrimType::Int);
        let cg = code_gen(&sym_table);
        let expr = ExpressionGenerator::new(&scanner, &sym_table, &cg);
        let node = expr.primary().unwrap();
        assert!(matches!(node, AstNode::FuncCall { .. }));
        assert_eq!(node.get_type(), Some(PrimType::Int));
    }

    // --- primary with ident variable ---

    #[test]
    fn primary_ident_variable() {
        // must have trailing token so maybe_token(LeftParen) doesn't hit EOF
        let scanner = scanner_from("x;");
        let sym_table = SymbolTable::new();
        sym_table.add_glob("x", PrimType::Int, StructuralType::Variable);
        let cg = code_gen(&sym_table);
        let expr = ExpressionGenerator::new(&scanner, &sym_table, &cg);
        let node = expr.primary().unwrap();
        assert!(matches!(node, AstNode::Ident { .. }));
        assert_eq!(node.get_type(), Some(PrimType::Int));
    }

    // --- prefix with & and * ---

    #[test]
    fn prefix_amper_creates_addr() {
        // trailing token prevents maybe_token(LeftParen) from hitting EOF
        let scanner = scanner_from("&x;");
        let sym_table = SymbolTable::new();
        sym_table.add_glob("x", PrimType::Int, StructuralType::Variable);
        let cg = code_gen(&sym_table);
        let expr = ExpressionGenerator::new(&scanner, &sym_table, &cg);
        let node = expr.binexpr(0).unwrap();
        assert!(matches!(node, AstNode::Addr { .. }));
        assert_eq!(node.get_type(), Some(PrimType::IntPtr));
    }

    #[test]
    fn prefix_star_creates_deref() {
        // trailing token prevents maybe_token(LeftParen) from hitting EOF
        let scanner = scanner_from("*x;");
        let sym_table = SymbolTable::new();
        sym_table.add_glob("x", PrimType::IntPtr, StructuralType::Variable);
        let cg = code_gen(&sym_table);
        let expr = ExpressionGenerator::new(&scanner, &sym_table, &cg);
        let node = expr.binexpr(0).unwrap();
        assert!(matches!(node, AstNode::Deref { .. }));
        assert_eq!(node.get_type(), Some(PrimType::Int));
    }
}
