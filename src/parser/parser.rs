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
        HashMap,
        VecDeque
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
    ExpectedCloseBlock,
    UnknownStatement,
    ExpectedVarName,
    ExpectedAssignment,
    ExpectedSemicolon,
    UnsupportedExpression
}

pub type ParseResult<T> = Result<T, ParseError>;

pub struct Parser {
    code: String,
}

fn op_prec(token: &Token) -> i8 {
    match token {
        Token::Times => 1,
        Token::Divide => 1,
        Token::Plus => 0,
        Token::Minus => 0,
        _ => {
            panic!("ERROR! Not an operator");
        }
    }
}

impl Parser {
    pub fn new(code: String) -> Self {
        Parser {
            code: code
        }
    }

    pub fn parse_decl_list(&self) -> ParseResult<Vec<Declaration>> {
        let mut ret = Vec::new();
        let mut lexer = Token::lexer(self.code.as_str());

        while lexer.token != Token::End &&
            lexer.token != Token::Error {
            if lexer.token == Token::Fn {
                ret.push(self.parse_fn_decl(&mut lexer)?);
            }
            lexer.advance();
        }

        Ok(ret)
    }

    pub fn parse_statement_list(&self, lexer: &mut Lexer<Token, &str>) -> ParseResult<Vec<Statement>> {
        let mut ret = Vec::new();

        while lexer.token != Token::End &&
            lexer.token != Token::Error {
            match lexer.token {
                Token::Int => {
                    ret.push(self.parse_var_decl(lexer)?);
                },
                Token::Text => {
                    ret.push(self.parse_var_assign(lexer)?);
                },
                _ => {
                    return Err(ParseError::UnknownStatement);
                }
            };
            
        }

        Ok(ret)
    }

    pub fn parse_var_decl(&self, lexer: &mut Lexer<Token, &str>) -> ParseResult<Statement> {
        let mut lexer_backup = lexer.clone();

        let var_type = match lexer.token {
            Token::Int => {
                Type::Int
            },
            _ => {
                return Err(ParseError::UnknownType);
            }
        };

        lexer.advance();
        
        if lexer.token != Token::Text {
            *lexer = lexer_backup;
            return Err(ParseError::ExpectedVarName);
        }

        let var_name = String::from(lexer.slice());

        lexer.advance();

        if lexer.token != Token::Assign {
            *lexer = lexer_backup;
            return Err(ParseError::ExpectedAssignment);
        }

        lexer.advance();

        let mut lexer_backup = lexer.clone();
        let mut tokens = Vec::new();
        let mut token_vals = Vec::new();
        while lexer.token != Token::Semicolon {
            tokens.push(lexer.token.clone());
            token_vals.push(String::from(lexer.slice()));
            lexer.advance();
        }

        let expr = self.parse_expr(&tokens, &token_vals)?;

        let var_decl_args = VariableDeclArgs {
            var_type: var_type,
            name: var_name,
            assignment: Box::new(expr)
        };

        lexer.advance();

        Ok(
            Statement::VariableDecl(var_decl_args)
        )
    }

    pub fn parse_var_assign(&self, lexer: &mut Lexer<Token, &str>) -> ParseResult<Statement> {
        if lexer.token != Token::Text {
            return Err(ParseError::UnknownStatement);
        }

        let var_name = String::from(lexer.slice());
        lexer.advance();

        if lexer.token != Token::Assign {
            return Err(ParseError::ExpectedAssignment);
        }

        lexer.advance();

        let mut lexer_backup = lexer.clone();
        let mut tokens = Vec::new();
        let mut token_vals = Vec::new();
        while lexer.token != Token::Semicolon {
            tokens.push(lexer.token.clone());
            token_vals.push(String::from(lexer.slice()));
            lexer.advance();
        }

        let assign_expr = self.parse_expr(&tokens, &token_vals)?;

        lexer.advance();

        Ok(
            Statement::Assignment(var_name, Box::new(assign_expr))
        )
    }

