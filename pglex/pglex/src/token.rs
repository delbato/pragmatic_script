use crate::{
    lexer::Lexer,
    source::Source
};

use std::{
    hash::{
        Hash
    },
    collections::{
        HashSet
    }
};

pub trait TokenType: Sized + Clone + Eq + Hash {
    fn lexer<'source, S: Source<'source>>(source: S) -> Lexer<Self, S>;
    fn match_token(slice: &str) -> Vec<Self>;
    fn get_end_variant() -> Self;
    fn get_error_variant() -> Self;
    fn should_skip(&self) -> bool; 
}