use crate::scan::Token;

pub fn fatal_pos(error_msg: &str, tok: Token) -> ! {
    let Token { line, col, ttype: _ } = tok;

    panic!("{error_msg} at {line},{col}")
}

pub fn fatal_tok(error_msg: &str, tok: Token) -> ! {
    let Token { line, col, ttype } = tok;

    panic!("{error_msg}: {ttype} at {line},{col}")
}

pub fn fatal_other<D>(error_msg: &str, tok: Token, data: D) -> !
where D: std::fmt::Display,
{
    let Token { line, col, ttype: _ } = tok;

    panic!("{error_msg}: {data} at {line},{col}")
}
