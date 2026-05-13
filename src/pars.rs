use anyhow::{Result, bail};
use crate::{
    ast::{Ast, AstNode},
    expr::binexpr,
    scan::{Scanner, Token},
    sym::SymbolTable,
    tree::Tree,
};

/* Grammar
 *  compound_statement: '{' '}'          // empty, i.e. no statement
 *      |      '{' statement '}'
 *      |      '{' statement statements '}'
 *      ;
 *
 * statement: print_statement
 *      |     declaration
 *      |     assignment_statement
 *      |     if_statement
 *      |     while_statement
 *      |     for_statement
 *      ;
 *
 * print_statement: 'print' expression ';'  ;
 *
 * declaration: 'int' identifier ';'  ;
 *
 * assignment_statement: identifier '=' expression ';'   ;
 *
 * if_statement: if_head
 *      |        if_head 'else' compound_statement
 *      ;
 *
 * if_head: 'if' '(' true_false_expression ')' compound_statement  ;
 *
 * while_statement: 'while' '(' true_false_expression ')' compound_statement  ;
 *
 * for_statement: 'for' '(' preop_statement ';'
 *                          true_false_expression ';'
 *                          postop_statement ')' compound_statement  ;
 *
 * preop_statement:  statement  ;        (for now)
 * postop_statement: statement  ;        (for now)
 *
 * function_declaration: 'void' identifier '(' ')' compound_statement   ;
 *
 * identifier: T_IDENT ;
 */

type ParseTree = Tree<AstNode>;

