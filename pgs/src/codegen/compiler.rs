use crate::{
    parser::{
        ast::*
    },
    vm::{
        is::Opcode
    }
};
use super::{
    builder::{
        Builder
    },
    checker::Checker,
    instruction::Instruction,
    context::{
        Context
    },
    program::Program
};

use std::{
    collections::{
        VecDeque,
        HashMap,
        HashSet
    }
};

use rand::{
    Rng,
    RngCore,
    thread_rng
};

pub struct Compiler {
    global_context: Context,
    context_stack: VecDeque<Context>,
    builder: Builder,
    function_uid_map: HashMap<String, u64>,
    function_uid_set: HashSet<u64>
}

pub type CompilerResult<T> = Result<T, CompilerError>;

#[derive(Debug)]
pub enum CompilerError {
    Unknown,
    UnknownType,
    UnknownFunction,
    NotImplemented,
    UnknownVariable,
    TypeMismatch
}

impl Compiler {
    pub fn new() -> Compiler {
        let comp = Compiler {
            global_context: Context::new(),
            context_stack: VecDeque::new(),
            builder: Builder::new(),
            function_uid_map: HashMap::new(),
            function_uid_set: HashSet::new()
        };
        comp
    }

    pub fn push_context(&mut self) {
        let stack_size = {
            let front_context_opt = self.context_stack.get(0);
            if front_context_opt.is_some() {
                front_context_opt.unwrap().stack_size
            } else {
                0
            }
        };
        let mut context = Context::new();
        context.stack_size = stack_size;
        self.context_stack.push_front(context);
    }

    pub fn get_context(&mut self) -> Option<Context> {
        self.context_stack.get(0).cloned()
    }

    pub fn push_new_context(&mut self, context: Context) {
        self.context_stack.push_front(context);
    }

    pub fn push_empty_context(&mut self) {
        self.context_stack.push_front(Context::new());
    }

    pub fn reset_global(&mut self) {
        self.global_context = Context::new();
    }

    pub fn get_function_uid(&mut self, function_name: String) -> u64 {
        let opt = self.function_uid_map.get(&function_name);
        println!("Getting UUID for function {}...", function_name);
        if opt.is_some() {
            println!("UUID exists!");
            opt.unwrap().clone()
        } else {
            println!("UUID does not exist, generating new one...");
            let mut rng = thread_rng();
            let mut uid = rng.next_u64();
            while self.function_uid_set.contains(&uid) {
                uid = rng.next_u64();
            }
            println!("UUID: {}", uid);
            self.function_uid_set.insert(uid.clone());
            self.function_uid_map.insert(function_name, uid.clone());
            uid
        }
    }

    pub fn reset_builder(&mut self) {
        self.builder = Builder::new();
    }

    pub fn compile_decl_list(&mut self, decl_list: Vec<Declaration>) -> CompilerResult<()> {
        self.context_stack.push_front(Context::new());
        for decl in decl_list {
            self.compile_decl(decl)?;
        }
        Ok(())
    }

    pub fn compile_decl(&mut self, decl: Declaration) -> CompilerResult<()> {
        match decl {
            Declaration::Function(_) => {
                self.compile_fn_decl(decl)?;
            },
            _ => {
                return Err(CompilerError::Unknown);
            }
        };
        Ok(())
    }

    pub fn compile_fn_decl(&mut self, fn_decl: Declaration) -> CompilerResult<()> {
        let Declaration::Function(fn_decl_args) = fn_decl;
        self.builder.push_label(fn_decl_args.name.clone());
        let _ = self.get_function_uid(fn_decl_args.name.clone());
        {
            let mut front_context = self.context_stack.get_mut(0)
            .ok_or(CompilerError::Unknown)?;

            front_context.functions.insert(fn_decl_args.name.clone(), fn_decl_args.returns.clone());
        }

        let mut context = Context::new();

        let mut stack_index = 0;
        for (_, (var_name, var_type)) in fn_decl_args.arguments.iter().rev() {
            let size = self.size_of_type(var_type)?;
            context.set_var(stack_index - size as i64, (var_name.clone(), var_type.clone()));
            stack_index -= size as i64;
        }

        context.return_type = Some(fn_decl_args.returns);

        self.context_stack.push_front(context);

        if let Some(statements) = fn_decl_args.code_block {
            for statement in statements {
                self.compile_statement(statement)?;
            }
        }

        self.context_stack.pop_front();

        Ok(())
    }

