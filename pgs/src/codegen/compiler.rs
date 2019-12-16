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
        FunctionContext,
        ModuleContext
    },
    program::Program
};

use std::{
    collections::{
        VecDeque,
        HashMap,
        HashSet,
        BTreeMap
    }
};

use rand::{
    Rng,
    RngCore,
    thread_rng
};

pub struct Compiler {
    global_context: FunctionContext,
    mod_context_stack: VecDeque<ModuleContext>,
    fn_context_stack: VecDeque<FunctionContext>,
    pub builder: Builder,
    function_uid_map: HashMap<String, u64>,
    function_uid_set: HashSet<u64>,
    loop_uid_set: HashSet<u64>
}

pub type CompilerResult<T> = Result<T, CompilerError>;

#[derive(Debug)]
pub enum CompilerError {
    Unknown,
    UnknownType,
    UnknownFunction,
    NotImplemented,
    UnknownVariable,
    TypeMismatch,
    DuplicateFunctionName,
    DuplicateModule,
    DuplicateStruct,
    InvalidArgumentCount,
}

impl Compiler {
    pub fn new() -> Compiler {
        let comp = Compiler {
            mod_context_stack: VecDeque::new(),
            global_context: FunctionContext::new(),
            fn_context_stack: VecDeque::new(),
            builder: Builder::new(),
            function_uid_map: HashMap::new(),
            function_uid_set: HashSet::new(),
            loop_uid_set: HashSet::new()
        };
        comp
    }

    pub fn push_context(&mut self) {
        let stack_size = {
            let front_context_opt = self.fn_context_stack.get(0);
            if front_context_opt.is_some() {
                front_context_opt.unwrap().stack_size
            } else {
                0
            }
        };
        let mut context = FunctionContext::new();
        context.stack_size = stack_size;
        self.fn_context_stack.push_front(context);
    }

    /// # Resolves a function name to the relevant data
    /// 
    /// Will resolve a function either by just the name:
    /// * First tries to resolve it by looking in the current modules declarations
    /// * Then looks in the relevant imports
    /// 
    /// Otherwise resolves the function by using the full module path.  
    /// Returns an `CompilerError::UnknownFunction` error if the function  
    /// name does not resolve to a function.
    /// 
    /// ### Params
    /// * `name`:   canonical name of the function
    /// ### Returns
    /// A Result containing the function data, errors otherwise
    pub fn resolve_fn(&self, name: &String) -> CompilerResult<(u64, Type, BTreeMap<usize, (String, Type)>)> {
        // If directly accessing via module namespace
        if name.contains("::") {
            let path = self.get_module_path(&name);

            let mut mod_ctx;
            let mut offset = 1;
            // Module access relative to the root module
            if path[0] == "root" {
                mod_ctx = self.get_root_module()?;
            }
            // Module access relative to this current modules parent
            else if path[0] == "super" {
                mod_ctx = self.get_super_module()?;
            }
            // Module access relative to this current module
            else {
                mod_ctx = self.get_current_module()?;
                offset = 0;
            }

            // Iterate from the offset (in case of super and root, 1)  
            // To the second last element (as the last is the function name)
            for i in offset..path.len() - 1 {
                mod_ctx = mod_ctx.modules.get(&String::from(path[i]))
                    .ok_or(CompilerError::Unknown)?;
            }

            return mod_ctx.functions.get(name)
                .cloned()
                .ok_or(CompilerError::UnknownFunction);
        }
        // If accessing relative to this module
        else {
            let mod_ctx = self.get_current_module()?;
            // If declared in this module
            if let Some(fn_tuple) = mod_ctx.functions.get(name) {
                return Ok(fn_tuple.clone());
            }
            // If imported from other module
            else if let Some(module_path) = mod_ctx.imports.get(name) {
                return self.resolve_fn(module_path);
            }
            // Otherwise, the function is unknown.
            else {
                return Err(CompilerError::UnknownFunction);
            }
        }
    }

