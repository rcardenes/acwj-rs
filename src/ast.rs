use crate::{
    scan::Token,
    sym::PrimType,
};

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
    IntLit { val: i64, dtype: PrimType },
    Ident { id: Identifier, dtype: PrimType },
    LvIdent { id: Identifier, dtype: PrimType },

    // Binary operations
    //   -- arithmetic
    Add { left: Box<AstNode>, right: Box<AstNode>, dtype: PrimType },
    Subtract { left: Box<AstNode>, right: Box<AstNode>, dtype: PrimType },
    Multiply { left: Box<AstNode>, right: Box<AstNode>, dtype: PrimType },
    Divide { left: Box<AstNode>, right: Box<AstNode>, dtype: PrimType },
    //   -- comparison
    Equal { left: Box<AstNode>, right: Box<AstNode>, dtype: PrimType },
    NotEqual { left: Box<AstNode>, right: Box<AstNode>, dtype: PrimType },
    LessThan { left: Box<AstNode>, right: Box<AstNode>, dtype: PrimType },
    GreaterThan { left: Box<AstNode>, right: Box<AstNode>, dtype: PrimType },
    LessThanOrEqual { left: Box<AstNode>, right: Box<AstNode>, dtype: PrimType },
    GreaterThanOrEqual { left: Box<AstNode>, right: Box<AstNode>, dtype: PrimType },

    // Declarations
    Function { name: Box<AstNode>, body: Box<AstNode> },
    GlobalDec { id: Identifier, dtype: PrimType },

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

    pub fn get_type(&self) -> Option<PrimType> {
        match self {
            AstNode::IntLit { dtype, .. }
                | AstNode::Ident { dtype, .. }
                | AstNode::LvIdent { dtype, .. }
                | AstNode::Add { dtype, .. }
                | AstNode::Subtract { dtype, .. }
                | AstNode::Multiply { dtype, .. }
                | AstNode::Divide { dtype, .. }
                | AstNode::Equal { dtype, .. }
                | AstNode::NotEqual { dtype, .. }
                | AstNode::LessThan { dtype, .. }
                | AstNode::GreaterThan { dtype, .. }
                | AstNode::LessThanOrEqual { dtype, .. }
                | AstNode::GreaterThanOrEqual { dtype, .. }
            => Some(*dtype),
            _ => None,
        }
    }

    pub fn new_type(self, dtype: PrimType) -> Self {
        match self {
            AstNode::IntLit { val, .. } => AstNode::IntLit { val, dtype },
            AstNode::Ident { id, .. } => AstNode::Ident { id, dtype },
            AstNode::Add { left, right, .. } => AstNode::Add { left, right, dtype },
            AstNode::Subtract { left, right, .. } => AstNode::Subtract { left, right, dtype },
            AstNode::Multiply { left, right, .. } => AstNode::Multiply { left, right, dtype },
            AstNode::Divide { left, right, .. } => AstNode::Divide { left, right, dtype },
            AstNode::Equal { left, right, .. } => AstNode::Equal { left, right, dtype },
            AstNode::NotEqual { left, right, .. } => AstNode::NotEqual { left, right, dtype },
            AstNode::LessThan { left, right, .. } => AstNode::LessThan { left, right, dtype },
            AstNode::GreaterThan { left, right, .. } => AstNode::GreaterThan { left, right, dtype },
            AstNode::LessThanOrEqual { left, right, .. } => AstNode::LessThanOrEqual { left, right, dtype },
            AstNode::GreaterThanOrEqual { left, right, .. } => AstNode::GreaterThanOrEqual { left, right, dtype },
            _ => panic!("Can't change type for {:?}", self),
        }
    }

    pub fn make_binary(op: Token, l: AstNode, r: AstNode, dtype: PrimType) -> AstNode {
        match op {
            Token::Plus => AstNode::Add {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            Token::Minus => AstNode::Subtract {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            Token::Star => AstNode::Multiply {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            Token::Slash => AstNode::Divide {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            Token::EQ => AstNode::Equal {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            Token::NE => AstNode::NotEqual {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            Token::LT => AstNode::LessThan {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            Token::GT => AstNode::GreaterThan {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            Token::LE => AstNode::LessThanOrEqual {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            Token::GE => AstNode::GreaterThanOrEqual {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            _ => panic!("Wrong token: {op:?}")
        }
    }

    pub fn make_intlit(val: i64, dtype: PrimType) -> AstNode {
        AstNode::IntLit { val, dtype }
    }

    pub fn make_ident(name: &str, dtype: PrimType) -> AstNode {
        AstNode::Ident { id: Identifier::new(name), dtype }
    }

    pub fn make_lvident(name: &str, dtype: PrimType) -> AstNode {
        AstNode::LvIdent { id: Identifier::new(name), dtype }
    }

    pub fn make_function(name: AstNode, body: AstNode) -> AstNode {
        if !matches!(name, AstNode::Ident{..} ) {
            panic!("Trying to make an assignment node without an ident as name")
        }

        AstNode::Function {
            name: Box::new(name),
            body: Box::new(body),
        }
    }

    pub fn make_global_declaration(name: &str, dtype: PrimType) -> AstNode {
        AstNode::GlobalDec { id: Identifier::new(name), dtype }
    }

    pub fn make_assign(id: AstNode, expr: AstNode) -> AstNode {
        if !matches!(id, AstNode::LvIdent {..}) {
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
    use crate::sym::PrimType;


    // --- Ast predicate methods ---

    #[test]
    fn is_arith_true_for_arithmetic_ops() {
        assert!(AstNode::make_binary(Token::Plus, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int).is_arith());
        assert!(AstNode::make_binary(Token::Minus, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int).is_arith());
        assert!(AstNode::make_binary(Token::Star, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int).is_arith());
        assert!(AstNode::make_binary(Token::Slash, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int).is_arith());
    }

    #[test]
    fn is_arith_false_for_non_arithmetic() {
        assert!(!AstNode::make_binary(Token::EQ, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_arith());
        assert!(!AstNode::make_if(AstNode::Empty, AstNode::Empty, None).is_arith());
        assert!(!AstNode::make_intlit(1, PrimType::Int).is_arith());
        assert!(!AstNode::make_while(AstNode::Empty, AstNode::Empty).is_arith());
    }

    #[test]
    fn is_comparison_true_for_comparisons() {
        assert!(AstNode::make_binary(Token::EQ, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_comparison());
        assert!(AstNode::make_binary(Token::NE, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_comparison());
        assert!(AstNode::make_binary(Token::LT, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_comparison());
        assert!(AstNode::make_binary(Token::GT, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_comparison());
        assert!(AstNode::make_binary(Token::LE, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_comparison());
        assert!(AstNode::make_binary(Token::GE, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_comparison());
    }

    #[test]
    fn is_comparison_false_for_non_comparisons() {
        assert!(!AstNode::make_binary(Token::Plus, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_comparison());
        assert!(!AstNode::make_while(AstNode::Empty, AstNode::Empty).is_comparison());
        assert!(!AstNode::make_intlit(0, PrimType::Int).is_comparison());
    }

    #[test]
    fn is_branching_stmt_true_for_if_and_while() {
        assert!(AstNode::make_if(AstNode::Empty, AstNode::Empty, None).is_branching_stmt());
        assert!(AstNode::make_while(AstNode::Empty, AstNode::Empty).is_branching_stmt());
    }

    #[test]
    fn is_branching_stmt_false_for_others() {
        assert!(!AstNode::make_binary(Token::Plus, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_branching_stmt());
        assert!(!AstNode::make_binary(Token::EQ, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_branching_stmt());
        assert!(!AstNode::make_intlit(5, PrimType::Int).is_branching_stmt());
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
        let node = AstNode::make_ident("x", PrimType::Int);
        assert!(matches!(node, AstNode::Ident { id: Identifier { name }, dtype: PrimType::Int } if name == "x"));
    }

    #[test]
    fn make_lvident_creates_lvident_variant() {
        let node = AstNode::make_lvident("y", PrimType::Int);
        assert!(matches!(node, AstNode::LvIdent { id: Identifier { name }, dtype: PrimType::Int } if name == "y"));
    }

    // --- make_binary: all token mappings ---

    #[test]
    fn make_binary_plus_creates_add() {
        let node = AstNode::make_binary(Token::Plus, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::Add { left, right, dtype: PrimType::Int }
            if *left == AstNode::make_intlit(1, PrimType::Int) && *right == AstNode::make_intlit(2, PrimType::Int)));
    }

    #[test]
    fn make_binary_minus_creates_subtract() {
        let node = AstNode::make_binary(Token::Minus, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::Subtract { left, right, dtype: PrimType::Int }
            if *left == AstNode::make_intlit(1, PrimType::Int) && *right == AstNode::make_intlit(2, PrimType::Int)));
    }

    #[test]
    fn make_binary_star_creates_multiply() {
        let node = AstNode::make_binary(Token::Star, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::Multiply { left, right, dtype: PrimType::Int }
            if *left == AstNode::make_intlit(1, PrimType::Int) && *right == AstNode::make_intlit(2, PrimType::Int)));
    }

    #[test]
    fn make_binary_slash_creates_divide() {
        let node = AstNode::make_binary(Token::Slash, AstNode::make_intlit(8, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::Divide { left, right, dtype: PrimType::Int }
            if *left == AstNode::make_intlit(8, PrimType::Int) && *right == AstNode::make_intlit(2, PrimType::Int)));
    }

    #[test]
    fn make_binary_eq_creates_equal() {
        let node = AstNode::make_binary(Token::EQ, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::Equal { .. }));
    }

    #[test]
    fn make_binary_ne_creates_notequal() {
        let node = AstNode::make_binary(Token::NE, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::NotEqual { .. }));
    }

    #[test]
    fn make_binary_lt_creates_lessthan() {
        let node = AstNode::make_binary(Token::LT, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::LessThan { .. }));
    }

    #[test]
    fn make_binary_gt_creates_greaterthan() {
        let node = AstNode::make_binary(Token::GT, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::GreaterThan { .. }));
    }

    #[test]
    fn make_binary_le_creates_lessorequal() {
        let node = AstNode::make_binary(Token::LE, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::LessThanOrEqual { .. }));
    }

    #[test]
    fn make_binary_ge_creates_greaterorequal() {
        let node = AstNode::make_binary(Token::GE, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::GreaterThanOrEqual { .. }));
    }

    #[test]
    #[should_panic(expected = "Wrong token")]
    fn make_binary_unknown_token_panics() {
        AstNode::make_binary(Token::Semi, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int);
    }

    // --- Factory methods: declarations ---

    #[test]
    fn make_global_declaration_creates_globaldec() {
        let d = PrimType::Void;
        let node = AstNode::make_global_declaration("g", d);
        assert!(matches!(&node, AstNode::GlobalDec { id, dtype: d } if id.name == "g"));
    }

    #[test]
    fn make_function_creates_function_node() {
        let name = AstNode::make_ident("main", PrimType::Void);
        let body = AstNode::make_glue(AstNode::Empty, AstNode::Empty);
        let node = AstNode::make_function(name, body);
        assert!(matches!(&node, AstNode::Function { .. }));
    }

    #[test]
    #[should_panic(expected = "without an ident")]
    fn make_function_with_non_ident_panics() {
        AstNode::make_function(AstNode::make_intlit(42, PrimType::Int), AstNode::Empty);
    }

    // --- Factory methods: statements ---

    #[test]
    fn make_assign_creates_assign_node() {
        let id = AstNode::make_lvident("x", PrimType::Int);
        let expr = AstNode::make_intlit(10, PrimType::Int);
        let node = AstNode::make_assign(id, expr);
        assert!(matches!(&node, AstNode::Assign { .. }));
    }

    #[test]
    #[should_panic(expected = "without an lvident")]
    fn make_assign_with_non_lvident_panics() {
        AstNode::make_assign(AstNode::make_ident("x", PrimType::Int),
                           AstNode::make_intlit(1, PrimType::Int));
    }

    #[test]
    fn make_if_without_else_branch() {
        let node = AstNode::make_if(AstNode::make_intlit(1, PrimType::Int), AstNode::Empty, None);
        assert!(matches!(&node, AstNode::If { branch_f: None, .. }));
    }

    #[test]
    fn make_if_with_else_branch() {
        let node = AstNode::make_if(AstNode::make_intlit(1, PrimType::Int), AstNode::Empty, Some(AstNode::Empty));
        assert!(matches!(&node, AstNode::If { branch_f: Some(..), .. }));
    }

    #[test]
    fn make_while_creates_while_node() {
        let node = AstNode::make_while(AstNode::make_intlit(1, PrimType::Int), AstNode::Empty);
        assert!(matches!(&node, AstNode::While { .. }));
    }

    #[test]
    fn make_print_creates_print_node() {
        let node = AstNode::make_print(AstNode::make_intlit(42, PrimType::Int));
        assert!(matches!(&node, AstNode::Print { .. }));
    }

    #[test]
    fn make_glue_creates_glue_node() {
        let node = AstNode::make_glue(AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int));
        assert!(matches!(node, AstNode::Glue { left, right }
            if left == Box::new(AstNode::make_intlit(1, PrimType::Int)) && right == Box::new(AstNode::make_intlit(2, PrimType::Int))));
    }

    // --- Structural ---

    #[test]
    fn ast_node_debug_contains_variant_name() {
        let s = format!("{:?}", AstNode::make_intlit(7, PrimType::Int));
        assert!(s.contains("IntLit"));
    }

    #[test]
    fn ast_node_partial_eq() {
        assert_eq!(AstNode::make_intlit(3, PrimType::Int), AstNode::make_intlit(3, PrimType::Int));
        assert_ne!(AstNode::make_intlit(3, PrimType::Int), AstNode::make_intlit(4, PrimType::Int));
    }
}
