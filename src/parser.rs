use super::{
    ast::{
        *
    },
    lexer::{
        Token
    }
};

use std::{
    collections::{
        HashMap
    },
    fmt::{
        Debug
    }
};

use logos::{
    Logos,
    Lexer
};

#[derive(Debug)]
pub enum ParseError {
    Unknown,
    EmptyInput,
    FnMissing,
    OpenParanMissing,
    CloseParanMissing,
    BlockMissing,
    ExpectedFunctionName,
    ReturnTypeMissing,
    UnknownType,
    ExpectedArgType,
    ExpectedArgName,
    DuplicateArg,
    ExpectedBlockOrSemicolon,
    ExpectedCloseBlock
}

pub type ParseResult<T> = Result<T, ParseError>;

pub struct Parser {
    code: String,
}

impl Parser {
    pub fn new(code: String) -> Self {
        Parser {
            code: code
        }
    }

    pub fn parse_decl_list(&self) -> ParseResult<Vec<Declaration>> {
        Err(ParseError::Unknown)
    }

    pub fn parse_function_decl(&self) -> ParseResult<Declaration> {
        let mut lexer = Token::lexer(self.code.as_str());
        let mut fn_decl_opt = None;

        // Parse "fn" literal
        if lexer.token != Token::Fn {
            return Err(ParseError::FnMissing);
        }
        lexer.advance();

        // Parse function name
        if lexer.token != Token::Text {
            return Err(ParseError::ExpectedFunctionName);
        }
        let fn_name = String::from(lexer.slice());
        lexer.advance();

        // Parse "("
        if lexer.token != Token::OpenParan {
            return Err(ParseError::OpenParanMissing);
        }
        lexer.advance();

        // Parse function arguments
        let fn_args = self.parse_fn_args(&mut lexer)?;

        lexer.advance();

        if lexer.token != Token::CloseParan {
            return Err(ParseError::CloseParanMissing);
        }
        lexer.advance();

        if lexer.token != Token::FnReturn {
            return Err(ParseError::ReturnTypeMissing);
        }
        lexer.advance();

        let fn_return_type = match lexer.token {
            Token::Float => {
                Type::Float
            },
            Token::Int => {
                Type::Int
            },
            Token::String => {
                Type::String
            },
            _ => {
                return Err(ParseError::UnknownType);
            }
        };

        lexer.advance();

        let code_block_opt;

        match lexer.token {
            Token::Semicolon => {
                code_block_opt = None;
            },
            Token::OpenBlock => {
                lexer.advance();
                if lexer.token != Token::CloseBlock {
                    return Err(ParseError::ExpectedCloseBlock);
                }
                code_block_opt = Some(Vec::new());
            },
            _ => {
                return Err(ParseError::ExpectedBlockOrSemicolon);
            }
        };

        let fn_raw = FunctionDeclArgs {
            name: fn_name,
            arguments: fn_args,
            returns: fn_return_type,
            code_block: code_block_opt
        };

        fn_decl_opt = Some(
            Declaration::Function(fn_raw)
        );

        fn_decl_opt.ok_or(ParseError::Unknown)
    }

    pub fn parse_fn_args(&self, lexer: &mut Lexer<Token, &str>) -> ParseResult<HashMap<String, Type>> {
        let mut ret = HashMap::new();

        loop {
            let fn_arg_res = self.parse_fn_arg(lexer);
            if fn_arg_res.is_err() {
                break;
            }
            let fn_arg = fn_arg_res.unwrap();
            if ret.insert(fn_arg.0, fn_arg.1) != None {
                return Err(ParseError::DuplicateArg);
            }
            let lexer_backup = lexer.clone();
            lexer.advance();
            if lexer.token != Token::Comma {
                *lexer = lexer_backup;
                break;
            }
        }

        Ok(ret)
    }

    pub fn parse_fn_arg(&self, lexer: &mut Lexer<Token, &str>) -> ParseResult<(String, Type)> {
        let mut lexer_backup = lexer.clone();

        let arg_type = match lexer.token {
            Token::Int => Type::Int,
            Token::Float => Type::Float,
            Token::String => Type::String,
            _ => {
                *lexer = lexer_backup;
                return Err(ParseError::ExpectedArgType);
            }
        };

        lexer.advance();

        lexer_backup = lexer.clone();

        if lexer.token != Token::Text {
            *lexer = lexer_backup;
            return Err(ParseError::ExpectedArgName);
        }

        let arg_name = String::from(lexer.slice());

        Ok(
            (arg_name, arg_type)
        )
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty_fn_decl() {
        let code = String::from("fn main(int arg) ~ int;");
        let parser = Parser::new(code);
        let decl_res = parser.parse_function_decl();

        assert!(decl_res.is_ok());

        let Declaration::Function(fn_decl) = decl_res.unwrap();

        assert_eq!(fn_decl.name, String::from("main"));
        assert_eq!(fn_decl.arguments.len(), 1);
        assert!(fn_decl.code_block.is_none());
    }

    #[test]
    fn test_full_fn_decl() {
        let code = String::from("fn main(int arg) ~ int {}");
        let parser = Parser::new(code);
        let decl_res = parser.parse_function_decl();

        assert!(decl_res.is_ok());

        let Declaration::Function(fn_decl) = decl_res.unwrap();

        assert_eq!(fn_decl.name, String::from("main"));
        assert_eq!(fn_decl.arguments.len(), 1);
        assert!(fn_decl.code_block.is_some());
    }
}