    pub fn compile_statement(&mut self, stmt: Statement) -> CompilerResult<()> {
        match stmt {
            Statement::VariableDecl(_) => {
                self.compile_var_decl(stmt)?
            },
            Statement::Assignment(_, _) => {
                self.compile_var_assign(stmt)?
            },
            Statement::Return(_) => {
                self.compile_return(stmt)?
            },
            _ => {
                return Err(CompilerError::NotImplemented);
            }
        };

        Ok(())
    }
    
    pub fn compile_return(&mut self, stmt: Statement) -> CompilerResult<()> {
        let ret_expr = match stmt {
            Statement::Return(expr) => expr,
            _ => return Err(CompilerError::Unknown)
        };

        let checker = Checker::new(&self);
        let expr_type = checker.check_expr_type(&ret_expr)
            .map_err(|_| CompilerError::TypeMismatch)?;

        let fn_type = self.context_stack.get(0)
            .ok_or(CompilerError::Unknown)?
            .return_type
            .as_ref()
            .ok_or(CompilerError::TypeMismatch)?
            .clone();
        
        if fn_type != expr_type {
            return Err(CompilerError::TypeMismatch);
        }

        self.compile_expr(&ret_expr)?;

        // Save return value to swap space
        let sv_swap_instr = match fn_type {
            Type::Int => {
                Instruction::new(Opcode::SVSWPI)
            },
            Type::Bool => {
                Instruction::new(Opcode::SVSWPB)
            },
            Type::Float => {
                Instruction::new(Opcode::SVSWPF)
            },
            _ => {
                return Err(CompilerError::Unknown);
            }
        };

        let size = self.size_of_type(&fn_type)?;
        let stack_size = {
            let front_context = self.context_stack.get_mut(0)
                .ok_or(CompilerError::Unknown)?;
            front_context.stack_size -= size;
            front_context.stack_size
        };
        
        // Pop everything off the stack
        let popn_instr = Instruction::new(Opcode::POPN)
            .with_operand::<u64>(&(stack_size as u64));

        // Load return value from swap space
        let ld_swap_instr = match fn_type {
            Type::Int => {
                Instruction::new(Opcode::LDSWPI)
            },
            Type::Bool => {
                Instruction::new(Opcode::LDSWPB)
            },
            Type::Float => {
                Instruction::new(Opcode::LDSWPF)
            },
            _ => {
                return Err(CompilerError::Unknown);
            }
        };

        self.builder.push_instr(sv_swap_instr);
        self.builder.push_instr(popn_instr);
        self.builder.push_instr(ld_swap_instr);

        Ok(())
    }

    pub fn compile_var_decl(&mut self, stmt: Statement) -> CompilerResult<()> {
        let var_decl_args = match stmt {
            Statement::VariableDecl(args) => args,
            _ => return Err(CompilerError::Unknown)
        };

        let size = self.size_of_type(&var_decl_args.var_type)?;
        let var_type = var_decl_args.var_type.clone();
        // Insert variable to context
        {
            let front_context = self.context_stack.get_mut(0)
                .ok_or(CompilerError::Unknown)?;
            front_context.push_var((var_decl_args.name.clone(), var_type.clone()));
        }

        let checker = Checker::new(&self);
        let expr_type = checker.check_expr_type(&var_decl_args.assignment)
            .map_err(|_| CompilerError::TypeMismatch)?;

        if expr_type != var_type {
            return Err(CompilerError::TypeMismatch);
        }

        self.compile_expr(&var_decl_args.assignment)?;

        Ok(())
    }

