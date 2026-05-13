use anyhow::{Result, bail};
use crate::{
    ast::{Ast, AstNode},
    expr::binexpr,
    scan::{Scanner, Token},
    sym::SymbolTable,
    tree::Tree,
};

type ParseTree = Tree<AstNode>;

pub struct Parser<'a, T>
    where T: std::io::Read,

{
    scanner: &'a Scanner<T>,
    symbols: &'a mut SymbolTable,
}

impl<'a, T> Parser<'a, T>
where T: std::io::Read,
{
    pub fn new(scanner: &'a Scanner<T>, symbols: &'a mut SymbolTable) -> Self {
        Parser {
            scanner,
            symbols,
        }
    }

    fn print_statement(&mut self) -> Result<ParseTree> {
        let root = AstNode::make_leaf(Ast::Print);
        let tree = binexpr(self.scanner, 0)?.new_root(root);

        self.scanner.semi();

        Ok(tree)
    }

    fn var_declaration(&mut self, _type_token: Token) -> Result<ParseTree> {
        let ident = self.scanner.ident();

        self.symbols.add_glob(&ident);
        // self.code_gen.gen_globsym(&ident)?;
        self.scanner.semi();

        Ok(Tree::new(AstNode::make_leaf(Ast::GlobalDec(ident))))
    }

    fn assignment_statement(&mut self, ident: String) -> Result<ParseTree> {
        if self.symbols.find_glob(&ident).is_some() {
            let right = AstNode::make_leaf(Ast::LvIdent(ident));
            self.scanner.matches(Token::Assign, "=");
            let mut left = binexpr(self.scanner, 0)?;


            let right_idx = left.append(right, false);
            let tree = left.new_root_with_right_idx(AstNode::make_leaf(Ast::Assign), right_idx);
            // let _ = self.code_gen.gen_ast(&tree, None, None)?;
            // self.code_gen.gen_freeregs()?;
            Ok(tree)
        } else {
            self.scanner.fatal_extra("Undeclared variable", ident)
        }
    }

    fn if_statement(&mut self) -> Result<ParseTree> {
        self.scanner.lparen();

        let condition = binexpr(self.scanner, 0)?;

        // Temporarily limit the "if" condition to comparisons
        match condition.get_root() {
            Some(root) => {
                if !root.op.is_comparison() {
                    self.scanner.fatal("Bad comparison operator");
                }
            },
            None => unreachable!("Binary expression tree without a root!"),
        }

        self.scanner.rparen();

        let true_branch = self.compound_statement()?;

        let false_branch = if self.scanner.maybe_token(Token::Else) {
            self.compound_statement()?
        } else {
            Tree::empty()
        };

        let t =  Tree::new(AstNode::make_leaf(Ast::If));
        let (t, cond_index) = t.concat(condition);
        let (t, true_index) = t.concat(true_branch);
        let (mut t, false_index) = t.concat(false_branch);

        t.set_root_indices(cond_index, true_index, false_index);

        Ok(t)
    }

    pub fn compound_statement(&mut self) -> Result<ParseTree> {
        self.scanner.lbrace();

        let mut left: Option<ParseTree> = None;

        while let Some(t) = self.scanner.scan() {
            let right = match t {
                Token::Print => self.print_statement()?,
                Token::Int => self.var_declaration(t)?,
                Token::Ident(id) => self.assignment_statement(id)?,
                Token::If => self.if_statement()?,
                Token::RightBrace => {
                    return Ok(left.unwrap_or(Tree::empty()));
                },
                Token::Else|Token::LeftBrace|Token::LeftParen|Token::RightParen
                    => bail!("Syntax error, token {}, at line {}", t, self.scanner.get_line()),
                Token::Plus|Token::Minus|Token::Star|Token::Slash|Token::Assign
                           |Token::EQ|Token::NE|Token::GT|Token::GE|Token::LT|Token::LE
                    => {
                    bail!("Found operator {:?} while expecting a statment, at line {}", t, self.scanner.get_line())
                },
                Token::IntLit(_) => {
                    bail!("Found integer while expecting a statment, at line {}", self.scanner.get_line())
                }
                Token::Semi => continue // A semicolon on its own equals an empty statement
            };

            left = match left {
                Some(l) => Some(l.glue(right)),
                None => Some(right),
            }
        }

        panic!("EOF found while expecting '}}'")
    }
}


