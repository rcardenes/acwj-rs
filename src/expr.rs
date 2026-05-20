use anyhow::{Result, bail};
use crate::{
    ast::AstNode,
    scan::{Scanner, Token},
    sym::{PrimType, SymbolTable},
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
fn op_precedence(line: usize, token: &Token) -> Result<Precedence> {
    Ok(match token {
        Token::Plus|Token::Minus => 10,
        Token::Star|Token::Slash => 20,
        Token::EQ|Token::NE => 30,
        Token::LT|Token::LE|Token::GT|Token::GE => 40,
        _  => {
            bail!("syntax error on line {}, token {:?}", line, token)
        }
    })
}

fn is_arithop(token: &Token) -> bool {
    matches!(token, Token::Plus
            |Token::Minus
            |Token::Star
            |Token::Slash
            |Token::EQ
            |Token::NE
            |Token::LT
            |Token::LE
            |Token::GT
            |Token::GE)
}

// Return if two primitive types are compatible.
// If only_right is true, then widening can happen only left to right
pub fn is_type_compatible(left: PrimType, right: PrimType, only_right: bool) -> Compatibility {
    match (left, right) {
        // Voids are not compatible with anything
        (PrimType::Void, _) | (_, PrimType::Void) => Compatibility::Incompatible,
        // Any other type is compatible with itself
        (x, y) if x == y => Compatibility::Compatible(x),
        // Widen Char to Int is required
        (PrimType::Char, PrimType::Int) => Compatibility::WidenLeft(PrimType::Int),
        (PrimType::Int, PrimType::Char) => if only_right {
            Compatibility::Incompatible
        } else {
            Compatibility::WidenRight(PrimType::Int)
        },
        // Anything else is compatible
        _ => unimplemented!(),
    }
}

pub struct ExpressionGenerator<'a, T> {
    scanner: &'a Scanner<T>,
    sym_table: &'a SymbolTable,
}

