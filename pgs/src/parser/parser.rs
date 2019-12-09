use super::{
    ast::{
        *
    },
    lexer::{
        Token,
        Lexer
    }
};

use std::{
    collections::{
        HashMap,
        VecDeque,
        HashSet,
        BTreeMap
    },
    fmt::{
        Debug
    }
};

use logos::{
    Logos
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
    UnsupportedExpression,
    ExpectedColon
}

pub type ParseResult<T> = Result<T, ParseError>;

pub struct Parser {
    code: String,
}

fn is_op(token: &Token) -> bool {
    match token {
        Token::Times => true,
        Token::Divide => true,
        Token::Plus => true,
        Token::Minus => true,
        _ => false
    }
}

fn op_prec(token: &Token) -> i8 {
    match token {
        Token::Times => 2,
        Token::Divide => 2,
        Token::Plus => 0,
        Token::Minus => 0,
        _ => {
            panic!("ERROR! Not an operator");
        }
    }
}

fn is_op_right_assoc(token: &Token) -> bool {
    match token {
        Token::Times => true,
        Token::Divide => false,
        Token::Plus => false,
        Token::Minus => false,
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

    pub fn parse_statement_list(&self, lexer: &mut Lexer) -> ParseResult<Vec<Statement>> {
        let mut ret = Vec::new();

        while lexer.token != Token::End &&
            lexer.token != Token::Error {
            match lexer.token {
                Token::Var => {
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

    pub fn parse_var_decl(&self, lexer: &mut Lexer) -> ParseResult<Statement> {
        let mut lexer_backup = lexer.clone();

        // Swallow "var"
        lexer.advance();

        // Parse ":"
        if lexer.token != Token::Colon {
            return Err(ParseError::ExpectedColon);
        }
        lexer.advance();

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

        let expr = self.parse_expr(lexer)?;

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

    pub fn parse_var_assign(&self, lexer: &mut Lexer) -> ParseResult<Statement> {
        if lexer.token != Token::Text {
            return Err(ParseError::UnknownStatement);
        }

        let var_name = String::from(lexer.slice());
        lexer.advance();

        if lexer.token != Token::Assign {
            return Err(ParseError::ExpectedAssignment);
        }

        lexer.advance();

        let assign_expr = self.parse_expr(lexer)?;

        lexer.advance();

        Ok(
            Statement::Assignment(var_name, Box::new(assign_expr))
        )
    }

    pub fn parse_expr_push(&self, operand_stack: &mut VecDeque<Expression>, operator_stack: &mut VecDeque<Token>) -> ParseResult<Expression> {
        let op = operator_stack.pop_front().unwrap();
        let expr = match op {
            Token::Plus => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::Addition(Box::new(lhs), Box::new(rhs))
            },
            Token::Minus => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::Subtraction(Box::new(lhs), Box::new(rhs))
            },
            Token::Times => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::Multiplication(Box::new(lhs), Box::new(rhs))
            },
            Token::Divide => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::Division(Box::new(lhs), Box::new(rhs))
            },
            _ => {
                return Err(ParseError::UnsupportedExpression);
            }
        };
        Ok(expr)
    }

    pub fn parse_expr(&self, lexer: &mut Lexer) -> ParseResult<Expression> {
        let mut operator_stack = VecDeque::new();
        let mut operand_stack = VecDeque::new();

        while lexer.token != Token::Semicolon &&
            lexer.token != Token::End &&
            lexer.token != Token::Error {
            
            if lexer.token == Token::Text {
                let expr = Expression::Variable(String::from(lexer.slice()));
                operand_stack.push_front(expr);
            }

            if lexer.token == Token::IntLiteral {
                let int = String::from(lexer.slice()).parse::<i64>()
                    .map_err(|_| ParseError::Unknown)?;
                let expr = Expression::IntLiteral(int);
                operand_stack.push_front(expr);
            }

            if is_op(&lexer.token) {
                loop {
                    let op_opt = operator_stack.get(0);
                    if op_opt.is_none() {
                        break; // Break if operator stack is empty
                    }
                    let op = op_opt.unwrap();
                    if *op == Token::OpenParan {
                        break; // Break if operator is a "("
                    }

                    if !(op_prec(&lexer.token) - op_prec(op) < 0) &&
                        !(op_prec(&lexer.token) == op_prec(op) && !is_op_right_assoc(op)) {
                        break; // Break if there is no operator of greater precedence on the stack or of equal precedence and right assoc
                    }

                    let expr = self.parse_expr_push(&mut operand_stack, &mut operator_stack)?;
                    operand_stack.push_front(expr);
                }
                operator_stack.push_front(lexer.token.clone());
            }

            if lexer.token == Token::OpenParan {
                operator_stack.push_front(lexer.token.clone());
            }

            if lexer.token == Token::CloseParan {
                let mut pop = false;                
                while operator_stack.len() > 0 {
                    {
                        let op_ref = operator_stack.get(0).unwrap();
                        if *op_ref == Token::OpenParan {
                            pop = true;
                            break;
                        }
                    }
                    let expr = self.parse_expr_push(&mut operand_stack, &mut operator_stack)?;
                    operand_stack.push_front(expr);
                }

                if pop {
                    operator_stack.pop_front();
                }
            }
            lexer.advance();
        }

        while operator_stack.len() > 0 {
            let expr = self.parse_expr_push(&mut operand_stack, &mut operator_stack)?;
            operand_stack.push_front(expr);
        }

        operand_stack.pop_front()
            .ok_or(ParseError::UnsupportedExpression)
    }

    pub fn parse_fn_decl(&self, lexer: &mut Lexer) -> ParseResult<Declaration> {
        let mut fn_decl_opt = None;

        // Parse "fn" literal
        if lexer.token != Token::Fn {
            return Err(ParseError::FnMissing);
        }
        lexer.advance();

        // Parse ":"
        if lexer.token != Token::Colon {
            return Err(ParseError::ExpectedColon);
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

    pub fn parse_fn_args(&self, lexer: &mut Lexer) -> ParseResult<BTreeMap<usize, (String, Type)>> {
        let mut ret = BTreeMap::new();
        let mut fn_arg_set = HashSet::new();

        let mut arg_index = 0;
        
        loop {
            let fn_arg_res = self.parse_fn_arg(lexer);
            if fn_arg_res.is_err() {
                break;
            }
            let fn_arg = fn_arg_res.unwrap();
            if fn_arg_set.contains(&fn_arg.0) {
                return Err(ParseError::DuplicateArg);
            }
            fn_arg_set.insert(fn_arg.0.clone());

            ret.insert(arg_index, fn_arg);

            let lexer_backup = lexer.clone();
            lexer.advance();
            if lexer.token != Token::Comma {
                *lexer = lexer_backup;
                break;
            }
            arg_index += 1;
            lexer.advance();
        }

        Ok(ret)
    }

    pub fn parse_fn_arg(&self, lexer: &mut Lexer) -> ParseResult<(String, Type)> {
        let mut lexer_backup = lexer.clone();
        if lexer.token != Token::Text {
            return Err(ParseError::ExpectedArgName);
        }
        let arg_name = String::from(lexer.slice());
        lexer.advance();

        // Parse ":"
        if lexer.token != Token::Colon {
            return Err(ParseError::ExpectedColon);
        }
        lexer.advance();


        let arg_type = match lexer.token {
            Token::Int => Type::Int,
            Token::Float => Type::Float,
            Token::String => Type::String,
            _ => {
                *lexer = lexer_backup;
                return Err(ParseError::ExpectedArgType);
            }
        };

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
        let code = String::from("fn: main(arg: int) ~ int;");
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
        let code = String::from("fn: main(arg: int) ~ int {}");
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
        let code = String::from("fn: main21(arg: int, noarg: int) ~ int {}");
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
            fn: main1(argc: int) ~ int;
            fn: test2(noint: float) ~ float {}
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
            var:int x = 4;
            var:int y = 6;
        ");

        let mut lexer = Token::lexer(code.as_str());
        let parser = Parser::new(code.clone());
        let stmt_list_res = parser.parse_statement_list(&mut lexer);

        assert!(stmt_list_res.is_ok());
        let stmt_list = stmt_list_res.unwrap();

        assert_eq!(stmt_list.len(), 2);
    }

    #[test]
    fn test_stmt_addition() {
        let code = String::from("
            var:int x = 4;
            y = 1 + 2 * 3 + x;
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

        let expr_res = parser.parse_expr(&mut lexer);
        assert!(expr_res.is_ok());
        let expr = expr_res.unwrap();
        expr.print(0);
    }
    
    #[test]
    fn test_raw_var_expr() {
        let code = String::from("
            (1 + z + 3) * x - 8 + y;
        ");
        let mut lexer = Token::lexer(code.as_str());
        let parser = Parser::new(code.clone());
        let expr_res = parser.parse_expr(&mut lexer);
        assert!(expr_res.is_ok());
        let expr = expr_res.unwrap();
        //expr.print(0);
    }
}
