use anyhow::{Result, bail};
use crate::{
    ast::{Ast, AstNode},
    cgen::{CodeBackend, CodeGenerator},
    expr::binexpr,
    scan::{Scanner, Token},
    sym::SymbolTable,
};

pub struct Parser<'a, T, B>
    where T: std::io::Read,
          B: CodeBackend,

{
    scanner: &'a Scanner<T>,
    code_gen: &'a mut CodeGenerator<B>,
    symbols: &'a mut SymbolTable,
}

impl<'a, T, B> Parser<'a, T, B>
where T: std::io::Read,
      B: CodeBackend,
{
    pub fn new(scanner: &'a Scanner<T>, code_gen: &'a mut CodeGenerator<B>, symbols: &'a mut SymbolTable) -> Self {
        Parser {
            scanner,
            code_gen,
            symbols,
        }
    }

    fn print_statement(&mut self) -> Result<()> {
        let tree = binexpr(self.scanner, 0)?;
        let reg = self.code_gen.gen_ast(&tree, None, None)?;
        self.code_gen.gen_printint(reg)?;
        self.code_gen.gen_freeregs()?;
        self.scanner.semi();

        Ok(())
    }

    fn var_declaration(&mut self, _type_token: Token) -> Result<()> {
        let ident = self.scanner.ident();
        self.symbols.add_glob(&ident);
        self.code_gen.gen_globsym(&ident)?;
        self.scanner.semi();

        Ok(())
    }

    fn assignment_statement(&mut self, ident: String) -> Result<()> {
        if self.symbols.find_glob(&ident).is_some() {
            let right = AstNode::make_leaf(Ast::LvIdent(ident));
            self.scanner.matches(Token::Equals, "=");
            let mut left = binexpr(self.scanner, 0)?;
            let right_idx = left.append(right, false);
            let tree = left.new_root_with_right_idx(AstNode::make_leaf(Ast::Assign), right_idx);
            let _ = self.code_gen.gen_ast(&tree, None, None)?;
            self.code_gen.gen_freeregs()?;
        } else {
            self.scanner.fatal_extra("Undeclared variable", ident)
        }

        Ok(())
    }

    pub fn statements(&mut self) -> Result<()> {
        while let Some(t) = self.scanner.scan() {
            match t {
                Token::Print => self.print_statement()?,
                Token::Int => self.var_declaration(t)?,
                Token::Ident(id) => self.assignment_statement(id)?,
                Token::Plus|Token::Minus|Token::Star|Token::Slash|Token::Equals => {
                    bail!("Found operator {:?} while expecting a statment, at line {}", t, self.scanner.get_line())
                },
                Token::IntLit(_) => {
                    bail!("Found integer while expecting a statment, at line {}", self.scanner.get_line())
                }
                Token::Semi => {} // A semicolon on its own equals an empty statement
            }
        }

        Ok(())
    }
}


