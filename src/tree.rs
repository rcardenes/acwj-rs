pub trait IndexableNode {
    fn is_leaf(&self) -> bool;
    fn get_left_index(&self) -> Option<usize>;
    fn get_mid_index(&self) -> Option<usize>;
    fn get_right_index(&self) -> Option<usize>;
    fn set_leaves(&mut self, left: Option<usize>, mid: Option<usize>, right: Option<usize>);
    fn shift_by(self, offset: usize) -> Self;
    fn make_glue() -> Self;
}

#[derive(Debug)]
pub struct Tree<T: IndexableNode> {
    root: Option<usize>,
    nodes: Vec<T>,
}

impl<T> Tree<T>
    where T: IndexableNode,
{
    // Begin -- Constructors
    pub fn new(node: T) -> Self {
        Tree {
            root: Some(0),
            nodes: vec![node],
        }
    }

    pub fn empty() -> Self {
        Tree {
            root: None,
            nodes: vec![],
        }
    }
    // End -- Constructors

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn append(&mut self, node: T, new_root: bool) -> usize {
        let idx = self.nodes.len();

        self.nodes.push(node);
        if new_root {
            self.root = Some(idx)
        }

        idx
    }

    pub fn new_root(mut self, mut node: T) -> Self {
        if node.get_left_index().is_some() {
            panic!("Tried to set a new root that has a left branch");
        }

        if let Some(root_idx) = self.root {
            node.set_leaves(Some(root_idx), None, None);
        }
        self.append(node, true);
        self
    }

    pub fn new_root_with_right_idx(mut self, mut node: T, idx: usize) -> Self {
        let old_root = self.root;
        node.set_leaves(old_root, None, Some(idx));
        self.append(node, true);
        self
    }

    pub fn new_root_with_left_idx(mut self, mut node: T, idx: usize) -> Self {
        let old_root = self.root;
        node.set_leaves(Some(idx), None, old_root);
        self.append(node, true);
        self
    }

    pub fn concat(self, other: Tree<T>) -> (Tree<T>, Option<usize>) {
        if self.is_empty() {
            (other, None)
        } else if other.is_empty() {
            (self, None)
        } else if let Some(other_root) = other.root {
            let first_index = self.nodes.len();
            let extended = other.nodes.into_iter().map(|n| n.shift_by(first_index)).collect::<Vec<_>>();

            let new_tree = Tree {
                root: self.root,
                nodes: self.nodes.into_iter().chain(extended).collect()
            };

            (new_tree, Some(other_root + first_index))
        } else {
            panic!("Can't concatenate rootless, non-empty trees")
        }
    }

    pub fn glue(self, right: Tree<T>) -> Self {
        let (cnt, right_idx) = self.concat(right);
        if let Some(idx) = right_idx {
            cnt.new_root_with_right_idx(T::make_glue(), idx)
        } else {
            panic!("Trying to glue a rootless tree")
        }
    }

    pub fn set_root_indices(&mut self, left: Option<usize>, mid: Option<usize>, right: Option<usize>) {
        if let Some(root) = self.get_root_mut() {
            root.set_leaves(left, mid, right);
        };
    }

    pub fn get_root_mut(&mut self) -> Option<&mut T> {
        self.root.map(|idx| &mut self.nodes[idx])
    }

    pub fn get_root(&self) -> Option<&T> {
        self.root.map(|idx| &self.nodes[idx])
    }

    pub fn get_left(&self) -> Option<&T> {
        self.get_root().and_then(|root| self.get_root_or_node(root.get_left_index()))
    }

    pub fn get_right(&self) -> Option<&T> {
        self.get_root().and_then(|root| self.get_root_or_node(root.get_right_index()))
    }

    pub fn get_node(&self, idx: usize) -> Option<&T> {
        if idx < self.nodes.len() {
            Some(&self.nodes[idx])
        } else {
            None
        }
    }

    pub fn get_root_or_node(&self, idx: Option<usize>) -> Option<&T> {
        match idx {
            Some(num) => self.get_node(num),
            None => self.get_root()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubNode {
        left: Option<usize>,
        mid: Option<usize>,
        right: Option<usize>,
    }

    impl IndexableNode for StubNode {
        fn is_leaf(&self) -> bool { self.left.is_none() && self.right.is_none() }
        fn get_left_index(&self) -> Option<usize> { self.left }
        fn get_mid_index(&self) -> Option<usize> { self.mid }
        fn get_right_index(&self) -> Option<usize> { self.right }
        fn set_leaves(&mut self, left: Option<usize>, mid: Option<usize>, right: Option<usize>) {
            self.left = left;
            self.mid = mid;
            self.right = right;
        }
        fn shift_by(self, offset: usize) -> Self {
            StubNode {
                left: self.left.map(|l| l + offset),
                mid: self.mid.map(|l| l + offset),
                right: self.right.map(|r| r + offset),
            }
        }
        fn make_glue() -> Self {
            StubNode { left: None, mid: None, right: None }
        }
    }

    fn leaf() -> StubNode { StubNode { left: None, mid: None, right: None } }
    fn branch(l: usize, r: usize) -> StubNode { StubNode { left: Some(l), mid: None, right: Some(r) } }

    #[test]
    fn new_root_is_first_node() {
        let tree = Tree::new(leaf());
        assert!(tree.get_root().unwrap().left.is_none());
    }

    #[test]
    fn append_returns_sequential_indices() {
        let mut tree = Tree::new(leaf());
        assert_eq!(tree.append(leaf(), false), 1);
        assert_eq!(tree.append(leaf(), false), 2);
    }

    #[test]
    fn append_without_new_root_keeps_original_root() {
        let mut tree = Tree::new(branch(0, 0));
        tree.append(leaf(), false);
        assert!(tree.get_root().unwrap().left.is_some()); // still the original branch node
    }

    #[test]
    fn append_with_new_root_updates_root() {
        let mut tree = Tree::new(leaf());          // idx 0
        let idx1 = tree.append(leaf(), false);     // idx 1
        tree.append(branch(0, idx1), true);        // idx 2, becomes root

        assert_eq!(tree.get_root().unwrap().left, Some(0));
        assert_eq!(tree.get_root().unwrap().right, Some(idx1));
    }

    #[test]
    fn get_node_returns_node_at_valid_index() {
        let mut tree = Tree::new(leaf());
        let idx = tree.append(branch(0, 0), false);
        assert_eq!(tree.get_node(idx).unwrap().left, Some(0));
    }

    #[test]
    fn get_node_returns_none_for_out_of_bounds() {
        let tree = Tree::new(leaf());
        assert!(tree.get_node(1).is_none());
        assert!(tree.get_node(999).is_none());
    }

    // --- new_root_with_right_idx / new_root_with_left_idx ---

    #[test]
    fn new_root_with_right_idx_places_old_root_as_left_and_idx_as_right() {
        let mut tree = Tree::new(leaf());              // idx 0, root=0
        let idx1 = tree.append(leaf(), false);         // idx 1
        let tree = tree.new_root_with_right_idx(branch(0, 0), idx1); // new root at idx 2
        assert_eq!(tree.get_root().unwrap().left, Some(0));     // old root
        assert_eq!(tree.get_root().unwrap().right, Some(idx1));
    }

    #[test]
    fn new_root_with_left_idx_places_idx_as_left_and_old_root_as_right() {
        let mut tree = Tree::new(leaf());              // idx 0, root=0
        let idx1 = tree.append(leaf(), false);         // idx 1
        let tree = tree.new_root_with_left_idx(branch(0, 0), idx1); // new root at idx 2
        assert_eq!(tree.get_root().unwrap().left, Some(idx1));
        assert_eq!(tree.get_root().unwrap().right, Some(0));    // old root
    }

    // --- concat ---

    #[test]
    fn concat_shifts_second_tree_indices_by_first_tree_length() {
        let tree1 = Tree::new(leaf());                      // [leaf], root=0, len=1
        let mut tree2 = Tree::new(leaf());                  // idx 0
        let t2_idx1 = tree2.append(leaf(), false);          // idx 1
        tree2.append(branch(0, t2_idx1), true);             // idx 2, root=2

        let (combined, new_root) = tree1.concat(tree2);
        // offset=1: branch indices become (0+1, 1+1)=(1,2); root becomes 2+1=3
        assert_eq!(new_root, Some(3));
        let root_node = combined.get_node(new_root.unwrap()).unwrap();
        assert_eq!(root_node.left, Some(1));
        assert_eq!(root_node.right, Some(2));
    }

    #[test]
    fn concat_preserves_first_tree_root() {
        let mut tree1 = Tree::new(branch(0, 0));  // idx 0, root=0
        tree1.append(leaf(), false);              // idx 1, root unchanged
        let tree2 = Tree::new(leaf());

        let (combined, _) = tree1.concat(tree2);
        assert!(combined.get_root().unwrap().left.is_some()); // still the original branch
    }

    #[test]
    fn concat_returns_correct_second_root_offset() {
        let tree1 = Tree::new(leaf()); // len=1
        let mut tree2 = Tree::new(leaf());
        tree2.append(leaf(), true); // root=1
        let (_, new_root) = tree1.concat(tree2);
        assert_eq!(new_root, Some(2)); // tree2.root(1) + offset(1)
    }

    // --- get_root_or_node ---

    #[test]
    fn get_root_or_node_none_returns_current_root() {
        let mut tree = Tree::new(leaf());
        tree.append(branch(0, 0), true); // root is now idx 1 (branch)
        let node = tree.get_root_or_node(None).unwrap();
        assert!(node.left.is_some()); // the branch node
    }

    #[test]
    fn get_root_or_node_some_returns_that_node() {
        let mut tree = Tree::new(branch(0, 0)); // root is branch
        let idx1 = tree.append(leaf(), false);
        let node = tree.get_root_or_node(Some(idx1)).unwrap();
        assert!(node.left.is_none()); // the leaf, not the root
    }

    #[test]
    fn get_root_or_node_invalid_index_returns_none() {
        let tree = Tree::new(leaf());
        assert!(tree.get_root_or_node(Some(999)).is_none());
    }
}