    pub fn get_module_path<'s>(&self, name: &'s String) -> Vec<&'s str> {
        name.split("::").collect()
    }

    pub fn get_root_module(&self) -> CompilerResult<&ModuleContext> {
        if self.mod_context_stack.len() == 0 {
            return Err(CompilerError::Unknown);
        }
        self.mod_context_stack.get(self.mod_context_stack.len() - 1)
            .ok_or(CompilerError::Unknown)
    }

    pub fn get_super_module(&self) -> CompilerResult<&ModuleContext> {
        if self.mod_context_stack.len() < 2 {
            return Err(CompilerError::Unknown);
        }
        self.mod_context_stack.get(1)
            .ok_or(CompilerError::Unknown)
    }

    pub fn get_current_module(&self) -> CompilerResult<&ModuleContext> {
        if self.mod_context_stack.len() == 0 {
            return Err(CompilerError::Unknown);
        }
        self.mod_context_stack.get(0)
            .ok_or(CompilerError::Unknown)
    }

    pub fn get_current_module_mut(&mut self) -> CompilerResult<&mut ModuleContext> {
        if self.mod_context_stack.len() == 0 {
            return Err(CompilerError::Unknown);
        }
        self.mod_context_stack.get_mut(0)
            .ok_or(CompilerError::Unknown)
    }

    pub fn get_context(&mut self) -> Option<FunctionContext> {
        self.fn_context_stack.get(0).cloned()
    }

    pub fn push_new_context(&mut self, context: FunctionContext) {
        self.fn_context_stack.push_front(context);
    }

    pub fn push_empty_context(&mut self) {
        self.fn_context_stack.push_front(FunctionContext::new());
    }

    pub fn push_default_module_context(&mut self) {
        self.mod_context_stack.push_front(
            ModuleContext::new(String::from("root"))
        );
    }

    pub fn pop_module_context(&mut self) -> Option<ModuleContext> {
        self.mod_context_stack.pop_front()
    }

    pub fn reset_global(&mut self) {
        self.global_context = FunctionContext::new();
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

    pub fn type_of_var(&self, var_name: &String) -> CompilerResult<Type> {
        let front_context = self.fn_context_stack.get(0)
            .ok_or(CompilerError::UnknownVariable)?;
        let var_type = front_context.variable_types.get(var_name)
            .ok_or(CompilerError::UnknownVariable)?;
        Ok(var_type.clone())
    }

    pub fn type_of_fn(&self, fn_name: &String) -> CompilerResult<Type> {
        let fn_type = {
            let front_mod_ctx = self.mod_context_stack.get(0)
                .ok_or(CompilerError::Unknown)?;
            let fun = front_mod_ctx.functions.get(fn_name)
                .ok_or(CompilerError::Unknown)?;
            fun.1.clone()
        };
        Ok(
            fn_type
        )
    }

    pub fn get_resulting_code(&mut self) -> Vec<u8> {
        let builder = self.builder.clone();
        builder.build()
    }

    pub fn get_builder_ref(&self) -> &Builder {
        &self.builder
    }

    pub fn get_program(&mut self) -> CompilerResult<Program> {
        let mut builder = self.builder.clone();
        let mut functions = HashMap::new();

        for (fn_name, fn_uid) in self.function_uid_map.iter() {
            let fn_offset = builder.get_label_offset(fn_name)
                .ok_or(CompilerError::UnknownFunction)?;
            functions.insert(*fn_uid, fn_offset);
        }

        let code = builder.build();

        let program = Program::new()
            .with_code(code)
            .with_functions(functions);

        Ok(program)
    }

    pub fn get_function_uid(&mut self, function_name: &String) -> u64 {
        let opt = self.function_uid_map.get(function_name);
        if opt.is_some() {
            opt.unwrap().clone()
        } else {
            let mut rng = thread_rng();
            let mut uid = rng.next_u64();
            while self.function_uid_set.contains(&uid) {
                uid = rng.next_u64();
            }
            self.function_uid_set.insert(uid.clone());
            self.function_uid_map.insert(function_name.clone(), uid.clone());
            uid
        }
    }

    pub fn get_loop_uid(&mut self) -> u64 {
        let mut rng = thread_rng();
        let mut uid = rng.next_u64();
        while self.loop_uid_set.contains(&uid) {
            uid = rng.next_u64();
        }
        uid
    }

    pub fn get_full_function_name(&mut self, function_name: &String) -> String {
        let mut full_fn_name = String::new();

        for module in self.mod_context_stack.iter().rev() {
            full_fn_name += &module.name;
            full_fn_name += "::";
        }

        full_fn_name += function_name;

        full_fn_name
    }

    pub fn reset_builder(&mut self) {
        self.builder = Builder::new();
    }

    pub fn decl_decl_list(&mut self, decl_list: &Vec<Declaration>) -> CompilerResult<()> {
        for decl in decl_list.iter() {
            self.decl_decl(decl)?;
        }
        Ok(())
    }

    pub fn decl_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        match decl {
            Declaration::Function(_) => self.decl_fn_decl(decl)?,
            Declaration::Module(_, _) => self.decl_mod_decl(decl)?,
            Declaration::Struct(_) => self.decl_struct_decl(decl)?,
            Declaration::Import(_, _) => self.decl_import_decl(decl)?,
            _ => {}
        };
        Ok(())
    }

    pub fn decl_import_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let (import_path, import_name) = match decl {
            Declaration::Import(import_path, import_name) => (import_path.clone(), import_name.clone()),
            _ => return Err(CompilerError::Unknown)
        };

        let mod_ctx = self.get_current_module_mut()?;
        mod_ctx.imports.insert(import_name, import_path);

        Ok(())
    }

    pub fn decl_fn_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let fn_decl_args = match decl {
            Declaration::Function(fn_decl_args) => fn_decl_args,
            _ => return Err(CompilerError::Unknown)
        };
        let full_fn_name = self.get_full_function_name(&fn_decl_args.name);
        let uid = self.get_function_uid(&full_fn_name);

        let fn_tuple = (uid, fn_decl_args.returns.clone(), fn_decl_args.arguments.clone());

        let front_mod_ctx = self.mod_context_stack.get_mut(0)
            .ok_or(CompilerError::Unknown)?;
        
        let insert_opt = front_mod_ctx.functions.insert(fn_decl_args.name.clone(), fn_tuple);
        if insert_opt.is_some() {
            return Err(CompilerError::DuplicateFunctionName);
        }

        Ok(())
    }

    pub fn decl_struct_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let struct_decl_args = match decl {
            Declaration::Struct(struct_decl_args) => struct_decl_args,
            _ => return Err(CompilerError::Unknown)
        };

        let struct_name = struct_decl_args.name.clone();
        
        let front_mod_ctx = self.mod_context_stack.get_mut(0)
            .ok_or(CompilerError::Unknown)?;

        let insert_opt = front_mod_ctx.structs.insert(struct_name, struct_decl_args.members.clone());
        if insert_opt.is_some() {
            return Err(CompilerError::DuplicateStruct);
        }

        Ok(())
    }

    pub fn decl_mod_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let (mod_name, decl_list) = match decl {
            Declaration::Module(mod_name, decl_list) => (mod_name, decl_list),
            _ => return Err(CompilerError::Unknown)
        };
        let mut mod_context = ModuleContext::new(mod_name.clone());
        self.mod_context_stack.push_front(mod_context);
        self.decl_decl_list(decl_list)?;
        mod_context = self.mod_context_stack.pop_front().unwrap();
        {
            let front_mod_ctx = self.mod_context_stack.get_mut(0)
                .ok_or(CompilerError::Unknown)?;

            let insert_opt = front_mod_ctx.modules.insert(mod_name.clone(), mod_context.clone());
            if insert_opt.is_some() {
                return Err(CompilerError::DuplicateModule);
            }
        }
        Ok(())
    }

    pub fn compile_decl_list(&mut self, decl_list: Vec<Declaration>) -> CompilerResult<()> {
        self.decl_decl_list(&decl_list)?;
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
        let fn_decl_args = match fn_decl {
            Declaration::Function(fn_decl_args) => fn_decl_args,
            _ => {
                return Err(CompilerError::Unknown);
            }
        };
        let full_fn_name = self.get_full_function_name(&fn_decl_args.name);
        let uid = self.get_function_uid(&full_fn_name);
        self.builder.push_label(full_fn_name.clone());

        let mut context = FunctionContext::new();

        let mut stack_index = 0;
        for (_, (var_name, var_type)) in fn_decl_args.arguments.iter().rev() {
            let size = self.size_of_type(var_type)?;
            context.set_var(stack_index - size as i64, (var_name.clone(), var_type.clone()));
            stack_index -= size as i64;
        }

        context.return_type = Some(fn_decl_args.returns);

        self.fn_context_stack.push_front(context);

        if let Some(statements) = fn_decl_args.code_block {
            for statement in statements {
                self.compile_statement(statement)?;
            }
        }

        self.fn_context_stack.pop_front();

        Ok(())
    }

    pub fn compile_statement(&mut self, stmt: Statement) -> CompilerResult<()> {
        match stmt {
            Statement::VariableDecl(_) => {
                self.compile_var_decl_stmt(stmt)?
            },
            Statement::Assignment(_, _) => {
                self.compile_var_assign_stmt(stmt)?
            },
            Statement::Return(_) => {
                self.compile_return_stmt(stmt)?
            },
            Statement::Call(_, _) => {
                self.compile_call_stmt(stmt)?;
            },
            _ => {
                return Err(CompilerError::NotImplemented);
            }
        };

        Ok(())
    }

    pub fn compile_call_stmt(&mut self, stmt: Statement) -> CompilerResult<()> {
        let (fn_name, params) = match stmt {
            Statement::Call(fn_name, params) => (fn_name, params),
            _ => {
                return Err(CompilerError::Unknown);
            }
        };

        let fn_decl_args = {
            self.global_context.functions.get(&fn_name)
                .ok_or(CompilerError::UnknownFunction)?
                .clone()
        };

        let fn_uid = self.get_function_uid(&fn_name);
        
        let fn_arg_req_len = fn_decl_args.arguments.len();
        if params.len() != fn_arg_req_len {
            return Err(CompilerError::InvalidArgumentCount);
        }
        let mut call_stack_diff = 0;
        for (i, (var_name, var_type)) in fn_decl_args.arguments.iter() {
            let arg_type = {
                let checker = Checker::new(self);
                checker.check_expr_type(&params[*i])
                    .map_err(|_| CompilerError::TypeMismatch)?
            };
            if arg_type != *var_type {
                return Err(CompilerError::TypeMismatch);
            }
            call_stack_diff += self.size_of_type(var_type)?;
            self.compile_expr(&params[*i])?;
        }
        let call_instr = Instruction::new(Opcode::CALL)
            .with_operand(&fn_uid);
        self.builder.push_instr(call_instr);


        let size = self.size_of_type(&fn_decl_args.returns)?;

        let front_context = self.fn_context_stack.get_mut(0)
            .ok_or(CompilerError::Unknown)?;
        
        front_context.stack_size += call_stack_diff;
        front_context.stack_size += size;

        Ok(())
    }
    
    pub fn compile_return_stmt(&mut self, stmt: Statement) -> CompilerResult<()> {
        let ret_expr = match stmt {
            Statement::Return(expr) => expr,
            _ => return Err(CompilerError::Unknown)
        };

        let checker = Checker::new(&self);
        let expr_type = checker.check_expr_type(&ret_expr)
            .map_err(|_| CompilerError::TypeMismatch)?;

        let fn_type = self.fn_context_stack.get(0)
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
            let front_context = self.fn_context_stack.get_mut(0)
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
        self.builder.push_instr(Instruction::new(Opcode::RET));

        Ok(())
    }

    pub fn compile_var_decl_stmt(&mut self, stmt: Statement) -> CompilerResult<()> {
        let var_decl_args = match stmt {
            Statement::VariableDecl(args) => args,
            _ => return Err(CompilerError::Unknown)
        };

        let size = self.size_of_type(&var_decl_args.var_type)?;
        let var_type = var_decl_args.var_type.clone();
        // Insert variable to context
        {
            let front_context = self.fn_context_stack.get_mut(0)
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

    pub fn compile_var_assign_stmt(&mut self, stmt: Statement) -> CompilerResult<()> {
        let (var_name, expr) = match stmt {
            Statement::Assignment(name, assign) => (name, assign),
            _ => return Err(CompilerError::Unknown)
        };

        let var_type = self.type_of_var(&var_name)?;
        let checker = Checker::new(&self);
        let expr_type = checker.check_expr_type(&expr)
            .map_err(|_| CompilerError::TypeMismatch)?;

        if expr_type != var_type {
            return Err(CompilerError::TypeMismatch);
        }

        self.compile_expr(&expr)?;
        
        let var_offset = {
            let front_context = self.fn_context_stack.get(0)
                .ok_or(CompilerError::Unknown)?;
            front_context.offset_of(&var_name)
                .ok_or(CompilerError::UnknownVariable)?
        };

        let mov_instr = match var_type {
            Type::Int => {
                let front_context = self.fn_context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size -= 8;
                Instruction::new(Opcode::SMOVI)
                    .with_operand(&var_offset)
            },
            _ => {
                return Err(CompilerError::NotImplemented);
            }
        };

        self.builder.push_instr(mov_instr);

        Ok(())
    }

    pub fn compile_call_expr(&mut self, expr: &Expression) -> CompilerResult<()> {
        let (name, args) = match expr {
            Expression::Call(name, args) => (name, args),
            _ => return Err(CompilerError::Unknown)
        };
        
        let front_mod_ctx = self.mod_context_stack.get(0)
            .ok_or(CompilerError::Unknown)?;
        
        let (fn_uid, fn_ret_type, fn_args) = front_mod_ctx.functions.get(name)
            .ok_or(CompilerError::UnknownFunction)?;
        
        for (_, (var_name, var_type)) in fn_args.iter() {
            let var_offset = {
                let front_context = self.fn_context_stack.get(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.offset_of(var_name)
                    .ok_or(CompilerError::UnknownVariable)?
            };
            let dupi_instr = Instruction::new(Opcode::SDUPI)
                .with_operand(&var_offset);
            self.builder.push_instr(dupi_instr);
            let front_context = self.fn_context_stack.get_mut(0)
                .ok_or(CompilerError::Unknown)?;
            front_context.stack_size += 8;
        }
        let call_instr = Instruction::new(Opcode::CALL)
            .with_operand(fn_uid);
        self.builder.push_instr(call_instr);

        let front_context = self.fn_context_stack.get_mut(0)
            .ok_or(CompilerError::Unknown)?;
        front_context.stack_size += 8;

        Ok(())
    }

    pub fn compile_expr(&mut self, expr: &Expression) -> CompilerResult<()> {
        match expr {
            Expression::IntLiteral(int) => {
                let pushi_instr = Instruction::new(Opcode::PUSHI)
                    .with_operand(int);
                self.builder.push_instr(pushi_instr);
                let front_context = self.fn_context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size += 8;
            },
            Expression::FloatLiteral(float) => {
                return Err(CompilerError::NotImplemented);
            },
            Expression::Call(_, _) => {
                self.compile_call_expr(expr)?;
            },
            Expression::Variable(var_name) => {      
                let var_offset = {
                    let front_context = self.fn_context_stack.get(0)
                        .ok_or(CompilerError::Unknown)?;
                    front_context.offset_of(var_name)
                        .ok_or(CompilerError::UnknownVariable)?
                };
                let dupi_instr = Instruction::new(Opcode::SDUPI)
                    .with_operand(&var_offset);
                self.builder.push_instr(dupi_instr);
                let front_context = self.fn_context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size += 8;
            },
            Expression::Addition(lhs, rhs) => {
                self.compile_expr(&lhs)?;
                self.compile_expr(&rhs)?;
                let addi_instr = Instruction::new(Opcode::ADDI);
                self.builder.push_instr(addi_instr);
                let front_context = self.fn_context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size -= 16;
                front_context.stack_size += 8;
            },
            Expression::Subtraction(lhs, rhs) => {
                self.compile_expr(&lhs)?;
                self.compile_expr(&rhs)?;
                let subi_instr = Instruction::new(Opcode::SUBI);
                self.builder.push_instr(subi_instr);
                let front_context = self.fn_context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size -= 16;
                front_context.stack_size += 8;
            },
            Expression::Multiplication(lhs, rhs) => {
                self.compile_expr(&lhs)?;
                self.compile_expr(&rhs)?;
                let muli_instr = Instruction::new(Opcode::MULI);
                self.builder.push_instr(muli_instr);
                let front_context = self.fn_context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size -= 16;
                front_context.stack_size += 8;
            },
            Expression::Division(lhs, rhs) => {
                self.compile_expr(&lhs)?;
                self.compile_expr(&rhs)?;
                let divi_instr = Instruction::new(Opcode::DIVI);
                self.builder.push_instr(divi_instr);
                let front_context = self.fn_context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size -= 16;
                front_context.stack_size += 8;
            },
            _ => return Err(CompilerError::NotImplemented)
        };
        Ok(())
    }
}
