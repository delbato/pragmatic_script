
use std::{
    fmt::{
        Debug,
        self
    }
};

use logos::{
    Logos,
    Lexer as LogosLexer,
    Source
};

pub type Lexer<'s> = LogosLexer<Token, &'s str>;

#[derive(Logos, Debug, PartialEq, Clone)]
pub enum Token {
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

    #[regex = "([a-zA-Z_][a-zA-Z0-9_]*)"]
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

    #[token = "ö"]
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
    Return,

    //#[regex = "//[.]*\n"]
    //#[regex = "#[.]*\n"]
    //#[regex = "/**[.]*/"]
    //#[callback = "ignore_comments"]

    #[end]
    End,

    #[regex = "//[^\n]*"]
    #[regex = "#[^\n]*"]
    #[token = "/*"]
    #[callback = "ignore_comments"]
    Comment,

    #[error]
    Error
}


/// # Skips producing Comment Tokens
/// 
/// Required as a workaround for Logos, which is sort of broken rn anyway.  
/// Consider forking.
pub fn ignore_comments<'source, Src: Source<'source>>(lexer: &mut LogosLexer<Token, Src>) {
    use logos::internal::LexerInternal;
    use logos::Slice;
    // If this fits the "multiline comment" token
    if lexer.slice().as_bytes() == b"/*" {
        // Loop until end of string or end of comment, skipping any content
        loop {
            // Read byte val at current position
            let read_opt = lexer.read();
            // If read errors, produce an error token
            if read_opt.is_none() {
                return lexer.token = Token::Error;
            }
            // Get value
            let val = read_opt.unwrap();
            match val {
                // If its zero for some reason
                0 => return lexer.token = Token::Error,
                // If current char is a "*"
                b'*' => {
                    // And the immediately next one is a "/", meaning the comment end with "*/"
                    if lexer.read_at(1) == Some(b'/') {
                        // Bump the lexer up by two char positions, effectively skipping the comment
                        lexer.bump(2);
                        break;
                    } else {
                        // Otherwise only skip this sole "*"
                        lexer.bump(1);
                    }
                },
                // Skip any and all characters
                _ => lexer.bump(1),
            }
        }
    }
    // Finally, produce the next token after the comment
    lexer.advance();
}