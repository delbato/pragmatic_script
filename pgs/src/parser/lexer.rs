
use std::{
    fmt::{
        Debug
    }
};

use logos::{
    Logos,
    Lexer as LogosLexer
};

pub type Lexer<'s> = LogosLexer<Token, &'s str>;

#[derive(Logos, Debug, PartialEq, Clone)]
pub enum Token {
    #[end]
    End,

    #[error]
    Error,

    #[token = "fn"]
    Fn,

    #[token = "struct"]
    Struct,

    #[token = "var"]
    Var,

    #[token = "mod"]
    Mod,

    #[token = "import"]
    Import,

    #[token = "int"]
    Int,

    #[token = "float"]
    Float,

    #[token = "string"]
    String,

    #[token = "bool"]
    Bool,

    #[token = "true"]
    True,

    #[token = "false"]
    False,

    #[regex = "([a-zA-Z][a-zA-Z0-9]*)"]
    Text,

    #[regex = "[0-9]+"]
    IntLiteral,

    #[regex = "[0-9]+\\.[0-9+]"]
    FloatLiteral,

    #[regex = "\"([^\"]|\\.)*\""]
    StringLiteral,

    #[token = "("]
    OpenParan,

    #[token = ")"]
    CloseParan,
    
    #[token = "{"]
    OpenBlock,

    #[token = "}"]
    CloseBlock,

    #[token = ","]
    Comma,
    
    #[token = ";"]
    Semicolon,

    #[token = ":"]
    Colon,

    #[token = "::"]
    DoubleColon,

    #[token = "="]
    Assign,

    #[token = "+"]
    Plus,
    
    #[token = "-"]
    Minus,

    #[token = "*"]
    Times,

    #[token = "/"]
    Divide,

    #[token = "=="]
    Equals,

    #[token = "!="]
    NotEquals,

    #[token = "<"]
    LessThan,

    #[token = ">"]
    GreaterThan,

    #[token = "<="]
    LessThanEquals,
    
    #[token = ">="]
    GreaterThanEquals,

    #[token = "~"]
    FnReturn,

    #[token = "return"]
    Return
}
