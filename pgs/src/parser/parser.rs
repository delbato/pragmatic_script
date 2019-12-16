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
    ExpectedColon,
    ExpectedOpenParan,
    ExpectedCloseParan,
    ExpectedStructName,
    ExpectedModName,
    ExpectedOpenBlock,
    ExpectedMemberType,
    ExpectedMemberName,
    DuplicateMember,
    ExpectedImport,
    ExpectedImportString
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
            if lexer.token == Token::Struct {
                ret.push(self.parse_struct_decl(&mut lexer)?);
            }
            if lexer.token == Token::Import {
                ret.push(self.parse_import_decl(&mut lexer)?);
            }
            lexer.advance();
        }

        Ok(ret)
    }

    pub fn parse_import_decl(&self, lexer: &mut Lexer) -> ParseResult<Declaration> {
        if lexer.token != Token::Import {
            return Err(ParseError::ExpectedImport);
        }

        // Swallow "import"
        lexer.advance();

        let delims = &[
            Token::Semicolon,
            Token::Assign,
            Token::End,
            Token::Error
        ];

        let mut import_string = String::new();
        let mut import_string_end = String::new();

        while !delims.contains(&lexer.token) {
            if lexer.token != Token::Text {
                return Err(ParseError::ExpectedImportString);
            }

            import_string += lexer.slice();
            import_string_end = String::from(lexer.slice());
            // Swallow the name
            lexer.advance();

            if lexer.token != Token::DoubleColon {
                break;
            }

            import_string += "::";

            // Swalow "::"
            lexer.advance();
        }
        let mut import_as = import_string_end;
        if lexer.token == Token::Assign {
            // Swallow "="
            lexer.advance();

            if lexer.token != Token::Text {
                return Err(ParseError::ExpectedImportString);
            }

            import_as = String::from(lexer.slice());
            // Swallow import name
            lexer.advance();
        }

        if lexer.token != Token::Semicolon {
            return Err(ParseError::ExpectedSemicolon);
        }

        // Swallow ";"
        lexer.advance();

        Ok(
            Declaration::Import(import_string, import_as)
        )
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
                let statements = self.parse_statement_list(lexer)?;
                code_block_opt = Some(statements);
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
        
        while lexer.token != Token::CloseParan &&
            lexer.token != Token::End &&
            lexer.token != Token::Error {
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

            lexer.advance();
            if lexer.token != Token::Comma {
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

    pub fn parse_struct_decl(&self, lexer: &mut Lexer) -> ParseResult<Declaration> {
        if lexer.token != Token::Struct {
            return Err(ParseError::Unknown);
        }

        // Swallow "struct"
        lexer.advance();

        if lexer.token != Token::Colon {
            return Err(ParseError::ExpectedColon);
        }

        // Swallow ":"
        lexer.advance();

        if lexer.token != Token::Text {
            return Err(ParseError::ExpectedStructName);
        }

        let struct_name = String::from(lexer.slice());

        // Swallow struct name
        lexer.advance();

        if lexer.token != Token::OpenBlock {
            return Err(ParseError::ExpectedOpenBlock);
        }

        // Swallow "{"
        lexer.advance();

        let members = self.parse_struct_members(lexer)?;

        // Swallow "}"
        lexer.advance();

        let struct_args = StructDeclArgs {
            name: struct_name,
            members: members
        };

        Ok(
            Declaration::Struct(struct_args)
        )
    }

    pub fn parse_struct_members(&self, lexer: &mut Lexer) -> ParseResult<BTreeMap<usize, (String, Type)>> {
        let mut ret = BTreeMap::new();
        let mut members = HashSet::new();
        let mut member_index = 0;
        while lexer.token != Token::CloseBlock &&
            lexer.token != Token::End &&
            lexer.token != Token::Error {
            
            let member = self.parse_struct_member(lexer)?;
            if members.contains(&member.0) {
                return Err(ParseError::DuplicateMember);
            }
            members.insert(member.0.clone());
            ret.insert(member_index, member);
            member_index += 1;
        }

        Ok(ret)
    }

    pub fn parse_struct_member(&self, lexer: &mut Lexer) -> ParseResult<(String, Type)> {
        if lexer.token != Token::Text {
            return Err(ParseError::ExpectedMemberName);
        }

        let mut member_name = String::from(lexer.slice());
        // Swallow member name
        lexer.advance();

        if member_name.starts_with("i") ||
            member_name.starts_with("b") ||
            member_name.starts_with("s") ||
            member_name.starts_with("f") {
            member_name += lexer.slice();
            lexer.advance(); // Workaround for bug in Logos
        }

        if lexer.token != Token::Colon {
            return Err(ParseError::ExpectedColon);
        }

        // Swallow ":"
        lexer.advance();

        let member_type = match lexer.token {
            Token::Int => Type::Int,
            Token::Float => Type::Float,
            Token::String => Type::String,
            Token::Bool => Type::Bool,
            Token::Text => {
                let type_name = String::from(lexer.slice());
                Type::Other(type_name)
            },
            _ => return Err(ParseError::ExpectedMemberType)
        };

        // Swallow member type
        lexer.advance();

        if lexer.token != Token::Semicolon {
            return Err(ParseError::ExpectedSemicolon);
        }

        // Swallow ";"
        lexer.advance();

        Ok(
            (member_name, member_type)
        )
    }

    pub fn parse_statement_list(&self, lexer: &mut Lexer) -> ParseResult<Vec<Statement>> {
        let mut ret = Vec::new();

        while lexer.token != Token::CloseBlock &&
            lexer.token != Token::End &&
            lexer.token != Token::Error {
            match lexer.token {
                Token::Var => {
                    ret.push(self.parse_var_decl(lexer)?);
                },
                Token::Text => {
                    ret.push(self.parse_var_assign(lexer)?);
                },
                Token::Return => {
                    ret.push(self.parse_return(lexer)?);
                },
                _ => {
                    return Err(ParseError::UnknownStatement);
                }
            };
            
        }

        Ok(ret)
    }

    pub fn parse_return(&self, lexer: &mut Lexer) -> ParseResult<Statement> {
        // Swallow "return"
        lexer.advance();

        let ret_expr = self.parse_expr(lexer, &[Token::Semicolon])?;

        // Swallow ";"
        lexer.advance();

        Ok(
            Statement::Return(Box::new(ret_expr))
        )
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

        let expr = self.parse_expr(lexer, &[Token::Semicolon])?;

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

        let assign_expr = self.parse_expr(lexer, &[Token::Semicolon])?;

        lexer.advance();

        Ok(
            Statement::Assignment(var_name, Box::new(assign_expr))
        )
    }

    pub fn parse_fn_call_stmt(&self, lexer: &mut Lexer) -> ParseResult<Statement> {
        if lexer.token != Token::Text {
            return Err(ParseError::ExpectedFunctionName);
        }

        let fn_name = String::from(lexer.slice());
        // Swallow fn name
        lexer.advance();

        if lexer.token != Token::OpenParan {
            return Err(ParseError::ExpectedOpenParan);
        }

        // Swallow "("
        lexer.advance();

        let mut params = Vec::new();

        while lexer.token != Token::CloseParan &&
            lexer.token != Token::End &&
            lexer.token != Token::Error {
            let arg = self.parse_expr(lexer, &[
                Token::Comma,
                Token::CloseParan
            ])?;
            if lexer.token == Token::Comma {
                lexer.advance(); // Swallow "," if its there
            }
            params.push(arg);
        }

        // Swallow ")"
        lexer.advance();

        if lexer.token != Token::Semicolon {
            return Err(ParseError::ExpectedSemicolon);
        }
        // Swallow ";"
        lexer.advance();

        Ok(
            Statement::Call(fn_name, params)
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

    pub fn try_parse_call_expr(&self, lexer: &mut Lexer) -> ParseResult<Expression> {
        let lexer_backup = lexer.clone(); // Create lexer backup for backtracking

        if lexer.token != Token::Text {
            return Err(ParseError::ExpectedFunctionName);
        }

        let fn_name = String::from(lexer.slice());
        // Swallow fn name
        lexer.advance();

        if lexer.token != Token::OpenParan {
            *lexer = lexer_backup;
            return Err(ParseError::ExpectedOpenParan);
        }

        // Swallow "("
        lexer.advance();

        let mut params = Vec::new();

        while lexer.token != Token::CloseParan &&
            lexer.token != Token::End &&
            lexer.token != Token::Error {
            let arg = self.parse_expr(lexer, &[
                Token::Comma,
                Token::CloseParan
            ])?;
            if lexer.token == Token::Comma {
                lexer.advance(); // Swallow "," if its there
            }
            params.push(arg);
        }

        // Swallow ")"
        lexer.advance();

        Ok(
            Expression::Call(fn_name, params)
        )
    }

    pub fn parse_expr(&self, lexer: &mut Lexer, delims: &[Token]) -> ParseResult<Expression> {
        let mut operator_stack = VecDeque::new();
        let mut operand_stack = VecDeque::new();

        // Counter for handling ")" being used as delim
        let mut open_paran_count = 0;

        while lexer.token != Token::End &&
            lexer.token != Token::Error {

            // If Token is delimiter
            if delims.contains(&lexer.token) {
                // Special case if ")" is a delimiter
                if lexer.token == Token::CloseParan && open_paran_count == 0 {
                    break;
                } else if lexer.token != Token::CloseParan {
                    break; // Break if delim is hit
                }
            }
            
            if lexer.token == Token::Text {
                let mut expr;
                let call_expr_res = self.try_parse_call_expr(lexer);
                if call_expr_res.is_ok() {
                    expr = call_expr_res.unwrap();
                } else {
                    let var_name = String::from(lexer.slice());
                    expr = Expression::Variable(var_name);
                }
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
                open_paran_count += 1;
            }

            if lexer.token == Token::CloseParan {
                let mut pop = false;               
                while operator_stack.len() > 0 {
                    {
                        let op_ref = operator_stack.get(0).unwrap();
                        if *op_ref == Token::OpenParan {
                            open_paran_count -= 1;
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
}
