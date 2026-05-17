use crate::scan::Token;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identifier {
    pub name: String,
}

impl Identifier {
    pub fn new(name: &str) -> Self {
        Identifier {
            name: name.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AstNode {
    // Literals and identifiers
    IntLit(i64),
    Ident(Identifier),
    LvIdent(Identifier),

    // Binary operations
    //   -- arithmetic
    Add { left: Box<AstNode>, right: Box<AstNode> },
    Subtract { left: Box<AstNode>, right: Box<AstNode> },
    Multiply { left: Box<AstNode>, right: Box<AstNode> },
    Divide { left: Box<AstNode>, right: Box<AstNode> },
    //   -- comparison
    Equal { left: Box<AstNode>, right: Box<AstNode> },
    NotEqual { left: Box<AstNode>, right: Box<AstNode> },
    LessThan { left: Box<AstNode>, right: Box<AstNode> },
    GreaterThan { left: Box<AstNode>, right: Box<AstNode> },
    LessThanOrEqual { left: Box<AstNode>, right: Box<AstNode> },
    GreaterThanOrEqual { left: Box<AstNode>, right: Box<AstNode> },

    // Declarations
    Function { name: Box<AstNode>, body: Box<AstNode> },
    GlobalDec { id: Identifier },

    // Statements
    Empty, // Empty statement, meant to ensure that we can describe empty compound statements
    Assign { id: Box<AstNode>, expr: Box<AstNode> },
    Glue { left: Box<AstNode>, right: Box<AstNode> },
    If { cond: Box<AstNode>, branch_t: Box<AstNode>, branch_f: Option<Box<AstNode>> },
    While { cond: Box<AstNode>, body: Box<AstNode> },
    Print { expr: Box<AstNode> },
}

impl AstNode {
    pub fn is_arith(&self) -> bool {
        matches!(self, AstNode::Add {..}
                     | AstNode::Subtract {..}
                     | AstNode::Multiply {..}
                     | AstNode::Divide {..})
    }

    pub fn is_comparison(&self) -> bool {
        matches!(self, AstNode::Equal {..}
                     | AstNode::NotEqual {..}
                     | AstNode::LessThan {..}
                     | AstNode::GreaterThan {..}
                     | AstNode::LessThanOrEqual {..}
                     | AstNode::GreaterThanOrEqual {..})
    }

    pub fn is_branching_stmt(&self) -> bool {
        matches!(self, AstNode::While {..}
                     | AstNode::If {..})
    }

    pub fn make_binary(op: Token, l: AstNode, r: AstNode) -> AstNode {
        match op {
            Token::Plus => AstNode::Add {
                left: Box::new(l),
                right: Box::new(r),
            },
            Token::Minus => AstNode::Subtract {
                left: Box::new(l),
                right: Box::new(r),
            },
            Token::Star => AstNode::Multiply {
                left: Box::new(l),
                right: Box::new(r),
            },
            Token::Slash => AstNode::Divide {
                left: Box::new(l),
                right: Box::new(r),
            },
            Token::EQ => AstNode::Equal {
                left: Box::new(l),
                right: Box::new(r),
            },
            Token::NE => AstNode::NotEqual {
                left: Box::new(l),
                right: Box::new(r),
            },
            Token::LT => AstNode::LessThan {
                left: Box::new(l),
                right: Box::new(r),
            },
            Token::GT => AstNode::GreaterThan {
                left: Box::new(l),
                right: Box::new(r),
            },
            Token::LE => AstNode::LessThanOrEqual {
                left: Box::new(l),
                right: Box::new(r),
            },
            Token::GE => AstNode::GreaterThanOrEqual {
                left: Box::new(l),
                right: Box::new(r),
            },
            _ => panic!("Wrong token: {op:?}")
        }
    }

    pub fn make_ident(name: &str) -> AstNode {
        AstNode::Ident(Identifier::new(name))
    }

    pub fn make_lvident(name: &str) -> AstNode {
        AstNode::LvIdent(Identifier::new(name))
    }

    pub fn make_function(name: AstNode, body: AstNode) -> AstNode {
        if !matches!(name, AstNode::Ident(_)) {
            panic!("Trying to make an assignment node without an ident as name")
        }

        AstNode::Function {
            name: Box::new(name),
            body: Box::new(body),
        }
    }

    pub fn make_global_declaration(name: &str) -> AstNode {
        AstNode::GlobalDec { id: Identifier::new(name) }
    }

    pub fn make_assign(id: AstNode, expr: AstNode) -> AstNode {
        if !matches!(id, AstNode::LvIdent(_)) {
            panic!("Trying to make an assignment node without an lvident on the left")
        }
        AstNode::Assign {
            id: Box::new(id),
            expr: Box::new(expr),
        }
    }

    pub fn make_if(cond: AstNode, branch_t: AstNode, branch_f: Option<AstNode>) -> AstNode {
        AstNode::If {
            cond: Box::new(cond),
            branch_t: Box::new(branch_t),
            branch_f: branch_f.map(Box::new),
        }
    }

    pub fn make_while(cond: AstNode, body: AstNode) -> AstNode {
        AstNode::While {
            cond: Box::new(cond),
            body: Box::new(body),
        }
    }

    pub fn make_print(expr: AstNode) -> AstNode {
        AstNode::Print {
            expr: Box::new(expr),
        }
    }

    pub fn make_glue(left: AstNode, right: AstNode) -> AstNode {
        AstNode::Glue {
            left: Box::new(left),
            right: Box::new(right),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    // --- Ast predicate methods ---

    #[test]
    fn is_arith_true_for_arithmetic_ops() {
        assert!(AstNode::make_binary(Token::Plus, AstNode::IntLit(5), AstNode::IntLit(2)).is_arith());
        assert!(AstNode::make_binary(Token::Minus, AstNode::IntLit(5), AstNode::IntLit(2)).is_arith());
        assert!(AstNode::make_binary(Token::Star, AstNode::IntLit(5), AstNode::IntLit(2)).is_arith());
        assert!(AstNode::make_binary(Token::Slash, AstNode::IntLit(5), AstNode::IntLit(2)).is_arith());
    }

    #[test]
    fn is_arith_false_for_non_arithmetic() {
        assert!(!AstNode::make_binary(Token::EQ, AstNode::IntLit(5), AstNode::IntLit(5)).is_arith());
        assert!(!AstNode::make_if(AstNode::Empty, AstNode::Empty, None).is_arith());
        assert!(!AstNode::IntLit(1).is_arith());
        assert!(!AstNode::make_while(AstNode::Empty, AstNode::Empty).is_arith());
    }

    #[test]
    fn is_comparison_true_for_comparisons() {
        assert!(AstNode::make_binary(Token::EQ, AstNode::IntLit(5), AstNode::IntLit(5)).is_comparison());
        assert!(AstNode::make_binary(Token::NE, AstNode::IntLit(5), AstNode::IntLit(5)).is_comparison());
        assert!(AstNode::make_binary(Token::LT, AstNode::IntLit(5), AstNode::IntLit(5)).is_comparison());
        assert!(AstNode::make_binary(Token::GT, AstNode::IntLit(5), AstNode::IntLit(5)).is_comparison());
        assert!(AstNode::make_binary(Token::LE, AstNode::IntLit(5), AstNode::IntLit(5)).is_comparison());
        assert!(AstNode::make_binary(Token::GE, AstNode::IntLit(5), AstNode::IntLit(5)).is_comparison());
    }

    #[test]
    fn is_comparison_false_for_non_comparisons() {
        assert!(!AstNode::make_binary(Token::Plus, AstNode::IntLit(5), AstNode::IntLit(5)).is_comparison());
        assert!(!AstNode::make_while(AstNode::Empty, AstNode::Empty).is_comparison());
        assert!(!AstNode::IntLit(0).is_comparison());
    }

    #[test]
    fn is_branching_stmt_true_for_if_and_while() {
        assert!(AstNode::make_if(AstNode::Empty, AstNode::Empty, None).is_branching_stmt());
        assert!(AstNode::make_while(AstNode::Empty, AstNode::Empty).is_branching_stmt());
    }

    #[test]
    fn is_branching_stmt_false_for_others() {
        assert!(!AstNode::make_binary(Token::Plus, AstNode::IntLit(5), AstNode::IntLit(5)).is_branching_stmt());
        assert!(!AstNode::make_binary(Token::EQ, AstNode::IntLit(5), AstNode::IntLit(5)).is_branching_stmt());
        assert!(!AstNode::IntLit(5).is_branching_stmt());
    }

    // --- Identifier construction ---

    #[test]
    fn identifier_new_stores_name() {
        let id = Identifier::new("foo");
        assert_eq!(id.name, "foo");
    }

    // --- Factory methods: literals and identifiers ---

    #[test]
    fn make_ident_creates_ident_variant() {
        let node = AstNode::make_ident("x");
        assert!(matches!(node, AstNode::Ident(Identifier { name }) if name == "x"));
    }

    #[test]
    fn make_lvident_creates_lvident_variant() {
        let node = AstNode::make_lvident("y");
        assert!(matches!(node, AstNode::LvIdent(Identifier { name }) if name == "y"));
    }

    // --- make_binary: all token mappings ---

    #[test]
    fn make_binary_plus_creates_add() {
        let node = AstNode::make_binary(Token::Plus, AstNode::IntLit(1), AstNode::IntLit(2));
        assert!(matches!(node, AstNode::Add { left, right }
            if *left == AstNode::IntLit(1) && *right == AstNode::IntLit(2)));
    }

    #[test]
    fn make_binary_minus_creates_subtract() {
        let node = AstNode::make_binary(Token::Minus, AstNode::IntLit(1), AstNode::IntLit(2));
        assert!(matches!(node, AstNode::Subtract { left, right }
            if *left == AstNode::IntLit(1) && *right == AstNode::IntLit(2)));
    }

    #[test]
    fn make_binary_star_creates_multiply() {
        let node = AstNode::make_binary(Token::Star, AstNode::IntLit(1), AstNode::IntLit(2));
        assert!(matches!(node, AstNode::Multiply { left, right }
            if *left == AstNode::IntLit(1) && *right == AstNode::IntLit(2)));
    }

    #[test]
    fn make_binary_slash_creates_divide() {
        let node = AstNode::make_binary(Token::Slash, AstNode::IntLit(8), AstNode::IntLit(2));
        assert!(matches!(node, AstNode::Divide { left, right }
            if *left == AstNode::IntLit(8) && *right == AstNode::IntLit(2)));
    }

    #[test]
    fn make_binary_eq_creates_equal() {
        let node = AstNode::make_binary(Token::EQ, AstNode::IntLit(1), AstNode::IntLit(2));
        assert!(matches!(node, AstNode::Equal { .. }));
    }

    #[test]
    fn make_binary_ne_creates_notequal() {
        let node = AstNode::make_binary(Token::NE, AstNode::IntLit(1), AstNode::IntLit(2));
        assert!(matches!(node, AstNode::NotEqual { .. }));
    }

    #[test]
    fn make_binary_lt_creates_lessthan() {
        let node = AstNode::make_binary(Token::LT, AstNode::IntLit(1), AstNode::IntLit(2));
        assert!(matches!(node, AstNode::LessThan { .. }));
    }

    #[test]
    fn make_binary_gt_creates_greaterthan() {
        let node = AstNode::make_binary(Token::GT, AstNode::IntLit(1), AstNode::IntLit(2));
        assert!(matches!(node, AstNode::GreaterThan { .. }));
    }

    #[test]
    fn make_binary_le_creates_lessorequal() {
        let node = AstNode::make_binary(Token::LE, AstNode::IntLit(1), AstNode::IntLit(2));
        assert!(matches!(node, AstNode::LessThanOrEqual { .. }));
    }

    #[test]
    fn make_binary_ge_creates_greaterorequal() {
        let node = AstNode::make_binary(Token::GE, AstNode::IntLit(1), AstNode::IntLit(2));
        assert!(matches!(node, AstNode::GreaterThanOrEqual { .. }));
    }

    #[test]
    #[should_panic(expected = "Wrong token")]
    fn make_binary_unknown_token_panics() {
        AstNode::make_binary(Token::Semi, AstNode::IntLit(0), AstNode::IntLit(0));
    }

    // --- Factory methods: declarations ---

    #[test]
    fn make_global_declaration_creates_globaldec() {
        let node = AstNode::make_global_declaration("g");
        assert!(matches!(&node, AstNode::GlobalDec { id } if id.name == "g"));
    }

    #[test]
    fn make_function_creates_function_node() {
        let name = AstNode::make_ident("main");
        let body = AstNode::make_glue(AstNode::Empty, AstNode::Empty);
        let node = AstNode::make_function(name, body);
        assert!(matches!(&node, AstNode::Function { .. }));
    }

    #[test]
    #[should_panic(expected = "without an ident")]
    fn make_function_with_non_ident_panics() {
        AstNode::make_function(AstNode::IntLit(42), AstNode::Empty);
    }

    // --- Factory methods: statements ---

    #[test]
    fn make_assign_creates_assign_node() {
        let id = AstNode::make_lvident("x");
        let expr = AstNode::IntLit(10);
        let node = AstNode::make_assign(id, expr);
        assert!(matches!(&node, AstNode::Assign { .. }));
    }

    #[test]
    #[should_panic(expected = "without an lvident")]
    fn make_assign_with_non_lvident_panics() {
        AstNode::make_assign(AstNode::make_ident("x"), AstNode::IntLit(1));
    }

    #[test]
    fn make_if_without_else_branch() {
        let node = AstNode::make_if(AstNode::IntLit(1), AstNode::Empty, None);
        assert!(matches!(&node, AstNode::If { branch_f: None, .. }));
    }

    #[test]
    fn make_if_with_else_branch() {
        let node = AstNode::make_if(AstNode::IntLit(1), AstNode::Empty, Some(AstNode::Empty));
        assert!(matches!(&node, AstNode::If { branch_f: Some(..), .. }));
    }

    #[test]
    fn make_while_creates_while_node() {
        let node = AstNode::make_while(AstNode::IntLit(1), AstNode::Empty);
        assert!(matches!(&node, AstNode::While { .. }));
    }

    #[test]
    fn make_print_creates_print_node() {
        let node = AstNode::make_print(AstNode::IntLit(42));
        assert!(matches!(&node, AstNode::Print { .. }));
    }

    #[test]
    fn make_glue_creates_glue_node() {
        let node = AstNode::make_glue(AstNode::IntLit(1), AstNode::IntLit(2));
        assert!(matches!(node, AstNode::Glue { left, right }
            if left == Box::new(AstNode::IntLit(1)) && right == Box::new(AstNode::IntLit(2))));
    }

    // --- Structural ---

    #[test]
    fn ast_node_debug_contains_variant_name() {
        let s = format!("{:?}", AstNode::IntLit(7));
        assert!(s.contains("IntLit"));
    }

    #[test]
    fn ast_node_partial_eq() {
        assert_eq!(AstNode::IntLit(3), AstNode::IntLit(3));
        assert_ne!(AstNode::IntLit(3), AstNode::IntLit(4));
    }
}
