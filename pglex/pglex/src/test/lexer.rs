use crate::{
    token::TokenType,
    lexer::Lexer,
    source::Source
};

use regex::Regex;
use derive::TokenType;
use lazy_static::lazy_static;

#[derive(Clone, PartialEq, Debug, Eq, Hash)]
enum Token {
    Int,
    Float,
    Bool,
    Mod,
    IntLiteral,
    FloatLiteral,
    Text,
    Colon,
    DoubleColon,
    End,
    Error
}

#[derive(TokenType, Clone, PartialEq, Eq, Debug, Hash)]
enum DerivedToken {
    #[token = "float"]
    Float,
    #[regex = "[0-9]+"]
    IntLiteral,
    #[token = "skip"]
    #[skip]
    Skip,
    #[end]
    End,
    #[error]
    Error
}

impl TokenType for Token {
    fn lexer<'source, S>(source: S) -> Lexer<Token, S>
        where S: Source<'source> {
        let mut ret = Lexer::new(source);
        ret.advance();
        ret
    }

    fn match_token(slice: &str) -> Vec<Token> {
        let mut matches = Vec::new();
        lazy_static! {
            static ref int_regex: Regex = Regex::new(r"^\d+$").unwrap();
            static ref float_regex: Regex = Regex::new(r"^[0-9]+\.[0-9]+$").unwrap();
            static ref text_regex: Regex = Regex::new(r"^[a-zA-Z][a-zA-Z0-9]*$").unwrap();
        }

        if "int" == slice {
            matches.push(Token::Int);
        }
        if "float" == slice {
            matches.push(Token::Float);
        }
        if "bool" == slice {
            matches.push(Token::Bool);
        }
        if "mod" == slice {
            matches.push(Token::Mod);
        }
        if "::" == slice {
            matches.push(Token::DoubleColon);
        }
        if ":" == slice {
            matches.push(Token::Colon);
        }
        if int_regex.is_match(slice) {
            matches.push(Token::IntLiteral);
        }
        if float_regex.is_match(slice) {
            matches.push(Token::FloatLiteral);
        }
        if text_regex.is_match(slice) {
            matches.push(Token::Text);
        }

        matches
    } 

    fn get_end_variant() -> Token {
        Token::End
    }
    
    fn get_error_variant() -> Token {
        Token::Error
    }

    fn should_skip(&self) -> bool {
        false
    }
}

#[test]
fn test_lexer_basic() {
    let code = "bool float int";
    let mut lexer = Token::lexer(code);

    assert_eq!(lexer.token, Token::Bool);
    assert_eq!(lexer.slice(), "bool");
}

#[test]
fn test_lexer_int_literal() {
    let code = "1231232 123331";
    let mut lexer = Token::lexer(code);

    use regex::Regex;

    let regex = Regex::new(r"[0-9]+$").unwrap();
    assert!(regex.is_match("1234"));
    assert!(!regex.is_match("1234 "));

    assert_eq!(lexer.token, Token::IntLiteral);
    assert_eq!(lexer.slice(), "1231232");

    lexer.advance();
    assert_eq!(lexer.token, Token::IntLiteral);
    assert_eq!(lexer.slice(), "123331");
}

#[test]
fn test_lexer_float_literal() {
    let code = "128.774 12 3.14";
    let mut lexer = Token::lexer(code);

    use regex::Regex;

    let int_regex = Regex::new(r"^[0-9]+$").unwrap();
    assert!(int_regex.is_match("1234"));
    assert!(!int_regex.is_match("1234.1234"));

    assert_eq!(lexer.token, Token::FloatLiteral);
    assert_eq!(lexer.slice(), "128.774");

    lexer.advance();
    assert_eq!(lexer.token, Token::IntLiteral);
    assert_eq!(lexer.slice(), "12");

    lexer.advance();
    assert_eq!(lexer.token, Token::FloatLiteral);
    assert_eq!(lexer.slice(), "3.14");
}

#[test]
fn test_lexer_text() {
    let code = "this is some text float is not";
    let mut lexer = Token::lexer(code);

    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "this");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "is");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "some");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "text");

    lexer.advance();
    assert_eq!(lexer.token, Token::Float);
    assert_eq!(lexer.slice(), "float");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "is");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "not");
}

#[test]
fn test_lexer_keyword_text() {
    let code = "float int thisisntafloat intisntoneeither";
    let mut lexer = Token::lexer(code);

    assert_eq!(lexer.token, Token::Float);
    assert_eq!(lexer.slice(), "float");
    
    lexer.advance();
    assert_eq!(lexer.token, Token::Int);
    assert_eq!(lexer.slice(), "int");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "thisisntafloat");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "intisntoneeither");
}

#[test]
fn test_lexer_import_string() {
    let code = "
        root::some::other::module::function
    ";

    let mut lexer = Token::lexer(code);
    
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "root");

    lexer.advance();
    assert_eq!(lexer.token, Token::DoubleColon);
    assert_eq!(lexer.slice(), "::");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "some");

    lexer.advance();
    assert_eq!(lexer.token, Token::DoubleColon);
    assert_eq!(lexer.slice(), "::");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "other");

    lexer.advance();
    assert_eq!(lexer.token, Token::DoubleColon);
    assert_eq!(lexer.slice(), "::");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "module");

    lexer.advance();
    assert_eq!(lexer.token, Token::DoubleColon);
    assert_eq!(lexer.slice(), "::");

    lexer.advance();
    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "function");

}

#[test]
fn test_lexer_derived_basic() {
    let code = "1234 float";
    let mut lexer = DerivedToken::lexer(code);

    assert_eq!(lexer.token, DerivedToken::IntLiteral);
    assert_eq!(lexer.slice(), "1234");
}