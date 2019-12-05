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
    instruction::Instruction
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
    context_stack: VecDeque<Context>,
    builder: Builder,
    function_uid_map: HashMap<String, u64>,
    function_uid_set: HashSet<u64>
}

pub struct Context {
    pub variables: HashMap<String, isize>,
    pub variable_types: HashMap<String, Type>,
    pub functions: HashMap<String, Type>,
    pub stack_size: usize
}

impl Context {
    pub fn new() -> Context {
        Context {
            variables: HashMap::new(),
            functions: HashMap::new(),
            variable_types: HashMap::new(),
            stack_size: 0
        }
    }
}

pub type CompilerResult<T> = Result<T, CompilerError>;

#[derive(Debug)]
pub enum CompilerError {
    Unknown,
    UnknownType,
    NotImplemented,
    UnknownVariable,
    TypeMismatch
}

impl Compiler {
    pub fn new() -> Compiler {
        let comp = Compiler {
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

    pub fn push_empty_context(&mut self) {
        self.context_stack.push_front(Context::new());
    }

    pub fn get_function_uid(&mut self, function_name: String) -> u64 {
        let opt = self.function_uid_map.get(&function_name);
        if opt.is_some() {
            opt.unwrap().clone()
        } else {
            let mut rng = thread_rng();
            let mut uid = rng.next_u64();
            while self.function_uid_set.contains(&uid) {
                uid = rng.next_u64();
            }
            self.function_uid_set.insert(uid.clone());
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

        {
            let mut front_context = self.context_stack.get_mut(0)
            .ok_or(CompilerError::Unknown)?;

            front_context.functions.insert(fn_decl_args.name.clone(), fn_decl_args.returns);
        }

        let mut context = Context::new();

        let mut stack_index = 0;
        for (_, (var_name, var_type)) in fn_decl_args.arguments.iter().rev() {
            let size = self.size_of_type(var_type)?;
            context.variables.insert(var_name.clone(), stack_index - size as isize);
            context.variable_types.insert(var_name.clone(), var_type.clone());
            stack_index -= size as isize;
        }

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
            _ => {
                return Err(CompilerError::NotImplemented);
            }
        };

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
            front_context.variables.insert(var_decl_args.name.clone(), front_context.stack_size as isize);
            front_context.variable_types.insert(var_decl_args.name.clone(), var_type.clone());
            front_context.stack_size += size;
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

        let var_offset = self.offset_of_var(var_name.clone())?;

        self.compile_expr(&expr)?;
        
        let mov_instr = match var_type {
            Type::Int => {
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
            },
            Expression::FloatLiteral(float) => {
                return Err(CompilerError::NotImplemented);
            },
            Expression::Variable(var_name) => {
                let front_context = self.context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                
                let stack_index = front_context.variables.get(var_name)
                    .ok_or(CompilerError::Unknown)?;

                let dupi_instr = Instruction::new(Opcode::DUPI)
                    .with_operand(&stack_index);
                self.builder.push_instr(dupi_instr);
            },
            Expression::Addition(lhs, rhs) => {
                self.compile_expr(&lhs)?;
                self.compile_expr(&rhs)?;
                let addi_instr = Instruction::new(Opcode::ADDI);
                self.builder.push_instr(addi_instr);
            },
            Expression::Subtraction(lhs, rhs) => {
                self.compile_expr(&lhs)?;
                self.compile_expr(&rhs)?;
                let subi_instr = Instruction::new(Opcode::SUBI);
                self.builder.push_instr(subi_instr);
            },
            Expression::Multiplication(lhs, rhs) => {
                self.compile_expr(&lhs)?;
                self.compile_expr(&rhs)?;
                let muli_instr = Instruction::new(Opcode::MULI);
                self.builder.push_instr(muli_instr);
            },
            Expression::Division(lhs, rhs) => {
                self.compile_expr(&lhs)?;
                self.compile_expr(&rhs)?;
                let divi_instr = Instruction::new(Opcode::DIVI);
                self.builder.push_instr(divi_instr);
            },
            _ => return Err(CompilerError::NotImplemented)
        };
        Ok(())
    }

    pub fn size_of_type(&self, var_type: &Type) -> CompilerResult<usize> {
        let size = match var_type {
            Type::Int => 8,
            Type::Float => 8,
            Type::String => 8,
            _ => {
                return Err(CompilerError::UnknownType);
            }
        };
        Ok(size)
    }

    pub fn offset_of_var(&self, var_name: String) -> CompilerResult<isize> {
        let front_context = self.context_stack.get(0)
            .ok_or(CompilerError::UnknownVariable)?;
        front_context.variables.get(&var_name)
            .ok_or(CompilerError::UnknownVariable)
            .map(|val| val.clone())
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
}


#[cfg(test)]
mod test {
    use crate::{
        parser::{
            lexer::Token,
            parser::Parser
        },
        vm::{
            is::Opcode            
        },
        codegen::{
            instruction::Instruction,
            builder::Builder
        }
    };
    use super::{
        Compiler
    };

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
            .with_operand::<i64>(&0);
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
            .with_operand::<i64>(&0);
        let pushi2_instr = Instruction::new(Opcode::PUSHI)
            .with_operand::<i64>(&4);
        let addi_instr = Instruction::new(Opcode::ADDI);
        let movi_instr = Instruction::new(Opcode::MOVI)
            .with_operand::<i64>(&0);

        comp_builder.push_instr(pushi_instr);
        comp_builder.push_instr(dupi_instr);
        comp_builder.push_instr(pushi2_instr);
        comp_builder.push_instr(addi_instr);
        comp_builder.push_instr(movi_instr);

        let comp_code = comp_builder.build();
        let code = compiler.get_resulting_code();

        assert_eq!(comp_code, code);
    }
}