impl<'a, T> ExpressionGenerator<'a, T>
    where T: std::io::Read,
{
    pub fn new(scanner: &'a Scanner<T>, sym_table: &'a SymbolTable) -> Self {
        ExpressionGenerator { scanner, sym_table }
    }

    pub fn primary(&self) -> AstNode
    {
        // For an INTLIT token, make a leaf AST node for it,
        // Otherwise, a syntax error for any other token type

        if let Some(token) = self.scanner.scan() {
            match &token {
                // For an IntLit token, make it a Char if it is within that type's range,
                // so that we don't have to narrow it later if needed. Widen the data is
                // always possible.
                Token::IntLit(val) => if *val >= 0 && *val < 256 {
                    AstNode::make_intlit(*val, PrimType::Char)
                } else {
                    AstNode::make_intlit(*val, PrimType::Int)
                },
                Token::Ident(id) => {
                    if let Some(found) = self.sym_table.find_glob(id) {
                        AstNode::make_ident(id, found.dtype)
                    } else {
                        self.scanner.fatal_extra("Unknown variable", id)
                    }
                },
                _ => self.scanner.fatal_extra("Syntax error, token", token)
            }
        } else {
            panic!("EOF reached, expected an integer")
        }
    }

    // Return an AST tree whose root is a binary operator.
    // ptp is the precedence of the previous token
    pub fn binexpr(&self, ptp: Precedence) -> Result<AstNode>
        where T: std::io::Read,
    {
        let mut left = self.primary();

        while let Some(token) = self.scanner.scan() {
            if !is_arithop(&token) {
                self.scanner.putback_token(token);
                break;
            }

            let curr_prec = op_precedence(self.scanner.get_line(), &token)?;
            if curr_prec <= ptp {
                self.scanner.putback_token(token);
                break;
            }

            let mut right = self.binexpr(curr_prec)?;
            let compat = if let (Some(left_type), Some(right_type)) = (left.get_type(), right.get_type()) {
                is_type_compatible(left_type, right_type, false)
            } else {
                Compatibility::Incompatible
            };

            let bin_type = match compat {
                Compatibility::Incompatible => self.scanner.fatal("Incompatible types"),
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

            left = match token {
                Token::Plus|Token::Minus|Token::Star|Token::Slash|Token::EQ|Token::NE|Token::LT|Token::LE|Token::GT|Token::GE => {
                    AstNode::make_binary(token, left, right, bin_type)
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

    fn scanner_from(s: &str) -> Scanner<Cursor<Vec<u8>>> {
        Scanner::new(Cursor::new(s.as_bytes().to_vec()))
    }

    type TestInput = Cursor<Vec<u8>>;

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

        fn primary(&self) -> AstNode {
            ExpressionGenerator::new(&self.scanner, &self.sym_table).primary()
        }

        fn binexpr(&self, ptp: Precedence) -> Result<AstNode> {
            ExpressionGenerator::new(&self.scanner, &self.sym_table).binexpr(ptp)
        }
    }

    #[fixture]
    fn testfr(#[default("")] text: &str) -> TestFramework {
        TestFramework::new(text)
    }


    // --- primary ---

    #[rstest]
    fn primary_intlit_returns_leaf_node(#[with("42")] testfr: TestFramework) {
        let node = testfr.primary();
        assert_eq!(node, AstNode::make_intlit(42, PrimType::Char));
    }

    #[rstest]
    fn primary_intlit_in_char_range(#[with("255")] testfr: TestFramework) {
        let node = testfr.primary();
        assert_eq!(node, AstNode::make_intlit(255, PrimType::Char));
    }

    #[rstest]
    #[should_panic]
    fn primary_panics_on_operator_token(#[with("+")] testfr: TestFramework) {
        testfr.primary();
    }

    #[rstest]
    #[should_panic]
    fn primary_panics_at_eof(#[with("")] testfr: TestFramework) {
        testfr.primary();
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
        assert_eq!(tree, AstNode::make_binary(Token::Plus,
                                              AstNode::make_intlit(3, PrimType::Char),
                                              AstNode::make_intlit(5, PrimType::Char),
                                              PrimType::Char));
    }

    #[rstest]
    // "2 - 3 + 5" parses as Add(Subtract(2, 3), 5): last op is root, left subtree holds earlier ops
    fn binexpr_equal_precedence_is_left_associative(#[with("2 - 3 + 5")] testfr: TestFramework) {
        let tree = testfr.binexpr(0).expect("Expected a clean parsing");
        assert_eq!(tree,
            AstNode::make_binary(Token::Plus,
                AstNode::make_binary(Token::Minus,
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
        assert_eq!(op_precedence(line, &Token::Plus).unwrap(), 10);
        assert_eq!(op_precedence(line, &Token::Minus).unwrap(), 10);
        assert_eq!(op_precedence(line, &Token::Star).unwrap(), 20);
        assert_eq!(op_precedence(line, &Token::Slash).unwrap(), 20);
        assert_eq!(op_precedence(line, &Token::EQ).unwrap(), 30);
        assert_eq!(op_precedence(line, &Token::NE).unwrap(), 30);
        assert_eq!(op_precedence(line, &Token::LT).unwrap(), 40);
        assert_eq!(op_precedence(line, &Token::LE).unwrap(), 40);
        assert_eq!(op_precedence(line, &Token::GT).unwrap(), 40);
        assert_eq!(op_precedence(line, &Token::GE).unwrap(), 40);
    }

    #[test]
    fn op_precedence_fails_on_non_operator() {
        assert!(op_precedence(1, &Token::Semi).is_err());
        assert!(op_precedence(1, &Token::IntLit(1)).is_err());
    }

    // --- is_arithop ---

    #[test]
    fn is_arithop_true_for_all_operator_tokens() {
        for tok in &[Token::Plus, Token::Minus, Token::Star, Token::Slash,
                     Token::EQ, Token::NE, Token::LT, Token::LE, Token::GT, Token::GE] {
            assert!(is_arithop(tok), "{tok:?} should be an arith op");
        }
    }

    #[test]
    fn is_arithop_false_for_non_operators() {
        assert!(!is_arithop(&Token::IntLit(1)));
        assert!(!is_arithop(&Token::Semi));
        assert!(!is_arithop(&Token::Ident("x".into())));
    }

    // --- binexpr with comparisons ---

    #[rstest]
    fn binexpr_equality_comparison_builds_equal_root(#[with("3 == 5")] testfr: TestFramework) {
        let tree = testfr.binexpr(0).expect("Expected a clean parsing");

        assert_eq!(tree, AstNode::make_binary(Token::EQ,
                                              AstNode::make_intlit(3, PrimType::Char),
                                              AstNode::make_intlit(5, PrimType::Char),
                                              PrimType::Char));
    }
}