    pub fn compile_var_assign(&mut self, stmt: Statement) -> CompilerResult<()> {
        let (var_name, expr) = match stmt {
            Statement::Assignment(name, assign) => (name, assign),
            _ => return Err(CompilerError::Unknown)
        };

        let var_type = self.type_of_var(var_name.clone())?;
        let checker = Checker::new(&self);
        let expr_type = checker.check_expr_type(&expr)
            .map_err(|_| CompilerError::TypeMismatch)?;

        if expr_type != var_type {
            return Err(CompilerError::TypeMismatch);
        }

        self.compile_expr(&expr)?;
        
        let var_offset = {
            let front_context = self.context_stack.get(0)
                .ok_or(CompilerError::Unknown)?;
            front_context.offset_of(&var_name)
                .ok_or(CompilerError::UnknownVariable)?
        };

        println!("Var offset for MOVI is: {}", var_offset);

        let mov_instr = match var_type {
            Type::Int => {
                let front_context = self.context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size -= 8;
                Instruction::new(Opcode::MOVI)
                    .with_operand(&var_offset)
            },
            _ => {
                return Err(CompilerError::NotImplemented);
            }
        };

        self.builder.push_instr(mov_instr);

        Ok(())
    }

    pub fn compile_expr(&mut self, expr: &Expression) -> CompilerResult<()> {
        match expr {
            Expression::IntLiteral(int) => {
                let pushi_instr = Instruction::new(Opcode::PUSHI)
                    .with_operand(int);
                self.builder.push_instr(pushi_instr);
                let front_context = self.context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size += 8;
            },
            Expression::FloatLiteral(float) => {
                return Err(CompilerError::NotImplemented);
            },
            Expression::Variable(var_name) => {      
                let var_offset = {
                    let front_context = self.context_stack.get(0)
                        .ok_or(CompilerError::Unknown)?;
                    front_context.offset_of(var_name)
                        .ok_or(CompilerError::UnknownVariable)?
                };
                let dupi_instr = Instruction::new(Opcode::DUPI)
                    .with_operand(&var_offset);
                self.builder.push_instr(dupi_instr);
                let front_context = self.context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size += 8;
            },
            Expression::Addition(lhs, rhs) => {
                self.compile_expr(&lhs)?;
                self.compile_expr(&rhs)?;
                let addi_instr = Instruction::new(Opcode::ADDI);
                self.builder.push_instr(addi_instr);
                let front_context = self.context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size -= 16;
                front_context.stack_size += 8;
            },
            Expression::Subtraction(lhs, rhs) => {
                self.compile_expr(&lhs)?;
                self.compile_expr(&rhs)?;
                let subi_instr = Instruction::new(Opcode::SUBI);
                self.builder.push_instr(subi_instr);
                let front_context = self.context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size -= 16;
                front_context.stack_size += 8;
            },
            Expression::Multiplication(lhs, rhs) => {
                self.compile_expr(&lhs)?;
                self.compile_expr(&rhs)?;
                let muli_instr = Instruction::new(Opcode::MULI);
                self.builder.push_instr(muli_instr);
                let front_context = self.context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size -= 16;
                front_context.stack_size += 8;
            },
            Expression::Division(lhs, rhs) => {
                self.compile_expr(&lhs)?;
                self.compile_expr(&rhs)?;
                let divi_instr = Instruction::new(Opcode::DIVI);
                self.builder.push_instr(divi_instr);
                let front_context = self.context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size -= 16;
                front_context.stack_size += 8;
            },
            _ => return Err(CompilerError::NotImplemented)
        };
        Ok(())
    }

    pub fn size_of_type(&self, var_type: &Type) -> CompilerResult<usize> {
        let size = match var_type {
            Type::Int => 8,
            Type::Float => 4,
            Type::String => 8,
            Type::Bool => 1,
            _ => {
                return Err(CompilerError::UnknownType);
            }
        };
        Ok(size)
    }

    pub fn type_of_var(&self, var_name: String) -> CompilerResult<Type> {
        let front_context = self.context_stack.get(0)
            .ok_or(CompilerError::UnknownVariable)?;
        let var_type = front_context.variable_types.get(&var_name)
            .ok_or(CompilerError::UnknownVariable)?;
        Ok(var_type.clone())
    }

    pub fn get_resulting_code(&mut self) -> Vec<u8> {
        let builder = self.builder.clone();
        builder.build()
    }

    pub fn get_program(&mut self) -> CompilerResult<Program> {
        println!("Getting program...");
        let mut builder = self.builder.clone();
        let mut functions = HashMap::new();

        for (fn_name, fn_uid) in self.function_uid_map.iter() {
            println!("Found fn {}, uid:{}", fn_name, fn_uid);
            let fn_offset = builder.get_label_offset(fn_name)
                .ok_or(CompilerError::UnknownFunction)?;
            println!("Offset: {}", fn_offset);
            functions.insert(*fn_uid, fn_offset);
        }

        println!("Instructions:");

        for instr in builder.instructions.iter() {
            println!("{:?}", instr);
        }

        let code = builder.build();

        let program = Program::new()
            .with_code(code)
            .with_functions(functions);

        Ok(program)
    }
}