    pub fn parse_expr(&self, tokens: &[Token], token_vals: &[String]) -> ParseResult<Expression> {
        let mut operator_stack = VecDeque::new();
        let mut operand_stack = VecDeque::new();

        'outer: for i in 0..tokens.len() {
            let token = &tokens[i];
            let token_val = &token_vals[i];

            match *token {
                Token::IntLiteral => {
                    let int = token_val.parse::<i64>()
                        .map_err(|_| ParseError::Unknown)?;
                    operand_stack.push_front(
                        Expression::IntLiteral(int)
                    );
                    continue;
                },
                Token::OpenParan => {
                    operator_stack.push_front(token.clone());
                },
                Token::CloseParan => {
                    while let Some(operator) = operator_stack.pop_front() {
                        if operator == Token::OpenParan {
                            continue 'outer;
                        } else {
                            match operator {
                                Token::Plus => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Addition(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                Token::Minus => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Subtraction(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                Token::Times => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Multiplication(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                Token::Divide => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Division(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                _ => {
                                    return Err(ParseError::UnsupportedExpression);
                                }
                            };
                        }
                    }
                    return Err(ParseError::UnsupportedExpression);
                },
                Token::Plus => {
                    let prec = op_prec(token);
                    while operator_stack.len() > 1 {
                        let next_op_ref = &operator_stack[1];
                        if *next_op_ref == Token::OpenParan {
                            break;
                        }
                        let next_prec = op_prec(next_op_ref);

                        if prec - next_prec <= 0 {
                            let next_op = operator_stack.pop_front().unwrap();
                            match next_op {
                                Token::Plus => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Addition(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                Token::Minus => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Subtraction(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                Token::Times => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Multiplication(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                Token::Divide => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Division(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                _ => {
                                    return Err(ParseError::UnsupportedExpression);
                                }
                            };
                        } else {
                            break;
                        }
                    }
                    operator_stack.push_front(token.clone());
                },
                Token::Minus => {
                    let prec = op_prec(token);
                    while operator_stack.len() > 1 {
                        let next_op_ref = &operator_stack[1];
                        let next_prec = op_prec(next_op_ref);

                        if prec - next_prec <= 0 {
                            let next_op = operator_stack.pop_front().unwrap();
                            match next_op {
                                Token::Plus => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Addition(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                Token::Minus => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Subtraction(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                Token::Times => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Multiplication(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                Token::Divide => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Division(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                _ => {
                                    return Err(ParseError::UnsupportedExpression);
                                }
                            };
                        } else {
                            break;
                        }
                    }
                    operator_stack.push_front(token.clone());
                },
                Token::Times => {
                    let prec = op_prec(token);
                    while operator_stack.len() > 1 {
                        let next_op_ref = &operator_stack[1];
                        let next_prec = op_prec(next_op_ref);

                        if prec - next_prec <= 0 {
                            let next_op = operator_stack.pop_front().unwrap();
                            match next_op {
                                Token::Plus => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Addition(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                Token::Minus => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Subtraction(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                Token::Times => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Multiplication(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                Token::Divide => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Division(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                _ => {
                                    return Err(ParseError::UnsupportedExpression);
                                }
                            };
                        } else {
                            break;
                        }
                    }
                    operator_stack.push_front(token.clone());
                },
                Token::Divide => {
                    let prec = op_prec(token);
                    while operator_stack.len() > 1 {
                        let next_op_ref = &operator_stack[1];
                        let next_prec = op_prec(next_op_ref);

                        if prec - next_prec <= 0 {
                            let next_op = operator_stack.pop_front().unwrap();
                            match next_op {
                                Token::Plus => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Addition(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                Token::Minus => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Subtraction(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                Token::Times => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Multiplication(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                Token::Divide => {
                                    let rhs = operand_stack.pop_front().unwrap();
                                    let lhs = operand_stack.pop_front().unwrap();
                                    let expr = Expression::Division(Box::new(lhs), Box::new(rhs));
                                    operand_stack.push_front(expr);
                                },
                                _ => {
                                    return Err(ParseError::UnsupportedExpression);
                                }
                            };
                        } else {
                            break;
                        }
                    }
                    operator_stack.push_front(token.clone());
                },
                _ => {
                    return Err(ParseError::UnsupportedExpression);
                }
            };
        }

        while let Some(operator) = operator_stack.pop_front() {
            match operator {
                Token::Plus => {
                    let rhs = operand_stack.pop_front().unwrap();
                    let lhs = operand_stack.pop_front().unwrap();
                    let expr = Expression::Addition(Box::new(lhs), Box::new(rhs));
                    operand_stack.push_front(expr);
                },
                Token::Minus => {
                    let rhs = operand_stack.pop_front().unwrap();
                    let lhs = operand_stack.pop_front().unwrap();
                    let expr = Expression::Subtraction(Box::new(lhs), Box::new(rhs));
                    operand_stack.push_front(expr);
                },
                Token::Times => {
                    let rhs = operand_stack.pop_front().unwrap();
                    let lhs = operand_stack.pop_front().unwrap();
                    let expr = Expression::Multiplication(Box::new(lhs), Box::new(rhs));
                    operand_stack.push_front(expr);
                },
                Token::Divide => {
                    let rhs = operand_stack.pop_front().unwrap();
                    let lhs = operand_stack.pop_front().unwrap();
                    let expr = Expression::Division(Box::new(lhs), Box::new(rhs));
                    operand_stack.push_front(expr);
                },
                _ => {
                    return Err(ParseError::UnsupportedExpression);
                }
            };
        }
        
        operand_stack.pop_front()
            .ok_or(ParseError::UnsupportedExpression)
    }

    pub fn parse_expr_old(&self, lexer: &mut Lexer<Token, &str>) -> ParseResult<Expression> {
        let mut token_list = Vec::new();

        match lexer.token {
            Token::IntLiteral => {
                let raw_int = String::from(lexer.slice());
                let int = raw_int.parse::<i64>()
                    .map_err(|_| ParseError::Unknown)?;
                return Ok(
                    Expression::IntLiteral(int)
                );
            },
            Token::OpenParan => {
                
            }
            _ => {
                return Err(ParseError::UnsupportedExpression);
            }
        };
        
        let mut lexer_backup = lexer.clone();
        while lexer.token != Token::Semicolon &&
            lexer.token != Token::Error &&
            lexer.token != Token::End {
            lexer_backup = lexer.clone();
            token_list.push(lexer.token.clone());
            lexer.advance();
        }
        *lexer = lexer_backup;

        if token_list.len() > 1 {
            if token_list[1] == Token::Plus {

            }
        }

        Err(
            ParseError::UnsupportedExpression
        )
    }

    pub fn parse_int_expr(&self, lexer: &mut Lexer<Token, &str>) -> ParseResult<Expression> {
        return Err(ParseError::Unknown);
    }

    pub fn parse_add_expr(&self, lexer: &mut Lexer<Token, &str>) -> ParseResult<Expression> {
        return Err(ParseError::Unknown);
    }

    pub fn parse_fn_decl(&self, lexer: &mut Lexer<Token, &str>) -> ParseResult<Declaration> {
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
        let fn_args = self.parse_fn_args(lexer)?;

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
            lexer.advance();
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
        let mut lexer = Token::lexer(code.as_str());
        let parser = Parser::new(code.clone());
        let decl_res = parser.parse_fn_decl(&mut lexer);

        assert!(decl_res.is_ok());

        let Declaration::Function(fn_decl) = decl_res.unwrap();

        assert_eq!(fn_decl.name, String::from("main"));
        assert_eq!(fn_decl.arguments.len(), 1);
        assert!(fn_decl.code_block.is_none());
    }

    #[test]
    fn test_full_fn_decl() {
        let code = String::from("fn main(int arg) ~ int {}");
        let mut lexer = Token::lexer(code.as_str());
        let parser = Parser::new(code.clone());
        let decl_res = parser.parse_fn_decl(&mut lexer);

        assert!(decl_res.is_ok());

        let Declaration::Function(fn_decl) = decl_res.unwrap();

        assert_eq!(fn_decl.name, String::from("main"));
        assert_eq!(fn_decl.arguments.len(), 1);
        assert!(fn_decl.code_block.is_some());
    }
    
    #[test]
    fn test_fn_mul_args() {
        let code = String::from("fn main21(int arg, int noarg) ~ int {}");
        let mut lexer = Token::lexer(code.as_str());
        let parser = Parser::new(code.clone());
        let decl_res = parser.parse_fn_decl(&mut lexer);

        assert!(decl_res.is_ok());

        let Declaration::Function(fn_decl) = decl_res.unwrap();

        assert_eq!(fn_decl.name, String::from("main21"));
        assert_eq!(fn_decl.arguments.len(), 2);
        assert!(fn_decl.code_block.is_some());
    }

    #[test]
    fn test_decl_list() {
        let code = String::from("
            fn main1(int argc) ~ int;
            fn test2(int none) ~ float {}
        ");
        let parser = Parser::new(code);

        let decl_list_res = parser.parse_decl_list();

        assert!(decl_list_res.is_ok());

        let decl_list = decl_list_res.unwrap();

        assert_eq!(decl_list.len(), 2);
    }

    #[test]
    fn test_stmt_list() {
        let code = String::from("
            int x = 4;
            int y = 6;
        ");

        let mut lexer = Token::lexer(code.as_str());
        let parser = Parser::new(code.clone());
        let stmt_list_res = parser.parse_statement_list(&mut lexer);

        assert!(stmt_list_res.is_ok());
        let stmt_list = stmt_list_res.unwrap();

        assert_eq!(stmt_list.len(), 2);
    }

    fn test_stmt_addition() {
        let code = String::from("
            int x = 0;
            x = 1 + 2 * 3 + 2;
        ");

        let mut lexer = Token::lexer(code.as_str());
        let parser = Parser::new(code.clone());
        let stmt_list_res = parser.parse_statement_list(&mut lexer);

        assert!(stmt_list_res.is_ok());
        let stmt_list = stmt_list_res.unwrap();

        assert_eq!(stmt_list.len(), 2);

        println!("{:?}", stmt_list);
    }

    #[test]
    fn test_raw_expr() {
        let code = String::from("
            (1 + 2 + 3) * 7 - 8 + 3;
        ");
        let mut lexer = Token::lexer(code.as_str());
        let parser = Parser::new(code.clone());

        let mut tokens = Vec::new();
        let mut token_vals = Vec::new();
        while lexer.token != Token::Semicolon {
            tokens.push(lexer.token.clone());
            token_vals.push(String::from(lexer.slice()));
            lexer.advance();
        }

        let expr_res = parser.parse_expr(&tokens, &token_vals);
        assert!(expr_res.is_ok());
        let expr = expr_res.unwrap();
        expr.print(0);
    }
}