fn compose(op: Ast, left: ParseTree, right: ParseTree) -> ParseTree {
    let (conc, right_idx) = left.concat(right);
    if let Some(idx) = right_idx {
        conc.new_root_with_right_idx(AstNode::make_leaf(op), idx)
    } else {
        conc
    }
}

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

    fn condition(&self) -> Result<ParseTree> {
        let tree = binexpr(self.scanner, 0)?;

        // Temporarily limit the "if" condition to comparisons
        match tree.get_root() {
            Some(root) => {
                if !root.op.is_comparison() {
                    self.scanner.fatal("Bad comparison operator");
                }
            },
            None => unreachable!("Binary expression tree without a root!"),
        }

        Ok(tree)
    }

    fn print_statement(&mut self) -> Result<ParseTree> {
        let root = AstNode::make_leaf(Ast::Print);
        let tree = binexpr(self.scanner, 0)?.new_root(root);

        Ok(tree)
    }

    fn var_declaration(&mut self, _type_token: Token) -> Result<ParseTree> {
        let ident = self.scanner.ident();

        self.symbols.add_glob(&ident);
        // self.code_gen.gen_globsym(&ident)?;

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
        let condition = self.condition()?;
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

    fn while_statement(&mut self) -> Result<ParseTree> {
        self.scanner.lparen();
        let condition = self.condition()?;
        self.scanner.rparen();

        let (t, right_idx) = condition.concat(self.compound_statement()?);
        let root = AstNode::make_leaf(Ast::While);
        Ok(match right_idx {
            Some(idx) => t.new_root_with_right_idx(root, idx),
            None => t.new_root(root)
        })
    }

    fn for_statement(&mut self) -> Result<ParseTree> {
        // No need for new grammar elements or code generation to
        // represent the "for" loop. Instead we'll treat it as syntactic
        // sugar for:
        //
        // pre_op;
        // while condition {
        //    body
        //    post_op;
        // }
        //
        //
        self.scanner.lparen();

        let pre_op = self.single_statement()?;
        self.scanner.semi();

        let condition = self.condition()?;
        self.scanner.semi();

        let post_op = self.single_statement()?;
        self.scanner.rparen();

        let body = self.compound_statement()?;

        let tree = compose(Ast::Glue,
                           pre_op,
                           compose(Ast::While,
                                   condition,
                                   compose(Ast::Glue,
                                           body,
                                           post_op)));

        Ok(tree)
    }

    pub fn function_declaration(&mut self) -> Result<Option<ParseTree>> {
        if let Some(t) = self.scanner.scan() {
            if t != Token::Void {
                bail!("Expected function declaration, found {}", t);
            }

            let ident = self.scanner.ident();
            self.symbols.add_glob(&ident);
            self.scanner.lparen();
            self.scanner.rparen();
            let tree = self.compound_statement()?
                           .new_root(AstNode::make_leaf(Ast::Function(ident)));
            Ok(Some(tree))
        } else {
            Ok(None)
        }

    }

    pub fn single_statement(&mut self) -> Result<ParseTree> {
        match self.scanner.scan() {
            Some(Token::Print) => self.print_statement(),
            Some(t @ Token::Int) => self.var_declaration(t),
            Some(Token::Ident(id)) => self.assignment_statement(id),
            Some(Token::If) => self.if_statement(),
            Some(Token::For) => self.for_statement(),
            Some(Token::While) => self.while_statement(),
            Some(t @ Token::Else
                |t @ Token::LeftBrace
                |t @ Token::LeftParen
                |t @ Token::RightParen)
                => bail!("Syntax error, token {}, at line {}", t, self.scanner.get_line()),
            Some(t @ Token::Plus
                |t @ Token::Minus
                |t @ Token::Star
                |t @ Token::Slash
                |t @ Token::Assign
                |t @ Token::EQ
                |t @ Token::NE
                |t @ Token::GT
                |t @ Token::GE
                |t @ Token::LT
                |t @ Token::LE)
                => {
                bail!("Found operator {:?} while expecting a statment, at line {}", t, self.scanner.get_line())
            },
            Some(Token::IntLit(_)) => {
                bail!("Found integer while expecting a statment, at line {}", self.scanner.get_line())
            }
            Some(Token::RightBrace) => panic!("Expected statement, found '}}'"),
            // A semicolon on its own equals an empty statement
            Some(Token::Semi) => panic!("Expected statement, found ';'"),
            // Only for function declarations right now
            Some(Token::Void) => panic!("Expected statement, found 'void'"),
            None => { panic!("EOF found while expecting a statement") }
        }
    }

    pub fn compound_statement(&mut self) -> Result<ParseTree> {
        self.scanner.lbrace();

        let mut left = Tree::empty();

        while !self.scanner.is_eof() {
            if self.scanner.maybe_token(Token::RightBrace) {
                return Ok(left);
            } else if self.scanner.maybe_token(Token::Semi) {
                // Empty statement
                continue
            }

            let tree = self.single_statement()?;
            if let Some(AstNode { op, .. })= tree.get_root() {
                if matches!(op, Ast::Print|Ast::Assign) {
                    self.scanner.semi();
                }

                left = compose(Ast::Glue, left, tree);
            }
        }

        panic!("EOF found while expecting '}}'")
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::*;
    use crate::ast::Ast;
    use crate::scan::Scanner;
    use crate::sym::SymbolTable;

    fn parser_from(s: &str) -> (Scanner<Cursor<Vec<u8>>>, SymbolTable) {
        (Scanner::new(Cursor::new(s.as_bytes().to_vec())), SymbolTable::new())
    }

    fn contains_op(tree: &ParseTree, op: &Ast) -> bool {
        let mut i = 0;
        while let Some(node) = tree.get_node(i) {
            if &node.op == op { return true; }
            i += 1;
        }
        false
    }

    #[test]
    fn compound_statement_empty_body() {
        let (scanner, mut symbols) = parser_from("{}");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert!(tree.is_empty());
    }

    #[test]
    fn compound_statement_var_declaration() {
        let (scanner, mut symbols) = parser_from("{ int x; }");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert!(matches!(tree.get_root().unwrap().op, Ast::GlobalDec(ref s) if s == "x"));
    }

    #[test]
    fn compound_statement_print() {
        let (scanner, mut symbols) = parser_from("{ print 42; }");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert!(contains_op(&tree, &Ast::Print));
        assert!(contains_op(&tree, &Ast::IntLit(42)));
    }

    #[test]
    fn compound_statement_assignment() {
        let (scanner, mut symbols) = parser_from("{ int x; x = 5; }");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert!(contains_op(&tree, &Ast::Assign));
        assert!(contains_op(&tree, &Ast::LvIdent("x".into())));
    }

    #[test]
    fn compound_statement_if_no_else() {
        let (scanner, mut symbols) = parser_from("{ int x; if (x == 5) { print x; } }");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert!(contains_op(&tree, &Ast::If));
        assert!(contains_op(&tree, &Ast::Equal));
    }

    #[test]
    fn compound_statement_if_else() {
        let (scanner, mut symbols) = parser_from(
            "{ int x; if (x < 5) { print x; } else { print x; } }"
        );
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert!(contains_op(&tree, &Ast::If));
        assert!(contains_op(&tree, &Ast::LessThan));
    }

    #[test]
    fn compound_statement_while() {
        let (scanner, mut symbols) = parser_from("{ int x; while (x < 10) { print x; } }");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert!(contains_op(&tree, &Ast::While));
        assert!(contains_op(&tree, &Ast::LessThan));
    }

    #[test]
    #[should_panic]
    fn compound_statement_undeclared_variable_panics() {
        let (scanner, mut symbols) = parser_from("{ x = 5; }");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let _ = parser.compound_statement();
    }

    #[test]
    fn compound_statement_for_loop() {
        let (scanner, mut symbols) = parser_from(
            "{ int i; for (i= 1; i <= 10; i= i + 1) { print i; } }"
        );
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert!(contains_op(&tree, &Ast::While));
        assert!(contains_op(&tree, &Ast::LessThanOrEqual));
        assert!(contains_op(&tree, &Ast::Print));
    }

    #[test]
    fn compound_statement_empty_semis() {
        let (scanner, mut symbols) = parser_from("{ ; ; ; }");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert!(tree.is_empty());
    }

    #[test]
    fn single_statement_var_declaration() {
        let (scanner, mut symbols) = parser_from("int x");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.single_statement().expect("parse failed");
        assert!(matches!(tree.get_root().unwrap().op, Ast::GlobalDec(ref s) if s == "x"));
    }

    #[test]
    fn single_statement_for() {
        let (scanner, mut symbols) = parser_from(
            "for (i= 1; i <= 10; i= i + 1) { print i; }"
        );
        symbols.add_glob("i");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.single_statement().expect("parse failed");
        assert!(contains_op(&tree, &Ast::While));
        assert!(contains_op(&tree, &Ast::LessThanOrEqual));
    }

    #[test]
    #[should_panic]
    fn condition_bad_comparison_panics() {
        let (scanner, mut symbols) = parser_from(
            "{ int x; if (x) { print x; } }"
        );
        let mut parser = Parser::new(&scanner, &mut symbols);
        let _ = parser.compound_statement();
    }
}

