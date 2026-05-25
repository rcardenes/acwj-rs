use anyhow::{Result, bail};
use crate::{
    ast::AstNode,
    expr::{Compatibility, ExpressionGenerator},
    scan::{Scanner, Token},
    cgen::{CodeBackend, CodeGenerator},
    sym::{PrimType, StructuralType, SymbolTable},
};

pub struct Parser<'a, T, B>
    where T: std::io::Read,
          B: CodeBackend,
{
    scanner: &'a Scanner<T>,
    symbols: &'a SymbolTable,
    expr: ExpressionGenerator<'a, T, B>,
}

impl<'a, T, B> Parser<'a, T, B>
where T: std::io::Read,
      B: CodeBackend,
{
    pub fn new(scanner: &'a Scanner<T>, symbols: &'a SymbolTable, code_gen: &'a CodeGenerator<B>) -> Self
    {
        let expr = ExpressionGenerator::new(scanner, symbols, code_gen);
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

    fn var_declaration(&self, type_token: Token) -> Result<AstNode> {
        let mut dtype = PrimType::from(type_token);
        while self.scanner.maybe_token(Token::Star) {
            dtype = dtype.pointer_to();
        }

        let ident = self.scanner.ident(None);

        self.symbols.add_glob(&ident, dtype, StructuralType::Variable);

        Ok(AstNode::make_global_declaration(&ident, dtype))
    }

    fn assignment_statement(&self, ident: String) -> Result<AstNode> {
        if let Some(sym) = self.symbols.find_glob(&ident) {
            let id = AstNode::make_lvident(&ident, sym.dtype);
            let expr = self.expr.binexpr(-1)?;

            let ltype = expr.get_type()
                                      .unwrap_or_else(|| self.scanner.fatal("Binary expression without a type!"));

            let expr = match self.expr.type_compatibility(ltype, sym.dtype, true) {
                Compatibility::Incompatible => self.scanner.fatal("Incompatible types!"),
                Compatibility::Compatible(_) => expr,
                Compatibility::WidenLeft(t) => expr.new_type(t),
                Compatibility::WidenRight(_) => unreachable!("Illegal to widen an lvalue")
            };


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
            if !t.is_type() {
                bail!("Expected function declaration, found {}", t);
            }

            let ftype: PrimType = t.into();
            let ident = self.scanner.ident(None);
            let name = AstNode::make_ident(&ident, ftype);
            self.symbols.add_glob_fn(&ident, ftype);
            self.scanner.lparen();
            self.scanner.rparen();
            self.expr.enter_function(ident.as_str());
            let body = self.compound_statement()?;
            self.expr.exit_function();
            if ftype != PrimType::Void {
                // Ensure that we return from this function
                let returns = match body {
                    AstNode::Return { .. } => true,
                    AstNode::Glue { ref right, .. } =>
                        matches!(**right, AstNode::Return { .. }),
                    _ => false,
                };

                if !returns {
                    self.scanner.fatal("No return for function with non-void type")
                }
            }
            Ok(Some(AstNode::make_function(name, body)))
        } else {
            Ok(None)
        }

    }

    pub fn return_statement(&self) -> Result<AstNode> {
        let ftype = self.expr.current_function_type();

        if ftype == PrimType::Void {
            self.scanner.fatal("Can't return from a void function")
        }

        self.scanner.lparen();
        let expr = self.expr.binexpr(0)?;
        let expr = match self.expr.type_compatibility(expr.get_type().unwrap(), ftype, true) {
            Compatibility::Incompatible => self.scanner.fatal("Incompatible return type"),
            Compatibility::Compatible(_) => expr,
            Compatibility::WidenLeft(t) => expr.new_type(t),
            Compatibility::WidenRight(_) => unreachable!("Can't widen right value when only_right is true"),
        };
        self.scanner.rparen();

        Ok(AstNode::make_return(&self.expr.current_function_name(), expr))
    }

    pub fn single_statement(&self) -> Result<AstNode> {
        match self.scanner.scan() {
            Some(t @ Token::Int
                |t @ Token::Char
                |t @ Token::Long) => self.var_declaration(t),
            Some(Token::Ident(id)) => {
                if self.scanner.maybe_token(Token::Assign) {
                    self.assignment_statement(id)
                } else if self.scanner.maybe_token(Token::LeftParen) {
                    self.expr.function_call(id.as_str())
                } else {
                    if let Some(next) = self.scanner.scan() {
                        self.scanner.fatal_extra("Expected assignment or function call, not", next)
                    } else {
                        self.scanner.fatal("EOF while parsing expression")
                    }
                }
            },
            Some(Token::If) => self.if_statement(),
            Some(Token::For) => self.for_statement(),
            Some(Token::While) => self.while_statement(),
            Some(Token::Return) => self.return_statement(),
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
                |t @ Token::LE
                |t @ Token::LogAnd) =>
            {
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
            Some(Token::Amper) => panic!("Expected statement, found '&'"),
            None => { panic!("EOF found while expecting a statement") },
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
            if matches!(tree, AstNode::Assign {..}|AstNode::FuncCall {..}) {
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
    use rstest::rstest;
    use super::*;
    use crate::{
        ast::Identifier,
        dummy_cg::DummyBackend,
    };

    type CodeGen<'a> = CodeGenerator<'a, DummyBackend>;

    fn codegen_from<'a>(symbols: &'a SymbolTable) -> CodeGen<'a> {
        CodeGenerator::new(DummyBackend{}, symbols)
    }

    fn parser_from(s: &str) -> (Scanner<Cursor<Vec<u8>>>, SymbolTable) {
        (Scanner::new(Cursor::new(s.as_bytes().to_vec())),
         SymbolTable::default())
    }

    #[test]
    fn compound_statement_empty_body() {
        let (scanner, symbols) = parser_from("{}");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::Empty);
    }

    #[rstest]
    #[case::char_decl("char", PrimType::Char)]
    #[case::int_decl("int", PrimType::Int)]
    #[case::long_decl("long", PrimType::Long)]
    fn compound_statement_var_declaration(#[case] kword: &str, #[case] dtype: PrimType) {
        let parser_text = format!("{{ {kword} x; }}");
        let (scanner, symbols) = parser_from(&parser_text);
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::GlobalDec { id: Identifier::new("x"), dtype });
    }

    #[test]
    fn compound_statement_multiple_statements() {
        let (scanner, symbols) = parser_from("{ printint(1); printint(2); }");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_glue(
            AstNode::make_function_call("printint", AstNode::make_intlit(1, PrimType::Char), PrimType::Void),
            AstNode::make_function_call("printint", AstNode::make_intlit(2, PrimType::Char), PrimType::Void),
        ));
    }

    #[test]
    fn compound_statement_skips_standalone_semicolons() {
        let (scanner, symbols) = parser_from("{ ; ; printint(42); }");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_function_call("printint", AstNode::make_intlit(42, PrimType::Char), PrimType::Void));
    }

    #[rstest]
    #[case::assign_char("char", PrimType::Char)]
    #[case::assign_char("int", PrimType::Int)]
    #[case::assign_char("long", PrimType::Long)]
    fn compound_statement_assignment(#[case] kword: &str, #[case] dtype: PrimType) {
        let parser_text = format!("{{ {kword} x; x = 5; }}");
        let (scanner, symbols) = parser_from(&parser_text);
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_glue(
            AstNode::make_global_declaration("x",  dtype),
            AstNode::make_assign(
                AstNode::make_lvident("x", dtype),
                AstNode::make_intlit(5, dtype),
            ),
        ));
    }

    #[test]
    #[should_panic]
    fn compound_statement_assignment_without_semi() {
        let (scanner, symbols) = parser_from("{ int x; x =5 }");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let _ = parser.compound_statement();
    }

    #[test]
    #[should_panic(expected = "Undeclared variable")]
    fn compound_statement_undeclared_var_panics() {
        let (scanner, symbols) = parser_from("{ x = 5; }");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let _ = parser.compound_statement();
    }

    #[test]
    fn compound_statement_if_without_else() {
        let (scanner, symbols) = parser_from("{ if (1 < 2) { printint(42); } }");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_if(
            AstNode::make_binary(Token::LT, AstNode::make_intlit(1, PrimType::Char), AstNode::make_intlit(2, PrimType::Char), PrimType::Char),
            AstNode::make_function_call("printint", AstNode::make_intlit(42, PrimType::Char), PrimType::Void),
            None,
        ));
    }

    #[test]
    fn compound_statement_if_with_else() {
        let (scanner, symbols) = parser_from(
            "{ if (1 < 2) { printint(1); } else { printint(2); } }"
        );
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_if(
            AstNode::make_binary(Token::LT, AstNode::make_intlit(1, PrimType::Char), AstNode::make_intlit(2, PrimType::Char), PrimType::Char),
            AstNode::make_function_call("printint", AstNode::make_intlit(1, PrimType::Char), PrimType::Void),
            Some(AstNode::make_function_call("printint", AstNode::make_intlit(2, PrimType::Char), PrimType::Void)),
        ));
    }

    #[test]
    fn compound_statement_while() {
        let (scanner, symbols) = parser_from("{ while (1 < 2) { printint(42); } }");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_while(
            AstNode::make_binary(Token::LT, AstNode::make_intlit(1, PrimType::Char), AstNode::make_intlit(2, PrimType::Char), PrimType::Char),
            AstNode::make_function_call("printint", AstNode::make_intlit(42, PrimType::Char), PrimType::Void),
        ));
    }

    #[test]
    fn compound_statement_for_desugars_to_glue_while() {
        // for (pre; cond; post) { body } becomes Glue(pre, While(cond, Glue(body, post)))
        let (scanner, symbols) = parser_from(
            "{ for (printint(1); 1 < 2; printint(3)) { printint(42); } }"
        );
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_glue(
            AstNode::make_function_call("printint", AstNode::make_intlit(1, PrimType::Char), PrimType::Void),
            AstNode::make_while(
                AstNode::make_binary(Token::LT, AstNode::make_intlit(1, PrimType::Char), AstNode::make_intlit(2, PrimType::Char), PrimType::Char),
                AstNode::make_glue(
                    AstNode::make_function_call("printint", AstNode::make_intlit(42, PrimType::Char), PrimType::Void),
                    AstNode::make_function_call("printint", AstNode::make_intlit(3, PrimType::Char), PrimType::Void),
                ),
            ),
        ));
    }

    #[test]
    fn compound_statement_empty_for_loop() {
        // for (pre; cond; post) { body } becomes Glue(pre, While(cond, Glue(body, post)))
        let (scanner, symbols) = parser_from(
            "{ char i; for (i = 0; i < 5; i = i + 1) {} }"
        );
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let tree = parser.compound_statement().expect("parse failed");
        let decl = AstNode::make_global_declaration("i", PrimType::Char);
        let while_stmt = AstNode::make_glue(
            AstNode::make_assign(AstNode::make_lvident("i", PrimType::Char), AstNode::make_intlit(0, PrimType::Char)),
            AstNode::make_while(
                AstNode::make_binary(Token::LT, AstNode::make_ident("i", PrimType::Char), AstNode::make_intlit(5, PrimType::Char), PrimType::Char),
                AstNode::make_glue(
                    AstNode::Empty,
                    AstNode::make_assign(AstNode::make_lvident("i", PrimType::Char),
                            AstNode::make_binary(Token::Plus, AstNode::make_ident("i", PrimType::Char),
                                                              AstNode::make_intlit(1, PrimType::Char),
                                                              PrimType::Char))
                ),
            ),
        );
        assert_eq!(tree, AstNode::make_glue(decl, while_stmt));
    }

    #[test]
    fn function_declaration_empty_body() {
        let (scanner, symbols) = parser_from("void foo() {}");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let result = parser.function_declaration().expect("parse failed");
        assert_eq!(result, Some(AstNode::make_function(
            AstNode::make_ident("foo", PrimType::Void),
            AstNode::Empty,
        )));
    }

    #[test]
    fn function_declaration_eof_returns_none() {
        let (scanner, symbols) = parser_from("");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let result = parser.function_declaration().expect("parse failed");
        assert_eq!(result, None);
    }

    #[test]
    fn single_statement_operator_fails() {
        let (scanner, symbols) = parser_from("+ 5");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        assert!(parser.single_statement().is_err());
    }

    #[test]
    fn single_statement_intlit_fails() {
        let (scanner, symbols) = parser_from("42");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        assert!(parser.single_statement().is_err());
    }

    #[test]
    #[should_panic]
    fn single_statement_rbrace_panics() {
        let (scanner, symbols) = parser_from("}");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let _ = parser.single_statement();
    }

    #[test]
    #[should_panic]
    fn single_statement_eof_panics() {
        let (scanner, symbols) = parser_from("");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let _ = parser.single_statement();
    }

    #[test]
    #[should_panic]
    fn condition_bad_comparison_panics() {
        let (scanner, symbols) = parser_from(
            "{ int x; if (x) { printint(x); } }"
        );
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let _ = parser.compound_statement();
    }

    // --- var_declaration with pointer types ---

    #[test]
    fn var_declaration_ptr() {
        let (scanner, symbols) = parser_from("{ int *x; }");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_global_declaration("x", PrimType::IntPtr));
    }

    #[test]
    fn var_declaration_char_ptr() {
        let (scanner, symbols) = parser_from("{ char *c; }");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let tree = parser.compound_statement().expect("parse failed");
        assert_eq!(tree, AstNode::make_global_declaration("c", PrimType::CharPtr));
    }

    // --- function_declaration with non-void return ---

    #[test]
    fn function_declaration_with_return_statement() {
        let (scanner, symbols) = parser_from("int foo() { return(42); }");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let result = parser.function_declaration().expect("parse failed");
        assert!(result.is_some());
    }

    #[test]
    #[should_panic(expected = "No return for function with non-void type")]
    fn function_declaration_non_void_no_return_panics() {
        let (scanner, symbols) = parser_from("int foo() {}");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let _ = parser.function_declaration();
    }

    #[test]
    #[should_panic(expected = "Incompatible return type")]
    fn return_incompatible_type_panics() {
        // type_compatibility(Long, Int, true) → Incompatible (Long larger than Int, only_right=true)
        let (scanner, symbols) = parser_from("int foo() { return(x); }");
        symbols.add_glob("x", PrimType::Long, StructuralType::Variable);
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let _ = parser.function_declaration();
    }

    // --- single_statement remaining error paths ---

    #[test]
    #[should_panic(expected = "Expected statement, found ';'")]
    fn single_statement_semi_panics() {
        let (scanner, symbols) = parser_from(";");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let _ = parser.single_statement();
    }

    #[test]
    #[should_panic(expected = "Expected statement, found 'void'")]
    fn single_statement_void_panics() {
        let (scanner, symbols) = parser_from("void");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let _ = parser.single_statement();
    }

    #[test]
    #[should_panic(expected = "Expected statement, found '&'")]
    fn single_statement_amper_panics() {
        let (scanner, symbols) = parser_from("&");
        let code_gen = codegen_from(&symbols);
        let parser = Parser::new(&scanner, &symbols, &code_gen);
        let _ = parser.single_statement();
    }
}

