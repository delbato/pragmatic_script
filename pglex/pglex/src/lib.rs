#[cfg(feature = "derive")]
pub extern crate pglex_derive as derive;
extern crate regex;
extern crate lazy_static;

pub mod lexer;

pub mod source;

pub mod token;

#[cfg(test)]
mod test;

pub mod prelude {
    pub use crate::lexer::Lexer;
    pub use crate::token::TokenType;
    pub use crate::source::Source;
}