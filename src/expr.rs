use crate::{
    ast::{Ast, AstNode},
    scan::{Scanner, Token},
    tree::Tree,
};

pub fn primary<T>(scanner: &Scanner<T>) -> AstNode
    where T: std::io::Read
{
    // For an INTLIT token, make a leaf AST node for it,
    // Otherwise, a syntax error for any other token type

    if let Some(token) = scanner.scan() {
        match token {
            Token::IntLit(val) => AstNode::make_leaf(Ast::IntLit(val)),
            _ => panic!("syntax error on line {}", scanner.get_line())
        }
    } else {
        panic!("EOF reached, expected an integer")
    }
}

pub fn arithop<T>(scanner: &Scanner<T>, token: Token) -> Ast
    where T: std::io::Read
{
    match token {
        Token::Plus => Ast::Add,
        Token::Minus => Ast::Subtract,
        Token::Star => Ast::Multiply,
        Token::Slash => Ast::Divide,
        _ => panic!("unknown token in airthop on line {}", scanner.get_line())
    }
}

pub fn binexpr<T>(scanner: &Scanner<T>) -> Tree<AstNode>
    where T: std::io::Read
{
    let left = Tree::new(primary(scanner));

    if let Some(token) = scanner.scan() {
        let node_type = arithop(scanner, token);
        let right = binexpr(scanner);

        let (mut new_tree, right_root) = left.concat(right);
        new_tree.new_root_with_right_idx(AstNode::make_leaf(node_type), right_root);

        new_tree
    }
    else {
        left
    }
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
        let tree = binexpr(&scanner);
        assert!(matches!(tree.get_root().op, Ast::IntLit(7)));
        assert!(tree.get_root().get_left_index().is_none());
    }

    #[test]
    fn binexpr_addition_builds_correct_tree() {
        let scanner = scanner_from("3 + 5");
        let tree = binexpr(&scanner);
        assert!(matches!(tree.get_root().op, Ast::Add));
        let left = tree.get_node(tree.get_root().get_left_index().unwrap()).unwrap();
        assert!(matches!(left.op, Ast::IntLit(3)));
        let right = tree.get_node(tree.get_root().get_right_index().unwrap()).unwrap();
        assert!(matches!(right.op, Ast::IntLit(5)));
    }

    #[test]
    fn binexpr_is_right_recursive_not_left_associative() {
        // "2 - 3 + 5" parses as Subtract(2, Add(3, 5)) due to right-recursion
        let scanner = scanner_from("2 - 3 + 5");
        let tree = binexpr(&scanner);
        assert!(matches!(tree.get_root().op, Ast::Subtract));
        let right_idx = tree.get_root().get_right_index().unwrap();
        assert!(matches!(tree.get_node(right_idx).unwrap().op, Ast::Add));
    }
}
