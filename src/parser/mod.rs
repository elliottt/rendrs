use thiserror::Error;

mod lexer;
mod parser;

pub use parser::{parse, Target};
pub use lexer::Range;

#[derive(Error, Debug)]
pub enum Error {
    #[error("lexer error")]
    LexerError { range: lexer::Range },

    #[error("parser error")]
    ParserError,
}
