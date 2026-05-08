use crate::tree::IndexableNode;

/* Grammar
 * expression: number
 *           | expression '*' expression
 *           | expression '/' expression
 *           | expression '+' expression
 *           | expression '-' expression
 *           ;
 *
 * number: T_INTLIT
 *         ;
 */

#[derive(Debug)]
pub enum Ast {
    Add,
    Subtract,
    Multiply,
    Divide,
    IntLit(i64),
}

#[derive(Debug)]
pub struct AstNode {
    pub op: Ast,
    left_index: Option<usize>,
    right_index: Option<usize>,
}

impl AstNode {
    pub fn new(op: Ast, left_index: Option<usize>, right_index: Option<usize>) -> Self {
        AstNode {
            op,
            left_index,
            right_index,
        }
    }

    pub fn make_leaf(op: Ast) -> Self {
        AstNode::new(op, None, None)
    }

    pub fn make_unary(op: Ast, left_index: usize) -> Self {
        AstNode::new(op, Some(left_index), None)
    }
}

impl IndexableNode for AstNode {
    fn get_left_index(&self) -> Option<usize> {
        self.left_index
    }

    fn get_right_index(&self) -> Option<usize> {
        self.right_index
    }

    fn set_leaves(&mut self, left: Option<usize>, right: Option<usize>) {
        self.left_index = left;
        self.right_index = right;
    }

    fn shift_by(self, offset: usize) -> AstNode {
        AstNode {
            op: self.op,
            left_index: self.left_index.map(|v| v + offset),
            right_index: self.right_index.map(|v| v + offset),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::Tree;

    // --- AstNode construction ---

    #[test]
    fn astnode_new_leaf_stores_op_and_no_indices() {
        let node = AstNode::new(Ast::IntLit(42), None, None);
        assert!(matches!(node.op, Ast::IntLit(42)));
        assert!(node.left_index.is_none());
        assert!(node.right_index.is_none());
    }

    #[test]
    fn astnode_new_binary_stores_op_and_both_indices() {
        let node = AstNode::new(Ast::Add, Some(0), Some(1));
        assert!(matches!(node.op, Ast::Add));
        assert_eq!(node.left_index, Some(0));
        assert_eq!(node.right_index, Some(1));
    }

    // --- IndexableNode trait impl ---

    #[test]
    fn indexable_node_leaf_returns_none() {
        let node = AstNode::new(Ast::IntLit(1), None, None);
        assert!(node.get_left_index().is_none());
        assert!(node.get_right_index().is_none());
    }

    #[test]
    fn indexable_node_binary_returns_indices() {
        let node = AstNode::new(Ast::Multiply, Some(3), Some(7));
        assert_eq!(node.get_left_index(), Some(3));
        assert_eq!(node.get_right_index(), Some(7));
    }

    // --- Ast debug representation ---

    #[test]
    fn ast_debug_intlit_contains_value() {
        let s = format!("{:?}", Ast::IntLit(99));
        assert!(s.contains("IntLit") && s.contains("99"));
    }

    #[test]
    fn ast_debug_operators() {
        assert!(format!("{:?}", Ast::Add).contains("Add"));
        assert!(format!("{:?}", Ast::Subtract).contains("Subtract"));
        assert!(format!("{:?}", Ast::Multiply).contains("Multiply"));
        assert!(format!("{:?}", Ast::Divide).contains("Divide"));
    }

    // --- Integration: building AST trees ---

    #[test]
    fn tree_simple_addition() {
        // 3 + 5
        let mut tree = Tree::new(AstNode::new(Ast::IntLit(3), None, None)); // idx 0
        let r = tree.append(AstNode::new(Ast::IntLit(5), None, None), false); // idx 1
        tree.append(AstNode::new(Ast::Add, Some(0), Some(r)), true);          // idx 2, new root

        assert!(matches!(tree.get_root().op, Ast::Add));
        assert_eq!(tree.get_root().get_left_index(), Some(0));
        assert_eq!(tree.get_root().get_right_index(), Some(r));
        assert!(matches!(tree.get_node(0).unwrap().op, Ast::IntLit(3)));
        assert!(matches!(tree.get_node(r).unwrap().op, Ast::IntLit(5)));
    }

    #[test]
    fn tree_nested_expression() {
        // (2 * 3) + 4
        let mut tree = Tree::new(AstNode::new(Ast::IntLit(2), None, None)); // idx 0
        let idx3 = tree.append(AstNode::new(Ast::IntLit(3), None, None), false); // idx 1
        let mul  = tree.append(AstNode::new(Ast::Multiply, Some(0), Some(idx3)), false); // idx 2
        let idx4 = tree.append(AstNode::new(Ast::IntLit(4), None, None), false); // idx 3
        tree.append(AstNode::new(Ast::Add, Some(mul), Some(idx4)), true);          // idx 4, new root

        assert!(matches!(tree.get_root().op, Ast::Add));
        let left_idx = tree.get_root().get_left_index().unwrap();
        assert!(matches!(tree.get_node(left_idx).unwrap().op, Ast::Multiply));
        let right_idx = tree.get_root().get_right_index().unwrap();
        assert!(matches!(tree.get_node(right_idx).unwrap().op, Ast::IntLit(4)));
    }

    #[test]
    fn tree_get_node_out_of_bounds_returns_none() {
        let tree = Tree::new(AstNode::new(Ast::IntLit(1), None, None));
        assert!(tree.get_node(999).is_none());
    }

    // --- AstNode factory helpers ---

    #[test]
    fn make_leaf_creates_node_with_no_indices() {
        let node = AstNode::make_leaf(Ast::IntLit(5));
        assert!(matches!(node.op, Ast::IntLit(5)));
        assert!(node.left_index.is_none());
        assert!(node.right_index.is_none());
    }

    #[test]
    fn make_unary_sets_left_index_only() {
        let node = AstNode::make_unary(Ast::Add, 3);
        assert!(matches!(node.op, Ast::Add));
        assert_eq!(node.left_index, Some(3));
        assert!(node.right_index.is_none());
    }

    // --- IndexableNode: shift_by and set_leaves ---

    #[test]
    fn shift_by_adjusts_both_indices_by_offset() {
        let node = AstNode::new(Ast::Add, Some(1), Some(2));
        let shifted = node.shift_by(10);
        assert_eq!(shifted.left_index, Some(11));
        assert_eq!(shifted.right_index, Some(12));
    }

    #[test]
    fn shift_by_leaf_leaves_none_indices_unchanged() {
        let node = AstNode::make_leaf(Ast::IntLit(42));
        let shifted = node.shift_by(5);
        assert!(shifted.left_index.is_none());
        assert!(shifted.right_index.is_none());
    }

    #[test]
    fn set_leaves_overwrites_both_indices() {
        let mut node = AstNode::make_leaf(Ast::Add);
        node.set_leaves(Some(7), Some(9));
        assert_eq!(node.left_index, Some(7));
        assert_eq!(node.right_index, Some(9));
    }

    #[test]
    fn set_leaves_can_clear_indices_to_none() {
        let mut node = AstNode::new(Ast::Add, Some(1), Some(2));
        node.set_leaves(None, None);
        assert!(node.left_index.is_none());
        assert!(node.right_index.is_none());
    }
}
