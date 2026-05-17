use anyhow::{Result, bail};
use crate::{
    ast::AstNode,
    scan::{Scanner, Token},
};

type Precedence = i16;

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
            Token::IntLit(val) => AstNode::IntLit(*val),
            Token::Ident(id) => AstNode::make_ident(id),
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

// Return an AST tree whose root is a binary operator.
// ptp is the precedence of the previous token
pub fn binexpr<T>(scanner: &Scanner<T>, ptp: Precedence) -> Result<AstNode>
    where T: std::io::Read,
{
    let mut left = primary(scanner);

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

        let right = binexpr(scanner, curr_prec)?;

        left = match token {
            Token::Plus|Token::Minus|Token::Star|Token::Slash|Token::EQ|Token::NE|Token::LT|Token::LE|Token::GT|Token::GE => {
                AstNode::make_binary(token, left, right)
            }
            _ => unreachable!("This shouldn't be reachable after we tested the op to be arithmetic")
        };
    }

    Ok(left)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::*;
    use crate::scan::Token;

    fn scanner_from(s: &str) -> Scanner<Cursor<Vec<u8>>> {
        Scanner::new(Cursor::new(s.as_bytes().to_vec()))
    }

    // --- primary ---

    #[test]
    fn primary_intlit_returns_leaf_node() {
        let scanner = scanner_from("42");
        let node = primary(&scanner);
        assert_eq!(node, AstNode::IntLit(42));
    }

    #[test]
    fn primary_ident_returns_leaf_node() {
        let scanner = scanner_from("x");
        let node = primary(&scanner);
        assert_eq!(node, AstNode::make_ident("x"));
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

    // --- binexpr ---

    #[test]
    fn binexpr_single_integer_returns_intlit_root() {
        let scanner = scanner_from("7");
        let tree = binexpr(&scanner, 0).expect("Expected a clean parsing");
        assert_eq!(tree, AstNode::IntLit(7));
    }

    #[test]
    fn binexpr_addition_builds_correct_tree() {
        let scanner = scanner_from("3 + 5");
        let tree = binexpr(&scanner, 0).expect("Expected a clean parsing");
        assert_eq!(tree, AstNode::make_binary(Token::Plus, AstNode::IntLit(3), AstNode::IntLit(5)));
    }

    #[test]
    fn binexpr_equal_precedence_is_left_associative() {
        // "2 - 3 + 5" parses as Add(Subtract(2, 3), 5): last op is root, left subtree holds earlier ops
        let scanner = scanner_from("2 - 3 + 5");
        let tree = binexpr(&scanner, 0).expect("Expected a clean parsing");
        assert_eq!(tree,
            AstNode::make_binary(Token::Plus,
                AstNode::make_binary(Token::Minus, AstNode::IntLit(2), AstNode::IntLit(3)),
                AstNode::IntLit(5)));
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

    #[test]
    fn binexpr_equality_comparison_builds_equal_root() {
        let scanner = scanner_from("3 == 5");
        let tree = binexpr(&scanner, 0).expect("Expected a clean parsing");

        assert_eq!(tree, AstNode::make_binary(Token::EQ, AstNode::IntLit(3), AstNode::IntLit(5)));
    }
}
