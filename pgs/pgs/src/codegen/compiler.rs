use crate::{
    codegen::{
        context::{
            ModuleContext,
            FunctionContext,
            VariableLocation
        },
        uid_generator::UIDGenerator,
        def::{
            ContainerDef,
            FunctionDef
        },
        builder::{
            Builder
        },
        register::{
            Register
        },
        instruction::{
            Instruction
        },
        data::{
            Data
        },
        program::{
            Program
        }
    },
    parser::{
        ast::{
            Declaration,
            Statement,
            Type,
            Expression,
            IfStatementArgs
        }
    },
    vm::{
        is::{
            Opcode
        }
    }
};

use std::{
    fmt::{
        Display,
        Result as FmtResult,
        Formatter
    },
    error::Error,
    collections::{
        VecDeque,
        HashMap
    },
    ops::{
        Deref
    }
};

#[derive(Debug, Clone)]
pub enum CompilerError {
    Unknown,
    Unimplemented(String),
    DuplicateVariable(String),
    DuplicateMember(String),
    DuplicateFunction(String),
    DuplicateModule(String),
    DuplicateContainer(String),
    DuplicateImport(String),
    UnknownFunction(String),
    UnknownContainer(String),
    UnknownVariable(String),
    UnknownModule(String),
    UnknownType(Type),
    UnsupportedExpression(Expression),
    TypeMismatch(Type, Type),
    RegisterMapping
}

impl Display for CompilerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}

impl Error for CompilerError {}

/// Convenience type for Results returned by a compilation process
pub type CompilerResult<T> = Result<T, CompilerError>;

/// The compiler
pub struct Compiler {
    fn_context_stack: VecDeque<FunctionContext>,
    mod_context_stack: VecDeque<ModuleContext>,
    fn_uid_map: HashMap<String, u64>,
    uid_generator: UIDGenerator,
    builder: Builder,
    current_cont: Option<String>,
    data: Data
}

impl Compiler {
    /// Creates a new compiler instance and pushes the "root" module on the context stack
    pub fn new() -> Compiler {
        let root_mod_ctx = ModuleContext::new(String::from("root"));
        let mut mod_context_stack = VecDeque::new();
        mod_context_stack.push_front(root_mod_ctx);
        Compiler {
            fn_context_stack: VecDeque::new(),
            mod_context_stack: mod_context_stack,
            fn_uid_map: HashMap::new(),
            uid_generator: UIDGenerator::new(),
            builder: Builder::new(),
            current_cont: None,
            data: Data::new()
        }
    }

    /// Retrieves a reference to the underlying builder
    pub fn get_builder(&self) -> &Builder {
        &self.builder
    }

    /// Retrieves the program instance compiled by this compiler instance.
    pub fn get_program(&self) -> CompilerResult<Program> {
        let mut builder = self.builder.clone();
        let data = self.data.clone();
        let data_len = data.bytes.len();

        // Modify target jump addresses of JMP instructions accordingly 
        for offset in builder.jmp_instructions.clone().iter() {
            let instr = builder.get_instr(offset)
                .ok_or(CompilerError::Unknown)?;
            let addr: u64 = match instr.opcode {
                Opcode::JMP => instr.get_operand(0, 8),
                Opcode::JMPF => instr.get_operand(1, 8),
                Opcode::JMPT => instr.get_operand(1, 8),
                _ => return Err(CompilerError::Unknown)
            };
            instr.remove_operand_bytes(8);
            instr.append_operand(addr + data_len as u64);
        }

        let mut functions: HashMap<u64, usize> = HashMap::new();

        // correctly set function offsets
        for (fn_name, fn_uid) in self.fn_uid_map.iter() {
            let fn_offset = builder.get_label_offset(fn_name)
                .ok_or(CompilerError::Unknown)?;
            functions.insert(fn_uid.clone(), fn_offset + data_len);
        }

        let mut code = data.bytes;
        let mut builder_code = builder.build();
        code.append(&mut builder_code);

        let program = Program::new()
            .with_code(code)
            .with_functions(functions);
        
        Ok(program)
    }

    // #region helpers

    /// Gets the module path on the stack, with trailing "::"
    pub fn get_module_path(&self) -> String {
        let mut ret = String::new();
        for mod_ctx in self.mod_context_stack.iter().rev() {
            ret += &mod_ctx.name;
            ret += "::"
        }
        ret
    }

    /// Gets the current module context (the one at the top of the stack)
    pub fn get_current_module(&self) -> CompilerResult<&ModuleContext> {
        self.mod_context_stack.get(0)
            .ok_or(CompilerError::Unknown)
    }

    /// Gets the current module context (the one at the top of the stack) as a mutable reference
    pub fn get_current_module_mut(&mut self) -> CompilerResult<&mut ModuleContext> {
        self.mod_context_stack.get_mut(0)
            .ok_or(CompilerError::Unknown)
    }

    /// Gets the current function context as a reference
    pub fn get_current_function(&self) -> CompilerResult<&FunctionContext> {
        self.fn_context_stack.get(0)
            .ok_or(CompilerError::Unknown)
    }

    /// Gets the current function context as a mutable reference
    pub fn get_current_function_mut(&mut self) -> CompilerResult<&mut FunctionContext> {
        self.fn_context_stack.get_mut(0)
            .ok_or(CompilerError::Unknown)
    }

    /// Gets the function context at stack index
    pub fn get_function(&self, index: usize) -> CompilerResult<&FunctionContext> {
        self.fn_context_stack.get(index)
            .ok_or(CompilerError::Unknown)
    }

    /// Gets the first parent non-weak function context
    pub fn get_parent_function(&self) -> CompilerResult<&FunctionContext> {
        self.fn_context_stack.iter().find(|fn_ctx| !fn_ctx.weak)
            .ok_or(CompilerError::Unknown)
    }

    /// Pushes a module context on the stack
    pub fn push_module_context(&mut self, mod_ctx: ModuleContext) {
        self.mod_context_stack.push_front(mod_ctx);
    }

