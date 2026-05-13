use crate::tree::IndexableNode;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Ast {
    // Operators
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    // Literals and identifiers
    IntLit(i64),
    Ident(String),
    LvIdent(String),
    // Statements
    GlobalDec(String),
    Assign,
    Glue,
    If,
    While,
    Print,
}

impl Ast {
    pub fn is_arith(&self) -> bool {
        matches!(self, Ast::Add|Ast::Subtract|Ast::Divide|Ast::Multiply)
    }

    pub fn is_comparison(&self) -> bool {
        matches!(self, Ast::Equal|Ast::NotEqual
                |Ast::LessThan|Ast::LessThanOrEqual
                |Ast::GreaterThan|Ast::GreaterThanOrEqual)
    }

    pub fn is_loop_with_comparison(&self) -> bool {
        matches!(self, Ast::If|Ast::While)
    }
}

pub struct AstNodeBuilder {
    op: Ast,
    left_index: Option<usize>,
    mid_index: Option<usize>,
    right_index: Option<usize>,
}

impl AstNodeBuilder {
    pub fn new(op: Ast) -> Self {
        Self { op, left_index: None, mid_index: None, right_index: None, }
    }

    pub fn left(mut self, idx: usize) -> Self {
        self.left_index = Some(idx);
        self
    }

    pub fn mid(mut self, idx: usize) -> Self {
        self.mid_index = Some(idx);
        self
    }

    pub fn right(mut self, idx: usize) -> Self {
        self.right_index = Some(idx);
        self
    }

    pub fn build(self) -> AstNode {
        AstNode::new(self.op, self.left_index, self.mid_index, self.right_index)
    }
}

#[derive(Debug)]
pub struct AstNode {
    pub op: Ast,
    left_index: Option<usize>,
    mid_index: Option<usize>,
    right_index: Option<usize>,
}

impl AstNode {
    pub fn new(op: Ast, left_index: Option<usize>, mid_index: Option<usize>, right_index: Option<usize>) -> Self {
        AstNode {
            op,
            left_index,
            mid_index,
            right_index,
        }
    }

    pub fn make_leaf(op: Ast) -> Self {
        AstNode::new(op, None, None, None)
    }

    pub fn make_unary(op: Ast, left_index: usize) -> Self {
        AstNode::new(op, Some(left_index), None, None)
    }
}

impl IndexableNode for AstNode {
    fn is_leaf(&self) -> bool {
        self.left_index.is_none() && self.right_index.is_none()
    }

    fn get_left_index(&self) -> Option<usize> {
        self.left_index
    }

    fn get_mid_index(&self) -> Option<usize> {
        self.mid_index
    }

    fn get_right_index(&self) -> Option<usize> {
        self.right_index
    }

    fn set_leaves(&mut self, left: Option<usize>, mid: Option<usize>, right: Option<usize>) {
        self.left_index = left;
        self.mid_index = mid;
        self.right_index = right;
    }

    fn shift_by(self, offset: usize) -> AstNode {
        AstNode {
            op: self.op,
            left_index: self.left_index.map(|v| v + offset),
            mid_index: self.mid_index.map(|v| v + offset),
            right_index: self.right_index.map(|v| v + offset),
        }
    }

