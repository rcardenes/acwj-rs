use anyhow::{Context, Result, bail};
use crate::{
    ast::{AstNode, Ast},
    tree::{Tree, IndexableNode},
};

pub fn interpret_ast(tree: &Tree<AstNode>, idx: Option<usize>) -> Result<i64> {
    let node = tree.get_root_or_node(idx).with_context(|| format!("Invalid node index: {idx:?}"))?;

    let (left_value, right_value) = if let Ast::IntLit(_) = node.op {
        (0, 0)
    } else {
        (interpret_ast(tree, node.get_left_index())?,
         interpret_ast(tree, node.get_right_index())?)
    };

    match &node.op {
        Ast::IntLit(val) => {
            Ok(*val)
        },
        Ast::Add => {
            Ok(left_value + right_value)
        },
        Ast::Subtract => {
            Ok(left_value - right_value)
        },
        Ast::Multiply => {
            Ok(left_value * right_value)
        },
        Ast::Divide => {
            Ok(left_value / right_value)
        },
        // op => {
        //     bail!("Unknown AST operator {:?}", op)
        // }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Ast, AstNode};
    use crate::tree::Tree;

    fn intlit_tree(val: i64) -> Tree<AstNode> {
        Tree::new(AstNode::make_leaf(Ast::IntLit(val)))
    }

    fn binop_tree(op: Ast, left_val: i64, right_val: i64) -> Tree<AstNode> {
        let mut tree = Tree::new(AstNode::make_leaf(Ast::IntLit(left_val))); // idx 0
        let right_idx = tree.append(AstNode::make_leaf(Ast::IntLit(right_val)), false); // idx 1
        tree.append(AstNode::new(op, Some(0), Some(right_idx)), true);       // idx 2, root
        tree
    }

    #[test]
    fn interpret_intlit_returns_value() {
        assert_eq!(interpret_ast(&intlit_tree(42), None).unwrap(), 42);
    }

    #[test]
    fn interpret_add() {
        assert_eq!(interpret_ast(&binop_tree(Ast::Add, 3, 5), None).unwrap(), 8);
    }

    #[test]
    fn interpret_subtract() {
        assert_eq!(interpret_ast(&binop_tree(Ast::Subtract, 10, 4), None).unwrap(), 6);
    }

    #[test]
    fn interpret_multiply() {
        assert_eq!(interpret_ast(&binop_tree(Ast::Multiply, 6, 7), None).unwrap(), 42);
    }

    #[test]
    fn interpret_divide() {
        assert_eq!(interpret_ast(&binop_tree(Ast::Divide, 15, 3), None).unwrap(), 5);
    }

    #[test]
    fn interpret_nested_expression() {
        // (2 * 3) + 4 = 10
        let mut tree = Tree::new(AstNode::make_leaf(Ast::IntLit(2)));                  // idx 0
        let idx3 = tree.append(AstNode::make_leaf(Ast::IntLit(3)), false);             // idx 1
        let mul  = tree.append(AstNode::new(Ast::Multiply, Some(0), Some(idx3)), false); // idx 2
        let idx4 = tree.append(AstNode::make_leaf(Ast::IntLit(4)), false);             // idx 3
        tree.append(AstNode::new(Ast::Add, Some(mul), Some(idx4)), true);              // idx 4, root

        assert_eq!(interpret_ast(&tree, None).unwrap(), 10);
    }

    #[test]
    fn interpret_with_node_index_evaluates_subtree() {
        let tree = binop_tree(Ast::Add, 3, 5); // root is Add, idx 0 is IntLit(3)
        assert_eq!(interpret_ast(&tree, Some(0)).unwrap(), 3);
    }

    #[test]
    fn interpret_invalid_index_returns_error() {
        let tree = intlit_tree(1);
        assert!(interpret_ast(&tree, Some(999)).is_err());
    }
}
