
use std::{
    fmt::{
        Debug,
        self
    }
};

use pglex::prelude::*;

pub type PgLexer<'source> = Lexer<Token, &'source str>;

#[derive(Lexable, Hash, Eq, Debug, PartialEq, Clone)]
pub enum Token {
    #[token = "fn"]
    #[prio = 1]
    Fn,

    #[token = "cont"]
    #[prio = 1]
    Container,

    #[token = "var"]
    #[prio = 1]
    Var,

    #[token = "mod"]
    #[prio = 1]
    Mod,

    #[token = "import"]
    #[prio = 1]
    Import,

    #[token = "int"]
    #[prio = 1]
    Int,

    #[token = "float"]
    #[prio = 1]
    Float,

    #[token = "string"]
    #[prio = 1]
    String,

    #[token = "for"]
    #[prio = 1]
    For,

    #[token = "loop"]
    #[prio = 1]
    Loop,

    #[token = "while"]
    #[prio = 1]
    While,

    #[token = "bool"]
    #[prio = 1]
    Bool,

    #[token = "true"]
    #[prio = 1]
    True,

    #[token = "false"]
    #[prio = 1]
    False,

    #[token = "if"]
    #[prio = 1]
    If,

    #[token = "!"]
    Not,

    #[token = "else"]
    #[prio = 1]
    Else,

    #[token = "break"]
    #[prio = 1]
    Break,

    #[token = "continue"]
    #[prio = 1]
    Continue,

    #[regex = "([a-zA-Z_][a-zA-Z0-9_]*)"]
    Text,

    #[regex = "[0-9]+"]
    IntLiteral,

    #[regex = "([0-9]+\\.[0-9]+)"]
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
    Tilde,

    #[token = "return"]
    #[prio = 1]
    Return,

    #[end]
    End,

    #[token_start = "//"]
    #[token_end = "\n"]
    #[skip]
    SingleLineComment,

    #[token_start = "#"]
    #[token_end = "\n"]
    #[skip]
    HashLineComment,

    #[token_start = "/*"]
    #[token_end = "*/"]
    #[skip]
    MultiLineComment,

    #[error]
    Error
}