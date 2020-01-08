#[cfg(feature = "derive")]
pub extern crate pglex_derive as derive;

extern crate regex;
#[macro_use] extern crate lazy_static;

pub mod lexer;

pub mod source;

pub mod token;

//#[cfg(test)]
mod test;