    fn make_glue() -> AstNode {
        AstNode::make_leaf(Ast::Glue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::Tree;

    // --- AstNode construction ---

    #[test]
    fn astnode_new_leaf_stores_op_and_no_indices() {
        let node = AstNode::new(Ast::IntLit(42), None, None, None);
        assert!(matches!(node.op, Ast::IntLit(42)));
        assert!(node.left_index.is_none());
        assert!(node.mid_index.is_none());
        assert!(node.right_index.is_none());
    }

    #[test]
    fn astnode_new_binary_stores_op_and_all_indices() {
        let node = AstNode::new(Ast::Add, Some(0), Some(1), Some(2));
        assert!(matches!(node.op, Ast::Add));
        assert_eq!(node.left_index, Some(0));
        assert_eq!(node.mid_index, Some(1));
        assert_eq!(node.right_index, Some(2));
    }

    // --- IndexableNode trait impl ---

    #[test]
    fn indexable_node_leaf_returns_none() {
        let node = AstNode::new(Ast::IntLit(1), None, None, None);
        assert!(node.get_left_index().is_none());
        assert!(node.get_mid_index().is_none());
        assert!(node.get_right_index().is_none());
    }

    #[test]
    fn indexable_node_binary_returns_indices() {
        let node = AstNode::new(Ast::Multiply, Some(3), None, Some(7));
        assert_eq!(node.get_left_index(), Some(3));
        assert_eq!(node.get_mid_index(), None);
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
        let mut tree = Tree::new(AstNode::make_leaf(Ast::IntLit(3))); // idx 0
        let r = tree.append(AstNode::make_leaf(Ast::IntLit(5)), false); // idx 1
        tree.append(AstNode::new(Ast::Add, Some(0), None, Some(r)), true);          // idx 2, new root

        assert!(matches!(tree.get_root().unwrap().op, Ast::Add));
        assert_eq!(tree.get_root().unwrap().get_left_index(), Some(0));
        assert_eq!(tree.get_root().unwrap().get_right_index(), Some(r));
        assert!(matches!(tree.get_node(0).unwrap().op, Ast::IntLit(3)));
        assert!(matches!(tree.get_node(r).unwrap().op, Ast::IntLit(5)));
    }

    #[test]
    fn tree_nested_expression() {
        // (2 * 3) + 4
        let mut tree = Tree::new(AstNode::make_leaf(Ast::IntLit(2))); // idx 0
        let idx3 = tree.append(AstNode::make_leaf(Ast::IntLit(3)), false); // idx 1
        let mul  = tree.append(AstNode::new(Ast::Multiply, Some(0), None, Some(idx3)), false); // idx 2
        let idx4 = tree.append(AstNode::make_leaf(Ast::IntLit(4)),false); // idx 3
        tree.append(AstNode::new(Ast::Add, Some(mul), None, Some(idx4)), true);          // idx 4, new root

        assert!(matches!(tree.get_root().unwrap().op, Ast::Add));
        let left_idx = tree.get_root().unwrap().get_left_index().unwrap();
        assert!(matches!(tree.get_node(left_idx).unwrap().op, Ast::Multiply));
        let right_idx = tree.get_root().unwrap().get_right_index().unwrap();
        assert!(matches!(tree.get_node(right_idx).unwrap().op, Ast::IntLit(4)));
    }

    #[test]
    fn tree_get_node_out_of_bounds_returns_none() {
        let tree = Tree::new(AstNode::make_leaf(Ast::IntLit(1)));
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
        let node = AstNode::new(Ast::Add, Some(1), None, Some(2));
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
        node.set_leaves(Some(7), None, Some(9));
        assert_eq!(node.left_index, Some(7));
        assert_eq!(node.mid_index, None);
        assert_eq!(node.right_index, Some(9));
    }

    #[test]
    fn set_leaves_can_clear_indices_to_none() {
        let mut node = AstNode::new(Ast::Add, Some(1), Some(2), Some(3));
        node.set_leaves(None, None, None);
        assert!(node.left_index.is_none());
        assert!(node.mid_index.is_none());
        assert!(node.right_index.is_none());
    }

    // --- Ast predicate methods ---

    #[test]
    fn is_arith_true_for_arithmetic_ops() {
        assert!(Ast::Add.is_arith());
        assert!(Ast::Subtract.is_arith());
        assert!(Ast::Multiply.is_arith());
        assert!(Ast::Divide.is_arith());
    }

    #[test]
    fn is_arith_false_for_non_arithmetic() {
        assert!(!Ast::Equal.is_arith());
        assert!(!Ast::If.is_arith());
        assert!(!Ast::IntLit(1).is_arith());
        assert!(!Ast::While.is_arith());
    }

    #[test]
    fn is_comparison_true_for_comparisons() {
        assert!(Ast::Equal.is_comparison());
        assert!(Ast::NotEqual.is_comparison());
        assert!(Ast::LessThan.is_comparison());
        assert!(Ast::LessThanOrEqual.is_comparison());
        assert!(Ast::GreaterThan.is_comparison());
        assert!(Ast::GreaterThanOrEqual.is_comparison());
    }

    #[test]
    fn is_comparison_false_for_non_comparisons() {
        assert!(!Ast::Add.is_comparison());
        assert!(!Ast::While.is_comparison());
        assert!(!Ast::IntLit(0).is_comparison());
    }

    #[test]
    fn is_loop_with_comparison_true_for_if_and_while() {
        assert!(Ast::If.is_loop_with_comparison());
        assert!(Ast::While.is_loop_with_comparison());
    }

    #[test]
    fn is_loop_with_comparison_false_for_others() {
        assert!(!Ast::Add.is_loop_with_comparison());
        assert!(!Ast::Equal.is_loop_with_comparison());
        assert!(!Ast::IntLit(5).is_loop_with_comparison());
    }

    // --- AstNodeBuilder ---

    #[test]
    fn astnode_builder_builds_node_with_all_indices() {
        let node = AstNodeBuilder::new(Ast::If)
            .left(1)
            .mid(2)
            .right(3)
            .build();
        assert!(matches!(node.op, Ast::If));
        assert_eq!(node.left_index, Some(1));
        assert_eq!(node.mid_index, Some(2));
        assert_eq!(node.right_index, Some(3));
    }

    #[test]
    fn astnode_builder_defaults_to_no_indices() {
        let node = AstNodeBuilder::new(Ast::Add).build();
        assert!(node.left_index.is_none());
        assert!(node.mid_index.is_none());
        assert!(node.right_index.is_none());
    }

    // --- IndexableNode::make_glue ---

    #[test]
    fn make_glue_creates_glue_leaf_with_no_indices() {
        let node = AstNode::make_glue();
        assert!(matches!(node.op, Ast::Glue));
        assert!(node.get_left_index().is_none());
        assert!(node.get_mid_index().is_none());
        assert!(node.get_right_index().is_none());
    }
}
