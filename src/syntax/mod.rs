pub mod cst;
pub mod layout;
pub mod lexer;
pub mod parser;
pub mod service;
pub mod token;

use std::path::PathBuf;

pub use cst::*;
pub use layout::layout;
pub use lexer::{LexMode, Lexer};
pub use parser::{ParseError, Parser};
pub use token::{Span, Token, TokenKind};

pub fn parse(src: &str) -> Result<Module, ParseError> {
    parse_with_path(src, PathBuf::from("<memory>"))
}

pub fn parse_with_path(src: &str, path: PathBuf) -> Result<Module, ParseError> {
    let laid_out = layout::layout(src);
    let lexer = Lexer::new(&laid_out);
    let mut parser = Parser::new(lexer);
    parser.parse_module(path)
}

pub fn parse_expression(src: &str) -> Result<Expr, ParseError> {
    let laid_out = layout::layout(src);
    let lexer = Lexer::new(&laid_out);
    let mut parser = Parser::new(lexer);
    parser.parse_expr()
}