#[cfg(test)]
mod test {
    use crate::{
        parser::{
            lexer::Token,
            parser::Parser,
            ast::Type
        },
        vm::{
            is::Opcode            
        },
        codegen::{
            instruction::Instruction,
            builder::Builder,
            context::Context,
            program::Program
        }
    };
    use super::{
        Compiler
    };

    use std::collections::HashMap;

    use logos::Logos;

    #[test]
    fn test_compile_addi() {
        let code = String::from("
            var:int x = 4;
            var:int y = x + 4;
        ");

        let mut lexer = Token::lexer(code.as_str());
        let parser = Parser::new(code.clone());
        let stmt_list_res = parser.parse_statement_list(&mut lexer);

        assert!(stmt_list_res.is_ok());
        let stmt_list = stmt_list_res.unwrap();

        let mut compiler = Compiler::new();
        compiler.reset_builder();
        compiler.push_empty_context();

        for stmt in stmt_list {
            let cmp_res = compiler.compile_statement(stmt);
            assert!(cmp_res.is_ok());
        }

        let mut comp_builder = Builder::new();

        let pushi_instr = Instruction::new(Opcode::PUSHI)
            .with_operand::<i64>(&4);
        let dupi_instr = Instruction::new(Opcode::DUPI)
            .with_operand::<i64>(&-8);
        let pushi2_instr = Instruction::new(Opcode::PUSHI)
            .with_operand::<i64>(&4);
        let addi_instr = Instruction::new(Opcode::ADDI);

        comp_builder.push_instr(pushi_instr);
        comp_builder.push_instr(dupi_instr);
        comp_builder.push_instr(pushi2_instr);
        comp_builder.push_instr(addi_instr);

        let comp_code = comp_builder.build();
        let code = compiler.get_resulting_code();

        assert_eq!(comp_code, code);
    }

    #[test]
    fn test_compile_addi_assign() {
        let code = String::from("
            var:int x = 4;
            x = x + 4;
        ");

        let mut lexer = Token::lexer(code.as_str());
        let parser = Parser::new(code.clone());
        let stmt_list_res = parser.parse_statement_list(&mut lexer);

        assert!(stmt_list_res.is_ok());
        let stmt_list = stmt_list_res.unwrap();

        let mut compiler = Compiler::new();
        compiler.reset_builder();
        compiler.push_empty_context();

        for stmt in stmt_list {
            let cmp_res = compiler.compile_statement(stmt);
            assert!(cmp_res.is_ok());
        }

        let mut comp_builder = Builder::new();

        let pushi_instr = Instruction::new(Opcode::PUSHI)
            .with_operand::<i64>(&4);
        let dupi_instr = Instruction::new(Opcode::DUPI)
            .with_operand::<i64>(&-8);
        let pushi2_instr = Instruction::new(Opcode::PUSHI)
            .with_operand::<i64>(&4);
        let addi_instr = Instruction::new(Opcode::ADDI);
        let movi_instr = Instruction::new(Opcode::MOVI)
            .with_operand::<i64>(&-16);

        comp_builder.push_instr(pushi_instr);
        comp_builder.push_instr(dupi_instr);
        comp_builder.push_instr(pushi2_instr);
        comp_builder.push_instr(addi_instr);
        comp_builder.push_instr(movi_instr);

        let comp_code = comp_builder.build();
        let code = compiler.get_resulting_code();

        assert_eq!(comp_code, code);
    }

    #[test]
    fn test_compile_muli_assign() {
        let code = String::from("
            var:int x = 4;
            x = x + 4;
            var:int z = x * 2;
            x = z;
            var:int w = 4;
            x = w;
        ");

        let mut lexer = Token::lexer(code.as_str());
        let parser = Parser::new(code.clone());
        let stmt_list_res = parser.parse_statement_list(&mut lexer);

        assert!(stmt_list_res.is_ok());
        let stmt_list = stmt_list_res.unwrap();

        let mut compiler = Compiler::new();
        compiler.reset_builder();
        compiler.push_empty_context();

        for stmt in stmt_list {
            let cmp_res = compiler.compile_statement(stmt);
            assert!(cmp_res.is_ok());
        }

        let mut comp_builder = Builder::new();

        let pushi_instr = Instruction::new(Opcode::PUSHI) // 4
            .with_operand::<i64>(&4);
        let dupi_instr = Instruction::new(Opcode::DUPI) // 4,4
            .with_operand::<i64>(&-8);
        let pushi2_instr = Instruction::new(Opcode::PUSHI) // 4,4,4
            .with_operand::<i64>(&4);
        let addi_instr = Instruction::new(Opcode::ADDI); // 4,8
        let movi_instr = Instruction::new(Opcode::MOVI) // 8
            .with_operand::<i64>(&-16);
        let dupi2_instr = Instruction::new(Opcode::DUPI) // 8,8
            .with_operand::<i64>(&-8);
        let pushi3_instr = Instruction::new(Opcode::PUSHI) // 8,8,2
            .with_operand::<i64>(&2);
        let muli_instr = Instruction::new(Opcode::MULI); // 8, 16
        let dupi3_instr = Instruction::new(Opcode::DUPI) // 8, 16, 16
            .with_operand::<i64>(&-8);
        let movi2_instr = Instruction::new(Opcode::MOVI) // 16, 16
            .with_operand::<i64>(&-24);
        let pushi4_instr = Instruction::new(Opcode::PUSHI) // 16, 16, 4
            .with_operand::<i64>(&4);
        let dupi4_instr = Instruction::new(Opcode::DUPI) // 16, 16, 4, 4
            .with_operand::<i64>(&-8);
        let movi3_instr = Instruction::new(Opcode::MOVI) // 4, 16, 4
            .with_operand::<i64>(&-32);

        comp_builder.push_instr(pushi_instr);
        comp_builder.push_instr(dupi_instr);
        comp_builder.push_instr(pushi2_instr);
        comp_builder.push_instr(addi_instr);
        comp_builder.push_instr(movi_instr);
        comp_builder.push_instr(dupi2_instr);
        comp_builder.push_instr(pushi3_instr);
        comp_builder.push_instr(muli_instr);
        comp_builder.push_instr(dupi3_instr);
        comp_builder.push_instr(movi2_instr);
        comp_builder.push_instr(pushi4_instr);
        comp_builder.push_instr(dupi4_instr);
        comp_builder.push_instr(movi3_instr);

        let comp_code = comp_builder.build();
        let code = compiler.get_resulting_code();

        assert_eq!(comp_code, code);
    }

    #[test]
    fn test_compile_return() {
        let code = String::from("
            var:int x = 4;
            var:int y = x + 4;
            return y - 4;
        ");

        let mut lexer = Token::lexer(code.as_str());
        let parser = Parser::new(code.clone());
        let stmt_list_res = parser.parse_statement_list(&mut lexer);

        assert!(stmt_list_res.is_ok());
        let stmt_list = stmt_list_res.unwrap();

        let mut compiler = Compiler::new();
        compiler.reset_builder();
        let mut context = Context::new();
        context.return_type = Some(Type::Int);
        compiler.push_new_context(context);

        for stmt in stmt_list {
            let cmp_res = compiler.compile_statement(stmt);
            assert!(cmp_res.is_ok());
        }

        let mut comp_builder = Builder::new();

        let pushi_instr = Instruction::new(Opcode::PUSHI) // 4
            .with_operand::<i64>(&4);
        let dupi_instr = Instruction::new(Opcode::DUPI) // 4, 4
            .with_operand::<i64>(&-8);
        let pushi2_instr = Instruction::new(Opcode::PUSHI) // 4, 4, 4
            .with_operand::<i64>(&4);
        let addi_instr = Instruction::new(Opcode::ADDI); // 4, 8
        let dupi2_instr = Instruction::new(Opcode::DUPI) // 4, 8, 8
            .with_operand::<i64>(&-8);
        let pushi3_instr = Instruction::new(Opcode::PUSHI) // 4, 8, 8, 4
            .with_operand::<i64>(&4);
        let subi_instr = Instruction::new(Opcode::SUBI); // 4, 8, 4
        let svswp_instr = Instruction::new(Opcode::SVSWPI); // 4, 8
        let popn_instr = Instruction::new(Opcode::POPN) // 
            .with_operand::<u64>(&16);
        let ldswp_instr = Instruction::new(Opcode::LDSWPI); // 4

        comp_builder.push_instr(pushi_instr);
        comp_builder.push_instr(dupi_instr);
        comp_builder.push_instr(pushi2_instr);
        comp_builder.push_instr(addi_instr);
        comp_builder.push_instr(dupi2_instr);
        comp_builder.push_instr(pushi3_instr);
        comp_builder.push_instr(subi_instr);
        comp_builder.push_instr(svswp_instr);
        comp_builder.push_instr(popn_instr);
        comp_builder.push_instr(ldswp_instr);

        println!("{:?}", compiler.builder.instructions);

        let comp_code = comp_builder.build();
        let code = compiler.get_resulting_code();

        assert_eq!(comp_code, code);
    }


    #[test]
    pub fn test_compile_fn_decl() {
        let code = String::from("
            fn: main(arg: int) ~ int {
                var:int x = arg * 4;
                var:int y = x + 4;

                return y - 4;
            }
        ");

        let mut lexer = Token::lexer(code.as_str());
        let parser = Parser::new(code.clone());
        let decl_list_res = parser.parse_decl_list();

        assert!(decl_list_res.is_ok());
        let decl_list = decl_list_res.unwrap();

        let mut compiler = Compiler::new();
        compiler.reset_builder();
        compiler.push_empty_context();
        
        let comp_res = compiler.compile_decl_list(decl_list);
        assert!(comp_res.is_ok());
        

        let mut comp_builder = Builder::new();

        let dupi0_instr = Instruction::new(Opcode::DUPI) // x
            .with_operand::<i64>(&-8);
        let pushi0_instr = Instruction::new(Opcode::PUSHI) // x, 4
            .with_operand::<i64>(&4);
        let mul_instr = Instruction::new(Opcode::MULI); // 4x
        let dupi_instr = Instruction::new(Opcode::DUPI) // 4x, 4x
            .with_operand::<i64>(&-8);
        let pushi_instr = Instruction::new(Opcode::PUSHI) // 4x, 4x, 4
            .with_operand::<i64>(&4);
        let addi_instr = Instruction::new(Opcode::ADDI); // 4x, 4x+4
        let dupi2_instr = Instruction::new(Opcode::DUPI) // 4x, 4x+4, 4x+4
            .with_operand::<i64>(&-8);
        let pushi2_instr = Instruction::new(Opcode::PUSHI) // 4x, 4x+4, 4x+4, 4
            .with_operand::<i64>(&4);
        let subi_instr = Instruction::new(Opcode::SUBI); // 4x, 4x+4, 4x
        let svswp_instr = Instruction::new(Opcode::SVSWPI); // 4x, 4x+4
        let popn_instr = Instruction::new(Opcode::POPN) // 
            .with_operand::<u64>(&16);
        let ldswp_instr = Instruction::new(Opcode::LDSWPI); // 4x

        comp_builder.push_instr(dupi0_instr);
        comp_builder.push_instr(pushi0_instr);
        comp_builder.push_instr(mul_instr);
        comp_builder.push_instr(dupi_instr);
        comp_builder.push_instr(pushi_instr);
        comp_builder.push_instr(addi_instr);
        comp_builder.push_instr(dupi2_instr);
        comp_builder.push_instr(pushi2_instr);
        comp_builder.push_instr(subi_instr);
        comp_builder.push_instr(svswp_instr);
        comp_builder.push_instr(popn_instr);
        comp_builder.push_instr(ldswp_instr);

        println!("{:?}", compiler.builder.instructions);

        let main_uid = compiler.get_function_uid(String::from("main"));

        let comp_code = comp_builder.build();
        let mut fn_map = HashMap::new();
        fn_map.insert(main_uid, 0);
        let comp_prog = Program::new()
            .with_code(comp_code)
            .with_functions(fn_map);
        let program_res = compiler.get_program();
        assert!(program_res.is_ok());
        let program = program_res.unwrap();
        assert_eq!(program, comp_prog);
    }
}