    /// Pops the front module context off the stack
    pub fn pop_module_context(&mut self) -> CompilerResult<ModuleContext> {
        self.mod_context_stack.pop_front()
            .ok_or(CompilerError::Unknown)
    }

    /// Pushes a function context on the stack
    pub fn push_function_context(&mut self, fn_ctx: FunctionContext) {
        self.fn_context_stack.push_front(fn_ctx);
    }

    /// Pops the front function context off the stack
    pub fn pop_function_context(&mut self) -> CompilerResult<FunctionContext> {
        self.fn_context_stack.pop_front()
            .ok_or(CompilerError::Unknown)
    }

    /// Resolves a function by name to a FunctionDef
    pub fn resolve_function(&self, name: &String) -> CompilerResult<FunctionDef> {
        Err(CompilerError::Unimplemented(format!("Function resolving not implemented yet!")))
    }

    /// Resolves a container by name to a ContainerDef
    pub fn resolve_container(&self, name: &String) -> CompilerResult<ContainerDef> {
        Err(CompilerError::Unimplemented(format!("Container resolving not implemented yet!")))
    }

    /// Returns the byte size of a given Type
    pub fn get_size_of_type(&self, var_type: &Type) -> CompilerResult<usize> {
        let size = match var_type {
            Type::String => 16,
            Type::Void => 0,
            Type::Int => 8,
            Type::Reference(inner) => {
                match inner.deref() {
                    Type::AutoArray(_) => 16,
                    _ => 8
                }
            },
            Type::Float => 4,
            Type::Bool => 4,
            Type::Other(cont_name) => {
                let cont_def = self.resolve_container(&cont_name)?;
                cont_def.get_size(self)?
            },
            Type::Array(inner_type, size) => {
                let inner_type_size = self.get_size_of_type(&inner_type)?;
                inner_type_size * size
            },
            _ => {
                return Err(CompilerError::UnknownType(var_type.clone()));
            }
        };
        Ok(size)
    }

    /// Returns the type of a given variable
    pub fn get_type_of_var(&self, var_name: &String) -> CompilerResult<Type> {
        let mut type_opt = None;

        for i in 0..self.fn_context_stack.len() {
            let fn_ctx = self.get_function(i)?;
            let var_type_res = fn_ctx.get_var_type(var_name);
            if var_type_res.is_ok() {
                type_opt = Some(var_type_res.unwrap());
                break;
            }
        }

        type_opt.ok_or(CompilerError::UnknownVariable(var_name.clone()))
    }

    /// Returns the offset to SP for a given variable
    pub fn get_sp_offset_of_var(&self, var_name: &String) -> CompilerResult<i64> {
        let fn_ctx = self.get_current_function()?;
        let stack_pos = fn_ctx.get_var_pos(var_name)?;
        let stack_size = fn_ctx.stack_size as i64;
        let mut offset = stack_size - stack_pos;
        if offset > 0 {
            offset = -offset;
        }
        Ok(
            offset
        )
    }

    /// Increments the stack of the current function context
    pub fn inc_stack(&mut self, size: usize) -> CompilerResult<usize> {
        let fn_ctx = self.get_current_function_mut()?;
        fn_ctx.stack_size += size;
        Ok(fn_ctx.stack_size)
    }

    /// Decrements the stack of the current function context
    pub fn dec_stack(&mut self, size: usize) -> CompilerResult<usize> {
        let fn_ctx = self.get_current_function_mut()?;
        fn_ctx.stack_size -= size;
        Ok(fn_ctx.stack_size)
    }

    // #endregion

    // #region declare functions

    /// (Pre-)declares a given declaration list
    pub fn declare_decl_list(&mut self, decl_list: &[Declaration]) -> CompilerResult<()> {
        for decl in decl_list.iter() {
            self.declare_decl(decl)?;
        }
        Ok(())
    }

