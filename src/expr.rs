use anyhow::{Result, bail};
use crate::{
    ast::{Ast, AstNode},
    scan::{Scanner, Token},
    tree::Tree,
};

type Precedence = u8;

/// Return numeric precence for the different tokens, so that we
/// can use it in a Pratt-style parser.
pub fn op_precedence(line: usize, token: &Token) -> Result<Precedence> {
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

pub fn primary<T>(scanner: &Scanner<T>) -> AstNode
    where T: std::io::Read,
{
    // For an INTLIT token, make a leaf AST node for it,
    // Otherwise, a syntax error for any other token type

    if let Some(token) = scanner.scan() {
        match &token {
            Token::IntLit(val) => AstNode::make_leaf(Ast::IntLit(*val)),
            Token::Ident(id) => AstNode::make_leaf(Ast::Ident(id.into())),
            _ => scanner.fatal_extra("Syntax error, token", token)
        }
    } else {
        panic!("EOF reached, expected an integer")
    }
}

pub fn is_arithop(token: &Token) -> bool {
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

pub fn arithop<T>(scanner: &Scanner<T>, token: Token) -> Ast
    where T: std::io::Read
{
    match token {
        Token::Plus => Ast::Add,
        Token::Minus => Ast::Subtract,
        Token::Star => Ast::Multiply,
        Token::Slash => Ast::Divide,
        Token::EQ => Ast::Equal,
        Token::NE => Ast::NotEqual,
        Token::LT => Ast::LessThan,
        Token::LE => Ast::LessThanOrEqual,
        Token::GT => Ast::GreaterThan,
        Token::GE => Ast::GreaterThanOrEqual,
        _ => scanner.fatal_extra("Syntax error, token", token)
    }
}

// Return an AST tree whose root is a binary operator.
// ptp is the precedence of the previous token
pub fn binexpr<T>(scanner: &Scanner<T>, ptp: Precedence) -> Result<Tree<AstNode>>
    where T: std::io::Read,
{
    let mut left = Tree::new(primary(scanner));

    while let Some(token) = scanner.scan() {
        if !is_arithop(&token) {
            scanner.putback_token(token);
            break;
        }

        let curr_prec = op_precedence(scanner.get_line(), &token)?;
        if curr_prec <= ptp {
            scanner.putback_token(token);
            break;
        }

        let node_type = arithop(scanner, token);
        let right = binexpr(scanner, curr_prec)?;

        let (new_tree, right_root) = left.concat(right);
        left = new_tree.new_root_with_right_idx(AstNode::make_leaf(node_type), right_root.unwrap());
            // AstNode::new(arithop(scanner, token), Some(left), Some(right));
    }

    Ok(left)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::*;
    use crate::ast::Ast;
    use crate::scan::Token;
    use crate::tree::IndexableNode;

    fn scanner_from(s: &str) -> Scanner<Cursor<Vec<u8>>> {
        Scanner::new(Cursor::new(s.as_bytes().to_vec()))
    }

    // --- primary ---

    #[test]
    fn primary_intlit_returns_leaf_node() {
        let scanner = scanner_from("42");
        let node = primary(&scanner);
        assert!(matches!(node.op, Ast::IntLit(42)));
        assert!(node.get_left_index().is_none());
        assert!(node.get_right_index().is_none());
    }

    #[test]
    fn primary_ident_returns_leaf_node() {
        let scanner = scanner_from("x");
        let node = primary(&scanner);
        assert!(matches!(node.op, Ast::Ident(ref s) if s == "x"));
        assert!(node.get_left_index().is_none());
        assert!(node.get_right_index().is_none());
    }

    #[test]
    #[should_panic]
    fn primary_panics_on_operator_token() {
        let scanner = scanner_from("+");
        primary(&scanner);
    }

    #[test]
    #[should_panic]
    fn primary_panics_at_eof() {
        let scanner = scanner_from("");
        primary(&scanner);
    }

    // --- arithop ---

    #[test]
    fn arithop_plus_yields_add() {
        let scanner = scanner_from("");
        assert!(matches!(arithop(&scanner, Token::Plus), Ast::Add));
    }

    #[test]
    fn arithop_minus_yields_subtract() {
        let scanner = scanner_from("");
        assert!(matches!(arithop(&scanner, Token::Minus), Ast::Subtract));
    }

    #[test]
    fn arithop_star_yields_multiply() {
        let scanner = scanner_from("");
        assert!(matches!(arithop(&scanner, Token::Star), Ast::Multiply));
    }

    #[test]
    fn arithop_slash_yields_divide() {
        let scanner = scanner_from("");
        assert!(matches!(arithop(&scanner, Token::Slash), Ast::Divide));
    }

    #[test]
    #[should_panic]
    fn arithop_panics_on_intlit_token() {
        let scanner = scanner_from("");
        arithop(&scanner, Token::IntLit(1));
    }

    // --- binexpr ---

    #[test]
    fn binexpr_single_integer_returns_intlit_root() {
        let scanner = scanner_from("7");
        let tree = binexpr(&scanner, 0).expect("Expected a clean parsing");
        assert!(matches!(tree.get_root().unwrap().op, Ast::IntLit(7)));
        assert!(tree.get_root().unwrap().get_left_index().is_none());
    }

    #[test]
    fn binexpr_addition_builds_correct_tree() {
        let scanner = scanner_from("3 + 5");
        let tree = binexpr(&scanner, 0).expect("Expected a clean parsing");
        assert!(matches!(tree.get_root().unwrap().op, Ast::Add));
        let left = tree.get_node(tree.get_root().unwrap().get_left_index().unwrap()).unwrap();
        assert!(matches!(left.op, Ast::IntLit(3)));
        let right = tree.get_node(tree.get_root().unwrap().get_right_index().unwrap()).unwrap();
        assert!(matches!(right.op, Ast::IntLit(5)));
    }

    #[test]
    fn binexpr_equal_precedence_is_left_associative() {
        // "2 - 3 + 5" parses as Add(Subtract(2, 3), 5): last op is root, left subtree holds earlier ops
        let scanner = scanner_from("2 - 3 + 5");
        let tree = binexpr(&scanner, 0).expect("Expected a clean parsing");
        assert!(matches!(tree.get_root().unwrap().op, Ast::Add));
        let left_idx = tree.get_root().unwrap().get_left_index().unwrap();
        assert!(matches!(tree.get_node(left_idx).unwrap().op, Ast::Subtract));
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

    // --- arithop comparison variants ---

    #[test]
    fn arithop_comparison_operators() {
        let s = scanner_from("");
        assert!(matches!(arithop(&s, Token::EQ), Ast::Equal));
        assert!(matches!(arithop(&s, Token::NE), Ast::NotEqual));
        assert!(matches!(arithop(&s, Token::LT), Ast::LessThan));
        assert!(matches!(arithop(&s, Token::LE), Ast::LessThanOrEqual));
        assert!(matches!(arithop(&s, Token::GT), Ast::GreaterThan));
        assert!(matches!(arithop(&s, Token::GE), Ast::GreaterThanOrEqual));
    }

    // --- binexpr with comparisons ---

    #[test]
    fn binexpr_equality_comparison_builds_equal_root() {
        let scanner = scanner_from("3 == 5");
        let tree = binexpr(&scanner, 0).expect("Expected a clean parsing");
        assert!(matches!(tree.get_root().unwrap().op, Ast::Equal));
        let left = tree.get_node(tree.get_root().unwrap().get_left_index().unwrap()).unwrap();
        assert!(matches!(left.op, Ast::IntLit(3)));
        let right = tree.get_node(tree.get_root().unwrap().get_right_index().unwrap()).unwrap();
        assert!(matches!(right.op, Ast::IntLit(5)));
    }
}
