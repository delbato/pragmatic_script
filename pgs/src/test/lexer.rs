use crate::{
    parser::{
        lexer::Token
    }
};
use logos::Logos;

#[test]
fn test_string_literal() {
    let lexer = Token::lexer("\"This is a string literal.\"");

    assert_eq!(lexer.token, Token::StringLiteral);
    assert_eq!(lexer.slice(), "\"This is a string literal.\"");
}

#[test]
fn test_function_decl() {
    let mut lexer = Token::lexer("fn main() {}");

    assert_eq!(lexer.token, Token::Fn);

    lexer.advance();

    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "main");
    
    lexer.advance();

    assert_eq!(lexer.token, Token::OpenParan);

    lexer.advance();

    assert_eq!(lexer.token, Token::CloseParan);

    lexer.advance();

    assert_eq!(lexer.token, Token::OpenBlock);

    lexer.advance();

    assert_eq!(lexer.token, Token::CloseBlock);
}