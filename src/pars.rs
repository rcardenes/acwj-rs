use anyhow::{Result, bail};
use crate::{
    ast::AstNode,
    expr::{Compatibility, ExpressionGenerator, is_type_compatible},
    scan::{Scanner, Token},
    sym::{PrimType, StructuralType, SymbolTable},
};

pub struct Parser<'a, T>
    where T: std::io::Read,

{
    scanner: &'a Scanner<T>,
    symbols: &'a SymbolTable,
    expr: ExpressionGenerator<'a, T>,
}

impl<'a, T> Parser<'a, T>
where T: std::io::Read,
{
    pub fn new(scanner: &'a Scanner<T>, symbols: &'a SymbolTable) -> Self {
        let expr = ExpressionGenerator::new(scanner, symbols);
        Parser {
            scanner,
            symbols,
            expr,
        }
    }

    fn condition(&self) -> Result<AstNode> {
        let tree = self.expr.binexpr(0)?;

        // Temporarily limit the boolean conditions to comparisons
        if !tree.is_comparison() {
            self.scanner.fatal("Bad comparison operator");
        }

        Ok(tree)
    }

    fn print_statement(&self) -> Result<AstNode> {
        let tree = self.expr.binexpr(0)?;
        let tree_type = tree.get_type()
                                      .unwrap_or_else(|| self.scanner.fatal("Binary expression without a type!"));
        let tree = match is_type_compatible(PrimType::Int, tree_type, false) {
            Compatibility::Incompatible => self.scanner.fatal("Incompatible types!"),
            Compatibility::Compatible(_) | Compatibility::WidenLeft(_) => tree,
            Compatibility::WidenRight(t) => tree.new_type(t),

        };
        Ok(AstNode::make_print(tree))
    }

    fn var_declaration(&self, type_token: Token) -> Result<AstNode> {
        let dtype = PrimType::from(type_token);
        let ident = self.scanner.ident();

        self.symbols.add_glob(&ident, dtype, StructuralType::Variable);
        // self.code_gen.gen_globsym(&ident)?;

        Ok(AstNode::make_global_declaration(&ident, dtype))
    }

    fn assignment_statement(&self, ident: String) -> Result<AstNode> {
        if let Some(sym) = self.symbols.find_glob(&ident) {
            let id = AstNode::make_lvident(&ident, sym.dtype);
            self.scanner.matches(Token::Assign, "=");
            let expr = self.expr.binexpr(-1)?;

            let ltype = expr.get_type()
                                      .unwrap_or_else(|| self.scanner.fatal("Binary expression without a type!"));

            let expr = match is_type_compatible(ltype, sym.dtype, true) {
                Compatibility::Incompatible => self.scanner.fatal("Incompatible types!"),
                Compatibility::Compatible(_) => expr,
                Compatibility::WidenLeft(t) => expr.new_type(t),
                Compatibility::WidenRight(_) => unreachable!("Illegal to widen an lvalue")
            };


            // let _ = self.code_gen.gen_ast(&tree, None, None)?;
            // self.code_gen.gen_freeregs()?;
            Ok(AstNode::make_assign(id, expr))
        } else {
            self.scanner.fatal_extra("Undeclared variable", ident)
        }
    }

    fn if_statement(&self) -> Result<AstNode> {
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

    fn while_statement(&self) -> Result<AstNode> {
        self.scanner.lparen();
        let condition = self.condition()?;
        self.scanner.rparen();

        let body = self.compound_statement()?;

        Ok(AstNode::make_while(condition, body))
    }

    fn for_statement(&self) -> Result<AstNode> {
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

    pub fn function_declaration(&self) -> Result<Option<AstNode>> {
        if let Some(t) = self.scanner.scan() {
            if t != Token::Void {
                bail!("Expected function declaration, found {}", t);
            }

            let ident = self.scanner.ident();
            let name = AstNode::make_ident(&ident, PrimType::Void);
            self.symbols.add_glob(&ident, PrimType::Void, StructuralType::Function);
            self.scanner.lparen();
            self.scanner.rparen();
            let body = self.compound_statement()?;
            Ok(Some(AstNode::make_function(name, body)))
        } else {
            Ok(None)
        }

    }

    pub fn single_statement(&self) -> Result<AstNode> {
        match self.scanner.scan() {
            Some(Token::Print) => self.print_statement(),
            Some(t @ Token::Int
                |t @ Token::Char) => self.var_declaration(t),
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

    pub fn compound_statement(&self) -> Result<AstNode> {
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
        let (scanner, symbols) = parser_from("{}");
        let parser = Parser::new(&scanner, &symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::Empty);
    }

    #[test]
    fn compound_statement_var_declaration() {
        let (scanner, symbols) = parser_from("{ int x; }");
        let parser = Parser::new(&scanner, &symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::GlobalDec { id: Identifier::new("x"), dtype: PrimType::Int });
    }

    #[test]
    fn compound_statement_print() {
        let (scanner, symbols) = parser_from("{ print 42; }");
        let parser = Parser::new(&scanner, &symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_print(AstNode::make_intlit(42, PrimType::Int)));
    }

    #[test]
    fn compound_statement_multiple_statements() {
        let (scanner, symbols) = parser_from("{ print 1; print 2; }");
        let parser = Parser::new(&scanner, &symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_glue(
            AstNode::make_print(AstNode::make_intlit(1, PrimType::Int)),
            AstNode::make_print(AstNode::make_intlit(2, PrimType::Int)),
        ));
    }

    #[test]
    fn compound_statement_skips_standalone_semicolons() {
        let (scanner, symbols) = parser_from("{ ; ; print 42; }");
        let parser = Parser::new(&scanner, &symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_print(AstNode::make_intlit(42, PrimType::Int)));
    }

    #[test]
    fn compound_statement_assignment() {
        let (scanner, symbols) = parser_from("{ int x; x = 5; }");
        let parser = Parser::new(&scanner, &symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_glue(
            AstNode::make_global_declaration("x",  PrimType::Int),
            AstNode::make_assign(
                AstNode::make_lvident("x", PrimType::Int),
                AstNode::make_intlit(5, PrimType::Int),
            ),
        ));
    }

    #[test]
    #[should_panic(expected = "Undeclared variable")]
    fn compound_statement_undeclared_var_panics() {
        let (scanner, symbols) = parser_from("{ x = 5; }");
        let parser = Parser::new(&scanner, &symbols);
        let _ = parser.compound_statement();
    }

    #[test]
    fn compound_statement_if_without_else() {
        let (scanner, symbols) = parser_from("{ if (1 < 2) { print 42; } }");
        let parser = Parser::new(&scanner, &symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_if(
            AstNode::make_binary(Token::LT, AstNode::make_intlit(1, PrimType::Char), AstNode::make_intlit(2, PrimType::Char), PrimType::Char),
            AstNode::make_print(AstNode::make_intlit(42, PrimType::Int)),
            None,
        ));
    }

    #[test]
    fn compound_statement_if_with_else() {
        let (scanner, symbols) = parser_from(
            "{ if (1 < 2) { print 1; } else { print 2; } }"
        );
        let parser = Parser::new(&scanner, &symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_if(
            AstNode::make_binary(Token::LT, AstNode::make_intlit(1, PrimType::Char), AstNode::make_intlit(2, PrimType::Char), PrimType::Char),
            AstNode::make_print(AstNode::make_intlit(1, PrimType::Int)),
            Some(AstNode::make_print(AstNode::make_intlit(2, PrimType::Int))),
        ));
    }

    #[test]
    fn compound_statement_while() {
        let (scanner, symbols) = parser_from("{ while (1 < 2) { print 42; } }");
        let parser = Parser::new(&scanner, &symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_while(
            AstNode::make_binary(Token::LT, AstNode::make_intlit(1, PrimType::Char), AstNode::make_intlit(2, PrimType::Char), PrimType::Char),
            AstNode::make_print(AstNode::make_intlit(42, PrimType::Int)),
        ));
    }

    #[test]
    fn compound_statement_for_desugars_to_glue_while() {
        // for (pre; cond; post) { body } becomes Glue(pre, While(cond, Glue(body, post)))
        let (scanner, symbols) = parser_from(
            "{ for (print 1; 1 < 2; print 3) { print 42; } }"
        );
        let parser = Parser::new(&scanner, &symbols);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_glue(
            AstNode::make_print(AstNode::make_intlit(1, PrimType::Int)),
            AstNode::make_while(
                AstNode::make_binary(Token::LT, AstNode::make_intlit(1, PrimType::Char), AstNode::make_intlit(2, PrimType::Char), PrimType::Char),
                AstNode::make_glue(
                    AstNode::make_print(AstNode::make_intlit(42, PrimType::Int)),
                    AstNode::make_print(AstNode::make_intlit(3, PrimType::Int)),
                ),
            ),
        ));
    }

    #[test]
    fn function_declaration_empty_body() {
        let (scanner, symbols) = parser_from("void foo() {}");
        let parser = Parser::new(&scanner, &symbols);
        let result = parser.function_declaration().expect("parse failed");
        assert_eq!(result, Some(AstNode::make_function(
            AstNode::make_ident("foo", PrimType::Void),
            AstNode::Empty,
        )));
    }

    #[test]
    fn function_declaration_eof_returns_none() {
        let (scanner, symbols) = parser_from("");
        let parser = Parser::new(&scanner, &symbols);
        let result = parser.function_declaration().expect("parse failed");
        assert_eq!(result, None);
    }

    #[test]
    fn function_declaration_non_void_fails() {
        let (scanner, symbols) = parser_from("int foo() {}");
        let parser = Parser::new(&scanner, &symbols);
        assert!(parser.function_declaration().is_err());
    }

    #[test]
    fn single_statement_operator_fails() {
        let (scanner, symbols) = parser_from("+ 5");
        let parser = Parser::new(&scanner, &symbols);
        assert!(parser.single_statement().is_err());
    }

    #[test]
    fn single_statement_intlit_fails() {
        let (scanner, symbols) = parser_from("42");
        let parser = Parser::new(&scanner, &symbols);
        assert!(parser.single_statement().is_err());
    }

    #[test]
    #[should_panic]
    fn single_statement_rbrace_panics() {
        let (scanner, symbols) = parser_from("}");
        let parser = Parser::new(&scanner, &symbols);
        let _ = parser.single_statement();
    }

    #[test]
    #[should_panic]
    fn single_statement_eof_panics() {
        let (scanner, symbols) = parser_from("");
        let parser = Parser::new(&scanner, &symbols);
        let _ = parser.single_statement();
    }

    #[test]
    #[should_panic]
    fn condition_bad_comparison_panics() {
        let (scanner, symbols) = parser_from(
            "{ int x; if (x) { print x; } }"
        );
        let parser = Parser::new(&scanner, &symbols);
        let _ = parser.compound_statement();
    }
}

