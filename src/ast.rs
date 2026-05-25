use crate::{
    scan::TokenType,
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

    // Unary operations
    FuncCall { id: Identifier, dtype: PrimType, left: Box<AstNode> },
    Addr { id: Identifier, dtype: PrimType },
    Deref { inner: Box<AstNode>, dtype: PrimType },

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
    Assign { id: Box<AstNode>, expr: Box<AstNode> },
    Glue { left: Box<AstNode>, right: Box<AstNode> },
    Empty, // Empty statement, meant to ensure that we can describe empty compound statements
    If { cond: Box<AstNode>, branch_t: Box<AstNode>, branch_f: Option<Box<AstNode>> },
    Return { id: Identifier, expr: Box<AstNode> },
    While { cond: Box<AstNode>, body: Box<AstNode> },
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
            | AstNode::FuncCall { dtype, .. }
            | AstNode::Addr { dtype, .. }
            | AstNode::Deref { dtype, .. }
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

    pub fn make_binary(op: TokenType, l: AstNode, r: AstNode, dtype: PrimType) -> AstNode {
        match op {
            TokenType::Plus => AstNode::Add {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            TokenType::Minus => AstNode::Subtract {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            TokenType::Star => AstNode::Multiply {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            TokenType::Slash => AstNode::Divide {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            TokenType::EQ => AstNode::Equal {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            TokenType::NE => AstNode::NotEqual {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            TokenType::LT => AstNode::LessThan {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            TokenType::GT => AstNode::GreaterThan {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            TokenType::LE => AstNode::LessThanOrEqual {
                left: Box::new(l),
                right: Box::new(r),
                dtype,
            },
            TokenType::GE => AstNode::GreaterThanOrEqual {
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

    pub fn make_function_call(name: &str, expr: AstNode, dtype: PrimType) -> AstNode {
        AstNode::FuncCall { id: Identifier::new(name), dtype, left: Box::new(expr) }
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

    pub fn make_glue(left: AstNode, right: AstNode) -> AstNode {
        AstNode::Glue {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    pub fn make_return(name: &str, expr: AstNode) -> AstNode {
        AstNode::Return { id: Identifier::new(name), expr: Box::new(expr), }
    }

    pub fn into_pointer(self, errmsg: &str) -> AstNode {
        match self {
            AstNode::Ident { id, dtype } => {
                AstNode::Addr { id, dtype: dtype.pointer_to() }
            },
            _ => panic!("{}", errmsg)
        }
    }

    pub fn make_deref(self, errmsg: &str) -> AstNode {
        match self {
            AstNode::Ident { dtype, .. } |
            AstNode::Deref { dtype, .. } => {
                AstNode::Deref { inner: Box::new(self), dtype: dtype.value_at() }
            },
            _ => panic!("{}", errmsg)
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
        assert!(AstNode::make_binary(TokenType::Plus, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int).is_arith());
        assert!(AstNode::make_binary(TokenType::Minus, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int).is_arith());
        assert!(AstNode::make_binary(TokenType::Star, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int).is_arith());
        assert!(AstNode::make_binary(TokenType::Slash, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int).is_arith());
    }

    #[test]
    fn is_arith_false_for_non_arithmetic() {
        assert!(!AstNode::make_binary(TokenType::EQ, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_arith());
        assert!(!AstNode::make_if(AstNode::Empty, AstNode::Empty, None).is_arith());
        assert!(!AstNode::make_intlit(1, PrimType::Int).is_arith());
        assert!(!AstNode::make_while(AstNode::Empty, AstNode::Empty).is_arith());
    }

    #[test]
    fn is_comparison_true_for_comparisons() {
        assert!(AstNode::make_binary(TokenType::EQ, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_comparison());
        assert!(AstNode::make_binary(TokenType::NE, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_comparison());
        assert!(AstNode::make_binary(TokenType::LT, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_comparison());
        assert!(AstNode::make_binary(TokenType::GT, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_comparison());
        assert!(AstNode::make_binary(TokenType::LE, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_comparison());
        assert!(AstNode::make_binary(TokenType::GE, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_comparison());
    }

    #[test]
    fn is_comparison_false_for_non_comparisons() {
        assert!(!AstNode::make_binary(TokenType::Plus, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_comparison());
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
        assert!(!AstNode::make_binary(TokenType::Plus, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_branching_stmt());
        assert!(!AstNode::make_binary(TokenType::EQ, AstNode::make_intlit(5, PrimType::Int), AstNode::make_intlit(5, PrimType::Int), PrimType::Int).is_branching_stmt());
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
        let node = AstNode::make_binary(TokenType::Plus, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::Add { left, right, dtype: PrimType::Int }
            if *left == AstNode::make_intlit(1, PrimType::Int) && *right == AstNode::make_intlit(2, PrimType::Int)));
    }

    #[test]
    fn make_binary_minus_creates_subtract() {
        let node = AstNode::make_binary(TokenType::Minus, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::Subtract { left, right, dtype: PrimType::Int }
            if *left == AstNode::make_intlit(1, PrimType::Int) && *right == AstNode::make_intlit(2, PrimType::Int)));
    }

    #[test]
    fn make_binary_star_creates_multiply() {
        let node = AstNode::make_binary(TokenType::Star, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::Multiply { left, right, dtype: PrimType::Int }
            if *left == AstNode::make_intlit(1, PrimType::Int) && *right == AstNode::make_intlit(2, PrimType::Int)));
    }

    #[test]
    fn make_binary_slash_creates_divide() {
        let node = AstNode::make_binary(TokenType::Slash, AstNode::make_intlit(8, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::Divide { left, right, dtype: PrimType::Int }
            if *left == AstNode::make_intlit(8, PrimType::Int) && *right == AstNode::make_intlit(2, PrimType::Int)));
    }

    #[test]
    fn make_binary_eq_creates_equal() {
        let node = AstNode::make_binary(TokenType::EQ, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::Equal { .. }));
    }

    #[test]
    fn make_binary_ne_creates_notequal() {
        let node = AstNode::make_binary(TokenType::NE, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::NotEqual { .. }));
    }

    #[test]
    fn make_binary_lt_creates_lessthan() {
        let node = AstNode::make_binary(TokenType::LT, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::LessThan { .. }));
    }

    #[test]
    fn make_binary_gt_creates_greaterthan() {
        let node = AstNode::make_binary(TokenType::GT, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::GreaterThan { .. }));
    }

    #[test]
    fn make_binary_le_creates_lessorequal() {
        let node = AstNode::make_binary(TokenType::LE, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::LessThanOrEqual { .. }));
    }

    #[test]
    fn make_binary_ge_creates_greaterorequal() {
        let node = AstNode::make_binary(TokenType::GE, AstNode::make_intlit(1, PrimType::Int), AstNode::make_intlit(2, PrimType::Int), PrimType::Int);
        assert!(matches!(node, AstNode::GreaterThanOrEqual { .. }));
    }

    #[test]
    #[should_panic(expected = "Wrong token")]
    fn make_binary_unknown_token_panics() {
        AstNode::make_binary(TokenType::Semi, AstNode::make_intlit(0, PrimType::Int), AstNode::make_intlit(0, PrimType::Int), PrimType::Int);
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

    // --- make_function_call ---

    #[test]
    fn make_function_call_creates_funccall() {
        let node = AstNode::make_function_call("foo", AstNode::make_intlit(42, PrimType::Int), PrimType::Void);
        assert!(matches!(&node, AstNode::FuncCall { id, dtype: PrimType::Void, .. } if id.name == "foo"));
    }

    // --- make_return ---

    #[test]
    fn make_return_creates_return_node() {
        let node = AstNode::make_return("foo", AstNode::make_intlit(7, PrimType::Int));
        assert!(matches!(&node, AstNode::Return { id, expr } if id.name == "foo"));
    }

    // --- into_pointer ---

    #[test]
    fn into_pointer_ident_creates_addr() {
        let id = AstNode::make_ident("x", PrimType::Int);
        let addr = id.into_pointer("err");
        assert!(matches!(&addr, AstNode::Addr { id: Identifier { name }, dtype: PrimType::IntPtr } if name == "x"));
    }

    #[test]
    #[should_panic(expected = "err")]
    fn into_pointer_non_ident_panics() {
        AstNode::make_intlit(1, PrimType::Int).into_pointer("err");
    }

    // --- make_deref ---

    #[test]
    fn make_deref_ident_creates_deref() {
        let id = AstNode::make_ident("p", PrimType::IntPtr);
        let deref = id.make_deref("err");
        assert!(matches!(&deref, AstNode::Deref { dtype: PrimType::Int, .. }));
    }

    #[test]
    #[should_panic(expected = "Unrecognized in value_at")]
    fn make_deref_on_non_pointer_deref_panics() {
        let id = AstNode::make_ident("p", PrimType::IntPtr);
        let deref = id.make_deref("err"); // Deref with dtype=Int
        deref.make_deref("err"); // Int.value_at() panics
    }

    #[test]
    #[should_panic(expected = "err")]
    fn make_deref_non_ident_panics() {
        AstNode::make_intlit(1, PrimType::Int).make_deref("err");
    }

    // --- new_type ---

    #[test]
    fn new_type_intlit_changes_type() {
        let node = AstNode::make_intlit(42, PrimType::Char).new_type(PrimType::Long);
        assert_eq!(node.get_type(), Some(PrimType::Long));
    }

    #[test]
    fn new_type_ident_changes_type() {
        let node = AstNode::make_ident("x", PrimType::Char).new_type(PrimType::Int);
        assert_eq!(node.get_type(), Some(PrimType::Int));
    }

    #[test]
    fn new_type_binary_changes_type() {
        let node = AstNode::make_binary(TokenType::Plus, AstNode::make_intlit(1, PrimType::Char), AstNode::make_intlit(2, PrimType::Int), PrimType::Char).new_type(PrimType::Int);
        assert_eq!(node.get_type(), Some(PrimType::Int));
    }

    #[test]
    #[should_panic(expected = "Can't change type")]
    fn new_type_panics_on_empty() {
        AstNode::Empty.new_type(PrimType::Int);
    }

    // --- get_type for variants not explicitly tested ---

    #[test]
    fn get_type_addr_returns_type() {
        let id = AstNode::make_ident("x", PrimType::Int);
        let addr = id.into_pointer("err");
        assert_eq!(addr.get_type(), Some(PrimType::IntPtr));
    }

    #[test]
    fn get_type_deref_returns_type() {
        let id = AstNode::make_ident("p", PrimType::IntPtr);
        let deref = id.make_deref("err");
        assert_eq!(deref.get_type(), Some(PrimType::Int));
    }

    #[test]
    fn get_type_funccall_returns_type() {
        let node = AstNode::make_function_call("foo", AstNode::make_intlit(1, PrimType::Int), PrimType::Void);
        assert_eq!(node.get_type(), Some(PrimType::Void));
    }

    #[test]
    fn get_type_empty_returns_none() {
        assert_eq!(AstNode::Empty.get_type(), None);
    }

    #[test]
    fn get_type_function_returns_none() {
        assert_eq!(AstNode::make_function(AstNode::make_ident("f", PrimType::Void), AstNode::Empty).get_type(), None);
    }

    #[test]
    fn get_type_assign_returns_none() {
        assert_eq!(AstNode::make_assign(AstNode::make_lvident("x", PrimType::Int), AstNode::make_intlit(5, PrimType::Int)).get_type(), None);
    }

    #[test]
    fn get_type_if_returns_none() {
        assert_eq!(AstNode::make_if(AstNode::make_intlit(1, PrimType::Int), AstNode::Empty, None).get_type(), None);
    }

    #[test]
    fn get_type_while_returns_none() {
        assert_eq!(AstNode::make_while(AstNode::make_intlit(1, PrimType::Int), AstNode::Empty).get_type(), None);
    }

    #[test]
    fn get_type_glue_returns_none() {
        assert_eq!(AstNode::make_glue(AstNode::Empty, AstNode::Empty).get_type(), None);
    }

    #[test]
    fn get_type_return_returns_none() {
        assert_eq!(AstNode::make_return("f", AstNode::make_intlit(1, PrimType::Int)).get_type(), None);
    }

    #[test]
    fn get_type_globaldec_returns_none() {
        assert_eq!(AstNode::make_global_declaration("x", PrimType::Int).get_type(), None);
    }
}
