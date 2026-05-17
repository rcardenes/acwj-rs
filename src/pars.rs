use anyhow::{Result, bail};
use crate::{
    ast::AstNode,
    expr::binexpr,
    scan::{Scanner, Token},
    sym::SymbolTable,
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

    fn condition(&self) -> Result<AstNode> {
        let tree = binexpr(self.scanner, 0)?;

        // Temporarily limit the boolean conditions to comparisons
        if !tree.is_comparison() {
            self.scanner.fatal("Bad comparison operator");
        }

        Ok(tree)
    }

    fn print_statement(&mut self) -> Result<AstNode> {
        Ok(AstNode::make_print(binexpr(self.scanner, 0)?))
    }

    fn var_declaration(&mut self, _type_token: Token) -> Result<AstNode> {
        let ident = self.scanner.ident();

        self.symbols.add_glob(&ident);
        // self.code_gen.gen_globsym(&ident)?;

        Ok(AstNode::make_global_declaration(&ident))
    }

    fn assignment_statement(&mut self, ident: String) -> Result<AstNode> {
        if self.symbols.find_glob(&ident).is_some() {
            let id = AstNode::make_lvident(&ident);
            self.scanner.matches(Token::Assign, "=");
            let expr = binexpr(self.scanner, -1)?;


            // let _ = self.code_gen.gen_ast(&tree, None, None)?;
            // self.code_gen.gen_freeregs()?;
            Ok(AstNode::make_assign(id, expr))
        } else {
            self.scanner.fatal_extra("Undeclared variable", ident)
        }
    }

    fn if_statement(&mut self) -> Result<AstNode> {
        self.scanner.lparen();
        let condition = self.condition()?;
        self.scanner.rparen();

        let true_branch = self.compound_statement()?;

        let false_branch = if self.scanner.maybe_token(Token::Else) {
            Some(self.compound_statement()?)
        } else {
            None
        };

        Ok(AstNode::make_if(condition, true_branch, false_branch))
    }

    fn while_statement(&mut self) -> Result<AstNode> {
        self.scanner.lparen();
        let condition = self.condition()?;
        self.scanner.rparen();

        let body = self.compound_statement()?;

        Ok(AstNode::make_while(condition, body))
    }

    fn for_statement(&mut self) -> Result<AstNode> {
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

        Ok(AstNode::make_glue(
                pre_op,
                AstNode::make_while(
                    condition,
                    AstNode::make_glue(body, post_op))))
    }

    pub fn function_declaration(&mut self) -> Result<Option<AstNode>> {
        if let Some(t) = self.scanner.scan() {
            if t != Token::Void {
                bail!("Expected function declaration, found {}", t);
            }

            let ident = self.scanner.ident();
            let name = AstNode::make_ident(&ident);
            self.symbols.add_glob(&ident);
            self.scanner.lparen();
            self.scanner.rparen();
            let body = self.compound_statement()?;
            Ok(Some(AstNode::make_function(name, body)))
        } else {
            Ok(None)
        }

    }

    pub fn single_statement(&mut self) -> Result<AstNode> {
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
                bail!("Found operator {:?} while expecting a statement, at line {}", t, self.scanner.get_line())
            },
            Some(Token::IntLit(_)) => {
                bail!("Found integer while expecting a statement, at line {}", self.scanner.get_line())
            }
            Some(Token::RightBrace) => panic!("Expected statement, found '}}'"),
            // A semicolon on its own equals an empty statement
            Some(Token::Semi) => panic!("Expected statement, found ';'"),
            // Only for function declarations right now
            Some(Token::Void) => panic!("Expected statement, found 'void'"),
            None => { panic!("EOF found while expecting a statement") }
        }
    }

    pub fn compound_statement(&mut self) -> Result<AstNode> {
        self.scanner.lbrace();

        let mut left = AstNode::Empty;

        while !self.scanner.is_eof() {
            if self.scanner.maybe_token(Token::RightBrace) {
                return Ok(left);
            } else if self.scanner.maybe_token(Token::Semi) {
                // Empty statement
                continue
            }

            let tree = self.single_statement()?;
            if matches!(tree, AstNode::Print {..} | AstNode::Assign {..}) {
                self.scanner.semi();
            }

            if !matches!(tree, AstNode::Empty) {
                left = if left != AstNode::Empty {
                    AstNode::make_glue(left, tree)
                } else {
                    tree
                };
            }
        }

        panic!("EOF found while expecting '}}'")
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::*;
    use crate::ast::Identifier;
    use crate::scan::{Scanner, Token};
    use crate::sym::SymbolTable;

    fn parser_from(s: &str) -> (Scanner<Cursor<Vec<u8>>>, SymbolTable) {
        (Scanner::new(Cursor::new(s.as_bytes().to_vec())), SymbolTable::new())
    }

    #[test]
    fn compound_statement_empty_body() {
        let (scanner, mut symbols) = parser_from("{}");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::Empty);
    }

    #[test]
    fn compound_statement_var_declaration() {
        let (scanner, mut symbols) = parser_from("{ int x; }");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::GlobalDec { id: Identifier::new("x") });
    }

    #[test]
    fn compound_statement_print() {
        let (scanner, mut symbols) = parser_from("{ print 42; }");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_print(AstNode::IntLit(42)));
    }

    #[test]
    fn compound_statement_multiple_statements() {
        let (scanner, mut symbols) = parser_from("{ print 1; print 2; }");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_glue(
            AstNode::make_print(AstNode::IntLit(1)),
            AstNode::make_print(AstNode::IntLit(2)),
        ));
    }

    #[test]
    fn compound_statement_skips_standalone_semicolons() {
        let (scanner, mut symbols) = parser_from("{ ; ; print 42; }");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_print(AstNode::IntLit(42)));
    }

    #[test]
    fn compound_statement_assignment() {
        let (scanner, mut symbols) = parser_from("{ int x; x = 5; }");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_glue(
            AstNode::GlobalDec { id: Identifier::new("x") },
            AstNode::make_assign(
                AstNode::make_lvident("x"),
                AstNode::IntLit(5),
            ),
        ));
    }

    #[test]
    #[should_panic(expected = "Undeclared variable")]
    fn compound_statement_undeclared_var_panics() {
        let (scanner, mut symbols) = parser_from("{ x = 5; }");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let _ = parser.compound_statement();
    }

    #[test]
    fn compound_statement_if_without_else() {
        let (scanner, mut symbols) = parser_from("{ if (1 < 2) { print 42; } }");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_if(
            AstNode::make_binary(Token::LT, AstNode::IntLit(1), AstNode::IntLit(2)),
            AstNode::make_print(AstNode::IntLit(42)),
            None,
        ));
    }

    #[test]
    fn compound_statement_if_with_else() {
        let (scanner, mut symbols) = parser_from(
            "{ if (1 < 2) { print 1; } else { print 2; } }"
        );
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_if(
            AstNode::make_binary(Token::LT, AstNode::IntLit(1), AstNode::IntLit(2)),
            AstNode::make_print(AstNode::IntLit(1)),
            Some(AstNode::make_print(AstNode::IntLit(2))),
        ));
    }

    #[test]
    fn compound_statement_while() {
        let (scanner, mut symbols) = parser_from("{ while (1 < 2) { print 42; } }");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_while(
            AstNode::make_binary(Token::LT, AstNode::IntLit(1), AstNode::IntLit(2)),
            AstNode::make_print(AstNode::IntLit(42)),
        ));
    }

    #[test]
    fn compound_statement_for_desugars_to_glue_while() {
        // for (pre; cond; post) { body } becomes Glue(pre, While(cond, Glue(body, post)))
        let (scanner, mut symbols) = parser_from(
            "{ for (print 1; 1 < 2; print 3) { print 42; } }"
        );
        let mut parser = Parser::new(&scanner, &mut symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_glue(
            AstNode::make_print(AstNode::IntLit(1)),
            AstNode::make_while(
                AstNode::make_binary(Token::LT, AstNode::IntLit(1), AstNode::IntLit(2)),
                AstNode::make_glue(
                    AstNode::make_print(AstNode::IntLit(42)),
                    AstNode::make_print(AstNode::IntLit(3)),
                ),
            ),
        ));
    }

    #[test]
    fn function_declaration_empty_body() {
        let (scanner, mut symbols) = parser_from("void foo() {}");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let result = parser.function_declaration().expect("parse failed");
        assert_eq!(result, Some(AstNode::make_function(
            AstNode::make_ident("foo"),
            AstNode::Empty,
        )));
    }

    #[test]
    fn function_declaration_eof_returns_none() {
        let (scanner, mut symbols) = parser_from("");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let result = parser.function_declaration().expect("parse failed");
        assert_eq!(result, None);
    }

    #[test]
    fn function_declaration_non_void_fails() {
        let (scanner, mut symbols) = parser_from("int foo() {}");
        let mut parser = Parser::new(&scanner, &mut symbols);
        assert!(parser.function_declaration().is_err());
    }

    #[test]
    fn single_statement_operator_fails() {
        let (scanner, mut symbols) = parser_from("+ 5");
        let mut parser = Parser::new(&scanner, &mut symbols);
        assert!(parser.single_statement().is_err());
    }

    #[test]
    fn single_statement_intlit_fails() {
        let (scanner, mut symbols) = parser_from("42");
        let mut parser = Parser::new(&scanner, &mut symbols);
        assert!(parser.single_statement().is_err());
    }

    #[test]
    #[should_panic]
    fn single_statement_rbrace_panics() {
        let (scanner, mut symbols) = parser_from("}");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let _ = parser.single_statement();
    }

    #[test]
    #[should_panic]
    fn single_statement_eof_panics() {
        let (scanner, mut symbols) = parser_from("");
        let mut parser = Parser::new(&scanner, &mut symbols);
        let _ = parser.single_statement();
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