    /// (Pre-)declares a given declaration
    pub fn declare_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        match decl {
            Declaration::Module(_, _) => self.declare_mod_decl(decl)?,
            Declaration::Function(_) => self.declare_fn_decl(decl)?,
            Declaration::Container(_) => self.declare_cont_decl(decl)?,
            Declaration::Import(_, _) => self.declare_import_decl(decl)?,
            Declaration::Impl(_, _, _) => self.declare_impl_decl(decl)?
        };
        Ok(())
    }

    /// (Pre-)declares a given function declaration
    pub fn declare_fn_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let fn_decl_args = match decl {
            Declaration::Function(fn_decl_args) => fn_decl_args,
            _ => return Err(CompilerError::Unknown)
        };

        let mut full_fn_name = self.get_module_path();
        if let Some(cont_name) = self.current_cont.as_ref().cloned() {
            full_fn_name += &cont_name;
            full_fn_name += "::";
        }
        full_fn_name += &fn_decl_args.name;

        let uid = self.uid_generator.get_function_uid(&full_fn_name);
        self.fn_uid_map.insert(full_fn_name.clone(), uid.clone());

        let fn_def = FunctionDef::from(fn_decl_args)
            .with_uid(uid);

        if let Some(cont_name) = self.current_cont.as_ref().cloned() {
            let mod_ctx = self.get_current_module_mut()?;
            let cont_def = mod_ctx.get_container(&cont_name)?;
            cont_def.add_member_function(fn_def)?;
        } else {
            let mod_ctx = self.get_current_module_mut()?;
            mod_ctx.add_function(fn_def)?;
        }

        Ok(())
    }

    /// (Pre-)declares a given module declaration
    pub fn declare_mod_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let (mod_name, decl_list) = match decl {
            Declaration::Module(mod_name, decl_list) => (mod_name, decl_list),
            _ => return Err(CompilerError::Unknown)
        };

        let mut mod_ctx = ModuleContext::new(mod_name.clone());

        self.push_module_context(mod_ctx);

        self.declare_decl_list(decl_list)?;

        mod_ctx = self.pop_module_context()?;

        let front_mod_ctx = self.get_current_module_mut()?;

        front_mod_ctx.add_module(mod_ctx)?;

        Ok(())
    }

    /// (Pre-)declares a given container declaration
    pub fn declare_cont_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let cont_decl_args = match decl {
            Declaration::Container(args) => args,
            _ => return Err(CompilerError::Unknown)
        };

        let cont_def = ContainerDef::from(cont_decl_args);

        let mod_ctx = self.get_current_module_mut()?;

        mod_ctx.add_container(cont_def)?;

        Ok(())
    }

    /// (Pre-)declares a given import declaration
    pub fn declare_import_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let (import_path, import_as) = match decl {
            Declaration::Import(import_path, import_as) => (import_path, import_as),
            _ => return Err(CompilerError::Unknown)
        };

        let mod_ctx = self.get_current_module_mut()?;
        mod_ctx.add_import(import_as.clone(), import_path.clone())?;

        Ok(())
    }

    /// (Pre-)declares a given impl declaration
    pub fn declare_impl_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let (impl_type, impl_for, decl_list) = match decl {
            Declaration::Impl(impl_type, impl_for, decl_list) => (impl_type, impl_for, decl_list), 
            _ => return Err(CompilerError::Unknown)
        };

        if impl_type == impl_for {
            let mod_ctx = self.get_current_module_mut()?;
            let cont_res = mod_ctx.get_container(impl_type);
            if cont_res.is_err() {
                let cont_def = ContainerDef::new(impl_type.clone());
                mod_ctx.add_container(cont_def)?;
            }
            self.current_cont = Some(impl_type.clone());
            self.declare_decl_list(decl_list)?;
            self.current_cont = None;
        } else {
            return Err(CompilerError::Unimplemented(format!("Cannot currently compile non-cont impls!")));
        }

        Ok(())
    }

    // #endregion
    
    // #region compile functions

    /// Compiles the decl list for the root module
    pub fn compile_root(&mut self, decl_list: &[Declaration]) -> CompilerResult<()> {
        self.declare_decl_list(decl_list)?;
        self.compile_decl_list(decl_list)?;
        Ok(())
    }

    /// Compiles a declaration list
    pub fn compile_decl_list(&mut self, decl_list: &[Declaration]) -> CompilerResult<()> {
        for decl in decl_list.iter() {
            self.compile_decl(decl)?;
        }
        Ok(())
    }

    /// Compiles a declaration
    pub fn compile_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        match decl {
            Declaration::Function(_) => self.compile_fn_decl(decl)?,
            Declaration::Impl(_, _, _) => self.compile_impl_decl(decl)?,
            Declaration::Module(_, _) => self.compile_mod_decl(decl)?,
            _ => {}
        };
        Ok(())
    }

    /// Compiles a function declaration
    pub fn compile_fn_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let fn_decl_args = match decl {
            Declaration::Function(fn_decl_args) => fn_decl_args,
            _ => return Err(CompilerError::Unknown)
        };

        let fn_def = {
            self.get_current_module()?
                .get_function(&fn_decl_args.name)?
                .clone()
        };

        let fn_ret_type = fn_def.ret_type.clone();

        let mut fn_ctx = FunctionContext::new(self, fn_def)?;

        let mut full_fn_name = self.get_module_path();
        full_fn_name += &fn_decl_args.name;


        self.builder.push_label(full_fn_name);

        self.push_function_context(fn_ctx);

        if let Some(stmt_list) = &fn_decl_args.code_block {
            self.compile_stmt_list(stmt_list)?;
        }

        // If the type is void, automatically add a return Statement
        if fn_ret_type == Type::Void {
            let ret_stmt = Statement::Return(None);
            self.compile_return_stmt(&ret_stmt)?;
        }

        // Instruction in case the function didnt return a value
        let halt_instr = Instruction::new(Opcode::HALT)
            .with_operand::<u8>(1);
        self.builder.push_instr(halt_instr);

        Ok(())
    }

    /// Compiles a stack cleanup for a given function context
    pub fn compile_stack_cleanup(&mut self, fn_ctx: &FunctionContext) -> CompilerResult<()> {
        let pop_size = fn_ctx.stack_size;

        // Instruction for popping values off the stack
        let pop_stack_instr = Instruction::new(Opcode::SUBU_I)
            .with_operand::<u8>(Register::SP.into())
            .with_operand::<u64>(pop_size as u64)
            .with_operand::<u8>(Register::SP.into());
        self.builder.push_instr(pop_stack_instr);

        Ok(())
    }

    /// Compiles a full stack unwind until the parent function is hit 
    pub fn compile_stack_cleanup_and_return(&mut self) -> CompilerResult<()> {
        let mut parent_fn_ctx_opt = None;
        let mut stack_size = 0;

        for ctx in self.fn_context_stack.iter() {
            stack_size += ctx.stack_size;
            if !ctx.weak {
                parent_fn_ctx_opt = Some(ctx);
                break;
            }
        }

        let parent_fn_ctx = parent_fn_ctx_opt.ok_or(CompilerError::Unknown)?;
        let ret_type = parent_fn_ctx.get_ret_type()?;
        let ret_size = self.get_size_of_type(&ret_type)?;
        let mut pop_size = stack_size;
        let stack_begin_offset = -(stack_size as i16);

        match ret_type {
            Type::Bool => {},
            Type::Int => {},
            Type::Reference(inner) => {
                match inner.deref() {
                    Type::AutoArray(_) => {
                        pop_size -= 16;
                        // Instruction for saving the return value
                        let mov_stack_instr = Instruction::new(Opcode::MOVN_A)
                            .with_operand::<u8>(Register::SP.into())
                            .with_operand::<i16>(-16)
                            .with_operand::<u8>(Register::SP.into())
                            .with_operand::<i16>(stack_begin_offset)
                            .with_operand::<u32>(16);
                        self.builder.push_instr(mov_stack_instr);
                    },
                    _ => {}
                };
            },
            Type::Float => {},
            Type::Void => {},
            _ => {
                pop_size -= ret_size;

                // Instruction for saving the return value
                let mov_stack_instr = Instruction::new(Opcode::MOVN_A)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(-(ret_size as i16))
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(stack_begin_offset)
                    .with_operand::<u32>(ret_size as u32);
                self.builder.push_instr(mov_stack_instr);
            }
        };

        // Instruction for popping values off the stack
        let pop_stack_instr = Instruction::new(Opcode::SUBU_I)
            .with_operand::<u8>(Register::SP.into())
            .with_operand::<u64>(pop_size as u64)
            .with_operand::<u8>(Register::SP.into());
        self.builder.push_instr(pop_stack_instr);

        Ok(())
    }

    /// Compiles a module declaration
    pub fn compile_mod_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let (mod_name, decl_list) = match decl {
            Declaration::Module(mod_name, decl_list) => (mod_name, decl_list),
            _ => return Err(CompilerError::Unknown)
        };

        let mod_ctx = ModuleContext::new(mod_name.clone());

        let module_declared = {
            let front_mod_ctx = self.get_current_module()?;
            front_mod_ctx.modules.contains_key(mod_name)
        };

        if !module_declared {
            return Err(CompilerError::UnknownModule(mod_name.clone()));
        }

        self.push_module_context(mod_ctx);

        self.compile_decl_list(decl_list)?;

        self.pop_module_context()?;

        Ok(())
    }

    /// Compiles an impl declaration
    pub fn compile_impl_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        Ok(())
    }

    /// Compiles a statement list
    pub fn compile_stmt_list(&mut self, stmt_list: &[Statement]) -> CompilerResult<()> {
        for stmt in stmt_list.iter() {
            self.compile_stmt(stmt)?;
        }
        Ok(())
    }

    /// Compiles a statement
    pub fn compile_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        match stmt {
            Statement::VariableDecl(_) => self.compile_var_decl_stmt(stmt)?,
            Statement::Expression(_) => self.compile_expr_stmt(stmt)?,
            Statement::Return(_) => self.compile_return_stmt(stmt)?,
            Statement::If(_) => self.compile_if_stmt(stmt)?,
            Statement::While(_, _) => self.compile_while_stmt(stmt)?, 
            _ => return Err(CompilerError::Unimplemented(format!("Compilation of {:?} not implemented!", stmt)))
        };
        Ok(())
    }

    /// Compiles a variable declaration statement
    pub fn compile_var_decl_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        let var_decl_args = match stmt {
            Statement::VariableDecl(var_decl_args) => var_decl_args,
            _ => return Err(CompilerError::Unknown)
        };
        // The variable name
        let var_name = var_decl_args.name.clone();
        // The variable type
        let var_type = var_decl_args.var_type.clone();
        // Byte size of this type
        let var_size = self.get_size_of_type(&var_type)?;
        // The assignment expression
        let assignment_expr = &var_decl_args.assignment;
        // Compile said expression
        self.compile_expr(assignment_expr)?;

        // If the type can be contained in a register
        if var_size <= 8 {
            let last_reg = {
                let fn_ctx = self.get_current_function()?;
                fn_ctx.register_allocator.get_last_temp_register()?
            };
            let var_sp_offset = -(var_size as i16);
            self.inc_stack(var_size)?;
            let mov_instr = match var_type {
                Type::Int => {
                    Instruction::new(Opcode::MOVI_RA)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(var_sp_offset)
                },
                Type::Float => {
                    Instruction::new(Opcode::MOVF_RA)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(var_sp_offset)
                },
                Type::Reference(_) => {
                    Instruction::new(Opcode::MOVA_RA)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(var_sp_offset)
                },
                Type::Bool => {
                    Instruction::new(Opcode::MOVB_RA)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(var_sp_offset)
                },
                _ => return Err(CompilerError::UnknownType(var_type))
            };
            let stack_inc_instr = Instruction::new_inc_stack(var_size);
            self.builder.push_instr(stack_inc_instr);
            self.builder.push_instr(mov_instr);
            
        }
        // Otherwise, the value is already on the top of the stack.
        // Set the variable in the context.
        let fn_ctx = self.get_current_function_mut()?;
        fn_ctx.set_stack_var((var_name, var_type), (fn_ctx.stack_size - var_size) as i64)?;
        Ok(())
    }

    /// Compiles a statement expression
    pub fn compile_expr_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        Err(CompilerError::Unimplemented(format!("Statement expr compilation not implemented!")))
    }

    /// Compiles an if statement
    pub fn compile_if_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        let if_stmt_args: &IfStatementArgs = match stmt {
            Statement::If(if_stmt_args) => if_stmt_args,
            _ => return Err(CompilerError::Unknown)
        };

        // Generate an instruction tag to fill in the end of this if/else chain
        let tag_end = self.uid_generator.generate();
        // Generate an instruction tag for the next branch
        let mut tag_next = self.uid_generator.generate();

        let expr_type = self.check_expr_type(&if_stmt_args.if_expr)?;
        // Only boolean expressions are allowed
        if expr_type != Type::Bool {
            return Err(CompilerError::TypeMismatch(expr_type, Type::Bool));
        }
        // Compile the if expression
        self.compile_expr(&if_stmt_args.if_expr)?;
        // Get the register the result of this boolean expression was saved in
        let last_reg = {
            self.get_current_function()?
                .register_allocator
                .get_last_temp_register()?
        };

        // Instruction for this if expr
        let jmpf_instr = Instruction::new(Opcode::JMPF)
            .with_operand::<u8>(last_reg.into())
            .with_operand(tag_next);
        self.builder.tag(tag_next);
        self.builder.push_instr(jmpf_instr);

        // Create new weak function context
        let mut if_fn_ctx = {
            let fn_ctx = self.get_current_function()?;
            FunctionContext::new_weak(fn_ctx)?
        };
        // And push it on the stack
        self.push_function_context(if_fn_ctx);

        // Compile the if statement list
        self.compile_stmt_list(&if_stmt_args.if_block)?;

        // Pop the function context off the stack again
        if_fn_ctx = self.pop_function_context()?;

        self.compile_stack_cleanup(&if_fn_ctx)?;

        // Instruction for jumping to the end
        let jmp_end_instr = Instruction::new(Opcode::JMP)
            .with_operand(tag_end);
        self.builder.tag(tag_end);
        self.builder.push_instr(jmp_end_instr);

        if if_stmt_args.else_if_list.is_some() {
            let else_if_list = if_stmt_args.else_if_list
                .as_ref()
                .ok_or(CompilerError::Unknown)?;
            for (else_if_expr, else_if_stmt_list) in else_if_list.iter() {
                // Current instruction position
                let pos = self.builder.get_current_offset();
                // Set the last JMPF to jump to this instruction
                {
                    // Retrieve the position list
                    let jmp_next_instr_pos_list = self.builder.get_tag(&tag_next)
                        .ok_or(CompilerError::Unknown)?;
                    // Retrieve the position
                    // (Only one instruction should exist with this tag)
                    let jmp_next_instr_pos = jmp_next_instr_pos_list.get(0)
                        .ok_or(CompilerError::Unknown)?;
                    // Get the mutable reference to this instruction
                    let jmp_next_instr = self.builder.get_instr(&jmp_next_instr_pos)
                        .ok_or(CompilerError::Unknown)?;
                    
                    // Update the jump destination
                    jmp_next_instr.remove_operand_bytes(8);
                    jmp_next_instr.append_operand(pos);
                }
                // Only boolean expressions are allowed
                let expr_type = self.check_expr_type(else_if_expr)?;
                if expr_type != Type::Bool {
                    return Err(CompilerError::TypeMismatch(expr_type, Type::Bool));
                }
                // Compile the expression
                self.compile_expr(else_if_expr)?;
                // Get the result register
                let last_reg = {
                    self.get_current_function()?
                        .register_allocator
                        .get_last_temp_register()?
                };
                // Generate new tag for the next jump
                tag_next = self.uid_generator.generate();
                // Instruction for jumping to next or inside statement list
                let jmpf_instr = Instruction::new(Opcode::JMPF)
                    .with_operand::<u8>(last_reg.into())
                    .with_operand(tag_next);
                self.builder.tag(tag_next);
                self.builder.push_instr(jmpf_instr);

                // Create a new weak function context
                let mut else_if_fn_ctx = {
                    let fn_ctx = self.get_current_function()?;
                    FunctionContext::new_weak(fn_ctx)?
                };
                // and push it on the stack
                self.push_function_context(else_if_fn_ctx);

                // Compile this "else if"s statement list
                self.compile_stmt_list(else_if_stmt_list)?;

                // Pop the context off the stack again
                else_if_fn_ctx = self.pop_function_context()?;

                self.compile_stack_cleanup(&else_if_fn_ctx)?;

                // Instruction for jumping to the end
                let jmp_end_instr = Instruction::new(Opcode::JMP)
                    .with_operand(tag_end);
                self.builder.tag(tag_end);
                self.builder.push_instr(jmp_end_instr);
            }
        }

        // If an "else" block exists
        if if_stmt_args.else_block.is_some() {
            let else_stmt_list = if_stmt_args.else_block.as_ref()
                .ok_or(CompilerError::Unknown)?;
            // Set the last JMPF to jump to this instruction
            let pos = self.builder.get_current_offset();
            {
                // Retrieve the position list
                let jmp_next_instr_pos_list = self.builder.get_tag(&tag_next)
                    .ok_or(CompilerError::Unknown)?;
                // Retrieve the position
                // (Only one instruction should exist with this tag)
                let jmp_next_instr_pos = jmp_next_instr_pos_list.get(0)
                    .ok_or(CompilerError::Unknown)?;
                // Get the mutable reference to this instruction
                let jmp_next_instr = self.builder.get_instr(&jmp_next_instr_pos)
                    .ok_or(CompilerError::Unknown)?;
                    
                // Update the jump destination
                jmp_next_instr.remove_operand_bytes(8);
                jmp_next_instr.append_operand(pos);
            }

            // Create a new weak function context
            let mut else_fn_ctx = {
                let fn_ctx = self.get_current_function()?;
                FunctionContext::new_weak(fn_ctx)?
            };
            // And push it on the stack
            self.push_function_context(else_fn_ctx);

            // Compile the statement list for this else block
            self.compile_stmt_list(else_stmt_list)?;

            // Pop it off the stack again
            else_fn_ctx = self.pop_function_context()?;

            self.compile_stack_cleanup(&else_fn_ctx)?;
        } else {
            // Set the last JMPF to jump to this instruction
            let pos = self.builder.get_current_offset();
            {
                // Retrieve the position list
                let jmp_next_instr_pos_list = self.builder.get_tag(&tag_next)
                    .ok_or(CompilerError::Unknown)?;
                // Retrieve the position
                // (Only one instruction should exist with this tag)
                let jmp_next_instr_pos = jmp_next_instr_pos_list.get(0)
                    .ok_or(CompilerError::Unknown)?;
                // Get the mutable reference to this instruction
                let jmp_next_instr = self.builder.get_instr(&jmp_next_instr_pos)
                    .ok_or(CompilerError::Unknown)?;
                    
                // Update the jump destination
                jmp_next_instr.remove_operand_bytes(8);
                jmp_next_instr.append_operand(pos);
            }
        }

        // Current position is at the end of the entire if/else if/else chain
        let pos_end = self.builder.get_current_offset();

        let jmp_end_pos_list = self.builder.get_tag(&tag_end)
            .ok_or(CompilerError::Unknown)?;

        // Make all the jump instructions jump to the end properly
        for jmp_end_pos in jmp_end_pos_list.iter() {
            let jmp_instr = self.builder.get_instr(jmp_end_pos)
                .ok_or(CompilerError::Unknown)?;
            jmp_instr.remove_operand_bytes(8);
            jmp_instr.append_operand(pos_end);
        }

        Ok(())
    }

    /// Compiles a while statement
    pub fn compile_while_stmt(&mut self, _stmt: &Statement) -> CompilerResult<()> {
        Err(CompilerError::Unimplemented(format!("While statement compilation not implemented!")))
    }

    /// Compiles a break statement
    pub fn compile_break_stmt(&mut self, _stmt: &Statement) -> CompilerResult<()> {
        Err(CompilerError::Unimplemented(format!("Break statement compilation not implemented!")))
    }

    /// Compiles a continue statement
    pub fn compile_continue_stmt(&mut self, _stmt: &Statement) -> CompilerResult<()> {
        Err(CompilerError::Unimplemented(format!("Continue statement compilation not implemented!")))
    }

    /// Compiles a return statement
    pub fn compile_return_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        let return_expr_opt = match stmt {
            Statement::Return(ret_expr) => ret_expr,
            _ => return Err(CompilerError::Unknown)
        };

        let mut return_expr_type = Type::Void;

        if return_expr_opt.is_some() {
            let return_expr_ref = return_expr_opt.as_ref().unwrap();
            return_expr_type = self.check_expr_type(return_expr_ref)?;
        }

        let fn_ret_type = {
            let fn_ctx = self.get_parent_function()?;
            fn_ctx.get_ret_type()?
        };

        if fn_ret_type != return_expr_type {
            return Err(CompilerError::TypeMismatch(fn_ret_type, return_expr_type));
        }

        if return_expr_opt.is_some() {
            let return_expr = return_expr_opt.as_ref().unwrap();
            self.compile_expr(return_expr)?;

            // Move to R0 register if type is primitive
            match fn_ret_type {
                Type::Int => {
                    let last_reg = {
                        let fn_ctx = self.get_current_function()?;
                        fn_ctx.register_allocator.get_last_temp_register()?
                    };
                    // Instruction for doing so
                    let mov_ret_instr = Instruction::new(Opcode::MOVI)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::R0.into());
                    self.builder.push_instr(mov_ret_instr);
                },
                Type::Float => {
                    let last_reg = {
                        let fn_ctx = self.get_current_function()?;
                        fn_ctx.register_allocator.get_last_temp_register()?
                    };
                    // Instruction for doing so
                    let mov_ret_instr = Instruction::new(Opcode::MOVF)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::R0.into());
                    self.builder.push_instr(mov_ret_instr);
                },
                Type::Bool => {
                    let last_reg = {
                        let fn_ctx = self.get_current_function()?;
                        fn_ctx.register_allocator.get_last_temp_register()?
                    };
                    // Instruction for doing so
                    let mov_ret_instr = Instruction::new(Opcode::MOVB)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::R0.into());
                    self.builder.push_instr(mov_ret_instr);
                },
                Type::Reference(inner) => {
                    match inner.deref() {
                        Type::AutoArray(_) => {},
                        _ => {
                            let last_reg = {
                                let fn_ctx = self.get_current_function()?;
                                fn_ctx.register_allocator.get_last_temp_register()?
                            };
                            // Instruction for doing so
                            let mov_ret_instr = Instruction::new(Opcode::MOVA)
                                .with_operand::<u8>(last_reg.into())
                                .with_operand::<u8>(Register::R0.into());
                            self.builder.push_instr(mov_ret_instr);
                        },
                    };
                },
                _ => {}
            };
        }

        // Clean up the stack.
        self.compile_stack_cleanup_and_return()?;

        // Add the RET function
        let ret_instr = Instruction::new(Opcode::RET);
        self.builder.push_instr(ret_instr);

        Ok(())
    }

    /// Compiles a variable assign statement expression
    pub fn compile_var_assign_expr_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        let assign_expr = match stmt {
            Statement::Expression(expr) => expr,
            _ => return Err(CompilerError::Unknown)
        };

        let (lhs_expr, rhs_expr) = match assign_expr {
            Expression::Assign(lhs, rhs) => (lhs.deref().clone(), rhs.deref().clone()),
            Expression::AddAssign(lhs, rhs) => {
                let rhs_expr = Expression::Addition(lhs.clone(), rhs.clone());
                (lhs.deref().clone(), rhs_expr)
            },
            Expression::SubAssign(lhs, rhs) => {
                let rhs_expr = Expression::Addition(lhs.clone(), rhs.clone());
                (lhs.deref().clone(), rhs_expr)
            },
            Expression::DivAssign(lhs, rhs) => {
                let rhs_expr = Expression::Addition(lhs.clone(), rhs.clone());
                (lhs.deref().clone(), rhs_expr)
            },
            Expression::MulAssign(lhs, rhs) => {
                let rhs_expr = Expression::Addition(lhs.clone(), rhs.clone());
                (lhs.deref().clone(), rhs_expr)
            },
            _ => return Err(CompilerError::Unknown)
        };

        // Compile the left hand side of this expression
        let lhs_expr_type = self.compile_lhs_assign_expr(&lhs_expr)?;
        // Get the result register
        // And block it from use
        let lhs_reg = {
            let fn_ctx = self.get_current_function_mut()?;
            let lhs_reg = fn_ctx.register_allocator.get_last_temp_register()?;
            fn_ctx.register_allocator.block_register(lhs_reg.clone())?;
            lhs_reg
        };

        // Check the type of the rhs expression
        let rhs_expr_type = self.check_expr_type(&rhs_expr)?;

        // Check for type mismatch
        if lhs_expr_type != rhs_expr_type {
            return Err(CompilerError::TypeMismatch(lhs_expr_type, rhs_expr_type));
        }

        // Compile the right hand of this expression
        self.compile_expr(&rhs_expr)?;

        // Last register used may contain the assignment value
        let rhs_reg = {
            let fn_ctx = self.get_current_function()?;
            fn_ctx.register_allocator.get_last_temp_register()?
        };

        // Move the value to the assignment destination
        let assign_instr = match rhs_expr_type {
            Type::Int => {
                Instruction::new(Opcode::MOVI_RA)
                    .with_operand::<u8>(rhs_reg.into())
                    .with_operand::<u8>(lhs_reg.into())
                    .with_operand::<i16>(0)
            },
            Type::Float => {
                Instruction::new(Opcode::MOVF_RA)
                    .with_operand::<u8>(rhs_reg.into())
                    .with_operand::<u8>(lhs_reg.into())
                    .with_operand::<i16>(0)
            },
            Type::Bool => {
                Instruction::new(Opcode::MOVB_RA)
                    .with_operand::<u8>(rhs_reg.into())
                    .with_operand::<u8>(lhs_reg.into())
                    .with_operand::<i16>(0)
            },
            Type::Reference(inner) => {
                match inner.deref() {
                    Type::AutoArray(_) => {
                        Instruction::new(Opcode::MOVN_A)
                            .with_operand::<u8>(Register::SP.into())
                            .with_operand::<i16>(-16)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<i16>(0)
                            .with_operand::<u32>(16)
                    },
                    _ => {
                        Instruction::new(Opcode::MOVA_RA)
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<i16>(0)
                    }
                }
            },
            _ => {
                let size = self.get_size_of_type(&rhs_expr_type)?;
                Instruction::new(Opcode::MOVN_A)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(-(size as i16))
                    .with_operand::<u8>(lhs_reg.into())
                    .with_operand::<i16>(0)
                    .with_operand::<u32>(size as u32)
            }
        };

        self.builder.push_instr(assign_instr);


        Err(CompilerError::Unimplemented(format!("Var assign compilation not implemented!")))
    }

    /// Compiles the left hand side of an assignment expression
    pub fn compile_lhs_assign_expr(&mut self, expr: &Expression) -> CompilerResult<Type> {
        let expr_type = match expr {
            Expression::Variable(var_name) => {
                let stack_offset = self.get_sp_offset_of_var(var_name)?.abs() as u64;
                let target_reg = {
                    let fn_ctx = self.get_current_function_mut()?;
                    fn_ctx.register_allocator.get_temp_register()?
                };
                // Instruction for assign
                let stack_offset_instr = Instruction::new(Opcode::SUBU_I)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<u64>(stack_offset)
                    .with_operand::<u8>(target_reg.into());
                self.builder.push_instr(stack_offset_instr);
                self.get_type_of_var(var_name)?
            },
            _ => return Err(CompilerError::UnsupportedExpression(expr.clone()))
        };
        Ok(expr_type)
    }

    /// Compiles an expression
    pub fn compile_expr(&mut self, expr: &Expression) -> CompilerResult<()> {
        match expr {
            Expression::IntLiteral(int) => {
                let reg = {
                    let fn_ctx = self.get_current_function_mut()?;
                    fn_ctx.register_allocator.get_temp_register()?
                };

                let ldi_instr = Instruction::new(Opcode::LDI)
                    .with_operand::<i64>(*int)
                    .with_operand::<u8>(reg.into());

                self.builder.push_instr(ldi_instr);
            },
            Expression::FloatLiteral(float) => {
                let reg = {
                    let fn_ctx = self.get_current_function_mut()?;
                    fn_ctx.register_allocator.get_temp_register()?
                };

                let ldf_instr = Instruction::new(Opcode::LDF)
                    .with_operand::<f32>(*float)
                    .with_operand::<u8>(reg.into());
                    
                self.builder.push_instr(ldf_instr);
            },
            Expression::BoolLiteral(boolean) => {
                let reg = {
                    let fn_ctx = self.get_current_function_mut()?;
                    fn_ctx.register_allocator.get_temp_register()?
                };

                let ldb_instr = Instruction::new(Opcode::LDB)
                    .with_operand::<bool>(*boolean)
                    .with_operand::<u8>(reg.into());
                    
                self.builder.push_instr(ldb_instr);
            },
            Expression::StringLiteral(string) => {
                let (string_size, string_addr) = self.data.get_string_slice(string);
                let stack_inc_instr = Instruction::new_inc_stack(16);
                let size_lda_instr = Instruction::new(Opcode::LDA)
                    .with_operand(string_size)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(-16);
                let addr_lda_instr = Instruction::new(Opcode::LDA)
                    .with_operand(string_addr)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(-8);
                self.inc_stack(16)?;
                self.builder.push_instr(stack_inc_instr);
                self.builder.push_instr(size_lda_instr);
                self.builder.push_instr(addr_lda_instr);
            },
            Expression::Variable(_) => {
                self.compile_var_expr(expr)?;
            },
            Expression::Addition(lhs, rhs) => {
                let expr_type = self.check_expr_type(lhs)?;
                self.compile_expr(lhs)?;
                let lhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                self.compile_expr(rhs)?;
                let rhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                match expr_type {
                    Type::Int => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let addi_instr = Instruction::new(Opcode::ADDI)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(addi_instr);
                    },
                    Type::Float => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let addf_instr = Instruction::new(Opcode::ADDF)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(addf_instr);
                    },
                    _ => return Err(CompilerError::UnsupportedExpression(lhs.deref().clone()))
                };
            },
            Expression::Subtraction(lhs, rhs) => {
                let expr_type = self.check_expr_type(lhs)?;
                self.compile_expr(lhs)?;
                let lhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                self.compile_expr(rhs)?;
                let rhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                match expr_type {
                    Type::Int => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let subi_instr = Instruction::new(Opcode::SUBI)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(subi_instr);
                    },
                    Type::Float => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let subf_instr = Instruction::new(Opcode::SUBF)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(subf_instr);
                    },
                    _ => return Err(CompilerError::UnsupportedExpression(lhs.deref().clone()))
                };
            },
            Expression::Multiplication(lhs, rhs) => {
                let expr_type = self.check_expr_type(lhs)?;
                self.compile_expr(lhs)?;
                let lhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                self.compile_expr(rhs)?;
                let rhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                match expr_type {
                    Type::Int => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let muli_instr = Instruction::new(Opcode::MULI)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(muli_instr);
                    },
                    Type::Float => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let mulf_instr = Instruction::new(Opcode::MULF)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(mulf_instr);
                    },
                    _ => return Err(CompilerError::UnsupportedExpression(lhs.deref().clone()))
                };
            },
            Expression::Division(lhs, rhs) => {
                let expr_type = self.check_expr_type(lhs)?;
                self.compile_expr(lhs)?;
                let lhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                self.compile_expr(rhs)?;
                let rhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                match expr_type {
                    Type::Int => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let divi_instr = Instruction::new(Opcode::DIVI)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(divi_instr);
                    },
                    Type::Float => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let divf_instr = Instruction::new(Opcode::DIVF)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(divf_instr);
                    },
                    _ => return Err(CompilerError::UnsupportedExpression(lhs.deref().clone()))
                };
            },
            Expression::LessThan(lhs, rhs) => {
                let expr_type = self.check_expr_type(lhs)?;
                self.compile_expr(lhs)?;
                let lhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                self.compile_expr(rhs)?;
                let rhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                match expr_type {
                    Type::Int => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let divi_instr = Instruction::new(Opcode::LTI)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(divi_instr);
                    },
                    Type::Float => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let divf_instr = Instruction::new(Opcode::LTF)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(divf_instr);
                    },
                    _ => return Err(CompilerError::UnsupportedExpression(lhs.deref().clone()))
                };
            },
            _ => return Err(CompilerError::UnsupportedExpression(expr.clone()))
        };
        Ok(())
        //Err(CompilerError::Unimplemented(format!("Expr compilation not implemented!")))
    }

    /// Compiles a variable expression
    pub fn compile_var_expr(&mut self, expr: &Expression) -> CompilerResult<()> {
        let var_name = match expr {
            Expression::Variable(var_name) => var_name,
            _ => return Err(CompilerError::Unknown)
        };

        let var_type = self.get_type_of_var(var_name)?;
        let var_offset = self.get_sp_offset_of_var(var_name)?;
        match var_type {
            Type::Int => {
                let reg = {
                    let fn_ctx = self.get_current_function_mut()?;
                    fn_ctx.register_allocator.get_temp_register()?
                };
                let movi_instr = Instruction::new(Opcode::MOVI_AR)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(var_offset as i16)
                    .with_operand::<u8>(reg.into());
                self.builder.push_instr(movi_instr);
            },
            Type::Float => {
                let reg = {
                    let fn_ctx = self.get_current_function_mut()?;
                    fn_ctx.register_allocator.get_temp_register()?
                };
                let movf_instr = Instruction::new(Opcode::MOVF_AR)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(var_offset as i16)
                    .with_operand::<u8>(reg.into());
                self.builder.push_instr(movf_instr);
            },
            Type::Bool => {
                let reg = {
                    let fn_ctx = self.get_current_function_mut()?;
                    fn_ctx.register_allocator.get_temp_register()?
                };
                let movb_instr = Instruction::new(Opcode::MOVB_AR)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(var_offset as i16)
                    .with_operand::<u8>(reg.into());
                self.builder.push_instr(movb_instr);
            },
            Type::Reference(inner_type) => {
                match inner_type.deref() {
                    Type::AutoArray(_) => {
                        let stack_inc_instr = Instruction::new_inc_stack(16);
                        self.inc_stack(16)?;
                        let var_offset = self.get_sp_offset_of_var(var_name)?;
                        let movn_instr = Instruction::new(Opcode::MOVN_A)
                            .with_operand::<u8>(Register::SP.into())
                            .with_operand::<i16>(var_offset as i16)
                            .with_operand::<u8>(Register::SP.into())
                            .with_operand::<i16>(-16);
                        self.builder.push_instr(stack_inc_instr);
                        self.builder.push_instr(movn_instr);
                    },
                    _ => {
                        let reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let mova_instr = Instruction::new(Opcode::MOVA_AR)
                            .with_operand::<u8>(Register::SP.into())
                            .with_operand::<i16>(var_offset as i16)
                            .with_operand::<u8>(reg.into());
                        self.builder.push_instr(mova_instr);
                    }
                };
            },
            Type::Other(cont_name) => {
                return Err(CompilerError::Unimplemented(format!("Containers not implemented yet!")))
            },
            _ => return Err(CompilerError::UnknownType(var_type)),
        };

        Ok(())
    }

    /// Returns the type of an expression and checks for type mismatches
    pub fn check_expr_type(&self, expr: &Expression) -> CompilerResult<Type> {
        let expr_type = match expr {
            Expression::IntLiteral(_) => Type::Int,
            Expression::FloatLiteral(_) => Type::Float,
            Expression::BoolLiteral(_) => Type::Bool,
            Expression::StringLiteral(_) => Type::String,
            Expression::Variable(var_name) => {
                self.get_type_of_var(var_name)?
            },
            Expression::Assign(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                lhs_type
            },
            Expression::Addition(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                lhs_type
            },
            Expression::Subtraction(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                lhs_type
            },
            Expression::Multiplication(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                lhs_type
            },
            Expression::Division(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                lhs_type
            },
            Expression::LessThan(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                Type::Bool
            },
            Expression::GreaterThan(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                Type::Bool
            },
            Expression::LessThanEquals(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                Type::Bool
            },
            Expression::GreaterThanEquals(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                Type::Bool
            },
            Expression::Equals(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                Type::Bool
            },
            Expression::NotEquals(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                Type::Bool
            },
            Expression::Not(op) => {
                let op_type = self.check_expr_type(op)?;
                if Type::Bool != op_type {
                    return Err(CompilerError::TypeMismatch(Type::Bool, op_type));
                }
                Type::Bool
            },
            _ => return Err(CompilerError::UnsupportedExpression(expr.clone()))
        };
        Ok(expr_type)
        //Err(CompilerError::Unimplemented(format!("Expr type checking not implemented!")))
    }

    // #endregion
}