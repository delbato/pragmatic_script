use crate::{
    parser::{
        ast::*
    },
    vm::{
        is::Opcode
    },
    api::{
        function::{
            FunctionResult,
            FunctionError,
            Function
        },
        module::{
            Module
        }
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
        ModuleContext,
        LoopContext,
        LoopType
    },
    container::{
        ContainerDef,
        ContainerMemberDef
    },
    program::Program,
    data::Data
};

use std::{
    collections::{
        VecDeque,
        HashMap,
        HashSet,
        BTreeMap
    },
    error::Error,
    fmt::{
        Display,
        Formatter,
        Result as FmtResult
    },
    ops::Deref
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
    loop_context_stack: VecDeque<LoopContext>,
    pub builder: Builder,
    pub data: Data,
    function_uid_map: HashMap<String, u64>,
    function_uid_set: HashSet<u64>,
    foreign_function_set: HashSet<u64>,
    loop_uid_set: HashSet<u64>,
    tag_set: HashSet<u64>,
    current_cont: String
}

pub type CompilerResult<T> = Result<T, CompilerError>;

#[derive(Debug)]
pub enum CompilerError {
    Unknown,
    UnknownType,
    UnknownFunction,
    UnknownModule,
    UnknownContainer,
    UnknownContainerFunction,
    UnknownMemberFunction,
    NotImplemented,
    UnknownVariable,
    TypeMismatch,
    DuplicateFunctionName,
    DuplicateModule,
    DuplicateStruct,
    InvalidArgumentCount,
    IfOnlyAcceptsBooleanExpressions,
    WhileOnlyAcceptsBooleanExpressions,
    ExpectedBreak,
    ExpectedContinue,
    UnsupportedStatementExpression,
    OnlyFunctionsAllowedInContImpl
}

impl Display for CompilerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}

impl Error for CompilerError {}

impl Compiler {
    /// Creates a new compiler instance
    pub fn new() -> Compiler {
        let comp = Compiler {
            mod_context_stack: VecDeque::new(),
            global_context: FunctionContext::new(),
            fn_context_stack: VecDeque::new(),
            loop_context_stack: VecDeque::new(),
            builder: Builder::new(),
            function_uid_map: HashMap::new(),
            function_uid_set: HashSet::new(),
            foreign_function_set: HashSet::new(),
            loop_uid_set: HashSet::new(),
            tag_set: HashSet::new(),
            data: Data::new(),
            current_cont: String::new()
        };
        comp
    }

    /// Registers a new foreign module in the compiler.
    /// 
    /// Params:
    /// * `module`: The foreign module to register in the engine
    /// * `parent_path`: The module to recursively register this module under
    /// 
    /// Returns:  
    /// Empty result if registered successfully, errors otherwise.
    pub fn register_foreign_module(&mut self, module: &mut Module, parent_path: String) -> CompilerResult<()> {
        let mod_name = module.name.clone();
        let mut path;
        if parent_path.len() > 0 {
            path = parent_path + "::" + &mod_name;
        } else {
            let curr_mod_name = {
                let mod_front_ctx = self.mod_context_stack.get(0)
                    .ok_or(CompilerError::Unknown)?;
                mod_front_ctx.name.clone()
            };

            path = curr_mod_name + "::" + &mod_name;
        }

        let mut mod_context = ModuleContext::new(mod_name);

        for function in module.functions.iter_mut() {
            let mut full_fn_name = path.clone();
            full_fn_name += "::";
            full_fn_name += &function.name; 
            
            let function_name = function.name.clone();
            let function_uid = self.get_function_uid(&full_fn_name);
            let fn_return_type = function.return_type
                .as_ref()
                .cloned()
                .ok_or(CompilerError::Unknown)?;

            let mut arg_bmap = BTreeMap::new();
            for i in 0..function.arguments.len() {
                let arg_type = function.arguments.get(i)
                    .cloned()
                    .ok_or(CompilerError::Unknown)?;
                arg_bmap.insert(i, (String::new(), arg_type));
            }
            let fn_tuple = (function_uid, fn_return_type, arg_bmap);
            mod_context.functions.insert(function_name, fn_tuple);
            self.foreign_function_set.insert(function_uid.clone());
            function.uid = Some(function_uid);
        }

        self.mod_context_stack.push_front(mod_context);

        for (_, module) in module.modules.iter_mut() {
            self.register_foreign_module(module, path.clone())?;
        }

        mod_context = self.mod_context_stack.pop_front()
            .ok_or(CompilerError::Unknown)?;

        let front_mod_ctx = self.mod_context_stack.get_mut(0)
            .ok_or(CompilerError::UnknownModule)?;

        ////println!"Registering module {} in module {}", mod_context.name, front_mod_ctx.name);

        front_mod_ctx.modules.insert(mod_context.name.clone(), mod_context);

        Ok(())
    }

    /// Pushes a loop context on the loop context stack
    /// 
    /// Params:
    /// * `ctx`: LoopContext to push on the stack
    pub fn push_loop_context(&mut self, ctx: LoopContext) {
        self.loop_context_stack.push_front(ctx);
    }

    /// Pops the current loop context off the stack
    /// 
    /// Returns:  
    /// A LoopContext
    pub fn pop_loop_context(&mut self) -> CompilerResult<LoopContext> {
        self.loop_context_stack.pop_front()
            .ok_or(CompilerError::Unknown)
    }

    /// Gets a reference to the current loop context
    /// 
    /// Returns:  
    /// Reference to current loop context
    pub fn get_current_loop_context(&self) -> CompilerResult<&LoopContext> {
        self.loop_context_stack.get(0)
            .ok_or(CompilerError::Unknown)
    }

    /// Gets a mutable reference to the current loop context
    /// 
    /// Returns:  
    /// Mutable reference to current loop context
    pub fn get_current_loop_context_mut(&mut self) -> CompilerResult<&mut LoopContext> {
        self.loop_context_stack.get_mut(0)
            .ok_or(CompilerError::Unknown)
    }

    /// Resolves a container name to a container definition
    /// 
    /// Returns:  
    /// Result containing the ContainerDef if found, errors otherwise
    pub fn resolve_cont(&self, name: &String) -> CompilerResult<ContainerDef> {
        // If directly accessing via module namespace
        if name.contains("::") {
            ////println!"Module accessor!");
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
                ////println!"Accessing from current module...");
                mod_ctx = self.get_current_module()?;
                offset = 0;
            }

            let canonical_cont_name = String::from(path[path.len() - 1]);

            // Iterate from the offset (in case of super and root, 1)  
            // To the second last element (as the last is the function name)
            for i in offset..path.len() - 1 {
                let mod_name = path[i];
                mod_ctx = mod_ctx.modules.get(&String::from(mod_name))
                    .ok_or(CompilerError::Unknown)?;
            }

            mod_ctx.containers.get(&canonical_cont_name)
                .cloned()
                .ok_or(CompilerError::UnknownContainer)
        }
        // If accessing relative to this module
        else {
            let mod_ctx = self.get_current_module()?;
            // If declared in this module
            if let Some(cont) = mod_ctx.containers.get(name) {
                return Ok(cont.clone());
            }
            // If imported from other module
            else if let Some(module_path) = mod_ctx.imports.get(name) {
                return self.resolve_cont(module_path);
            }
            // Otherwise, the function is unknown.
            else {
                return Err(CompilerError::UnknownContainer);
            }
        }
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
    /// * `name`: name of the function
    /// ### Returns
    /// A Result containing the function data, errors otherwise
    pub fn resolve_fn(&self, name: &String) -> CompilerResult<(u64, Type, BTreeMap<usize, (String, Type)>)> {
        // If directly accessing via module namespace
        if name.contains("::") {
            ////println!"Module accessor!");
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
                ////println!"Accessing from current module...");
                mod_ctx = self.get_current_module()?;
                offset = 0;
            }

            let canonical_fn_name = String::from(path[path.len() - 1]);

            // Iterate from the offset (in case of super and root, 1)  
            // To the second last element (as the last is the function name)
            for i in offset..path.len() - 1 {
                let mod_name = path[i];
                mod_ctx = mod_ctx.modules.get(&String::from(mod_name))
                    .ok_or(CompilerError::Unknown)?;
            }

            ////println!"Getting function {} from module {}...", canonical_fn_name, mod_ctx.name);
            ////println!"Module {} fn decls: {}", mod_ctx.name, mod_ctx.functions.len());

            return mod_ctx.functions.get(&canonical_fn_name)
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

    /// Gets a reference to the first parent non-weak FunctionContext and its index
    /// 
    /// Returns:  
    /// A Result containing a Tuple with the index and a reference to the FunctionContext
    pub fn get_parent_fn(&self) -> CompilerResult<(usize, &FunctionContext)> {
        let mut fn_opt = None;

        let mut index = 0;

        for i in 0..self.fn_context_stack.len() {
            fn_opt = self.fn_context_stack.get(i);
            
            if let Some(fn_context) = fn_opt {
                if !fn_context.weak {
                    index = i;
                    break;
                }
            }
        }

        let ctx = fn_opt.ok_or(CompilerError::UnknownFunction)?;
        Ok((index, ctx))
    }

    /// Gets a mutable reference to the first parent non-weak FunctionContext and its index
    /// 
    /// Returns:  
    /// A Result containing a Tuple with the index and a mutable reference to the FunctionContext
    pub fn get_parent_fn_mut(&mut self) -> CompilerResult<(usize, &mut FunctionContext)> {
        let mut fn_opt = None;

        /*
        let mut index = 0;

        for i in 0..self.fn_context_stack.len() {
            let fn_opt_temp = self.fn_context_stack.get_mut(i);
            
            if let Some(fn_context) = fn_opt_temp {
                if !fn_context.weak {
                    index = i;
                    fn_opt = fn_opt_temp;
                    break;
                }
            }
        }
        */

        let ctx = fn_opt.ok_or(CompilerError::UnknownFunction)?;
        Ok((0, ctx))
    }

    /// Splits a module path
    pub fn get_module_path<'s>(&self, name: &'s String) -> Vec<&'s str> {
        name.split("::").collect()
    }

    /// Retrieves the root module, which is always at the top of the stack
    pub fn get_root_module(&self) -> CompilerResult<&ModuleContext> {
        if self.mod_context_stack.len() == 0 {
            return Err(CompilerError::Unknown);
        }
        self.mod_context_stack.get(self.mod_context_stack.len() - 1)
            .ok_or(CompilerError::Unknown)
    }

    /// Retrieves the direct parent module
    pub fn get_super_module(&self) -> CompilerResult<&ModuleContext> {
        if self.mod_context_stack.len() < 2 {
            return Err(CompilerError::Unknown);
        }
        self.mod_context_stack.get(1)
            .ok_or(CompilerError::Unknown)
    }

    /// Retrieves the current module
    pub fn get_current_module(&self) -> CompilerResult<&ModuleContext> {
        if self.mod_context_stack.len() == 0 {
            return Err(CompilerError::Unknown);
        }
        self.mod_context_stack.get(0)
            .ok_or(CompilerError::Unknown)
    }

    /// Retrieves a mutable reference to the current module
    pub fn get_current_module_mut(&mut self) -> CompilerResult<&mut ModuleContext> {
        if self.mod_context_stack.len() == 0 {
            return Err(CompilerError::Unknown);
        }
        self.mod_context_stack.get_mut(0)
            .ok_or(CompilerError::Unknown)
    }

    /// Gets a copy of the current function context
    pub fn get_context(&mut self) -> Option<FunctionContext> {
        self.fn_context_stack.get(0).cloned()
    }

    /// Pushes a function context on the stack
    pub fn push_new_context(&mut self, context: FunctionContext) {
        self.fn_context_stack.push_front(context);
    }

    /// Pushes a new empty function context on the stack
    pub fn push_empty_context(&mut self) {
        self.fn_context_stack.push_front(FunctionContext::new());
    }

    /// Pushes the root module context on the stack
    pub fn push_default_module_context(&mut self) {
        self.mod_context_stack.push_front(
            ModuleContext::new(String::from("root"))
        );
    }

    /// Pops the current module context off the stack
    pub fn pop_module_context(&mut self) -> Option<ModuleContext> {
        self.mod_context_stack.pop_front()
    }

    /// DEPRECATED: Reset global function context
    pub fn reset_global(&mut self) {
        self.global_context = FunctionContext::new();
    }

    /// Returns the byte size of a given Type
    pub fn size_of_type(&self, var_type: &Type) -> CompilerResult<usize> {
        let size = match var_type {
            Type::Int => 8,
            Type::Float => 4,
            Type::String => 16,
            Type::Bool => 1,
            Type::Void => 0,
            Type::Reference(inner_type) => match *(*inner_type) {
                Type::AutoArray(_) => 16,
                _ => 8
            },
            _ => {
                return Err(CompilerError::UnknownType);
            }
        };
        Ok(size)
    }

    /// Returns the type of a variable
    pub fn type_of_var(&self, var_name: &String) -> CompilerResult<Type> {
        let front_context = self.fn_context_stack.get(0)
            .ok_or(CompilerError::UnknownVariable)?;
        let var_type = front_context.variable_types.get(var_name)
            .ok_or(CompilerError::UnknownVariable)?;
        Ok(var_type.clone())
    }

    /// Returns the type of a function
    pub fn type_of_fn(&self, fn_name: &String) -> CompilerResult<Type> {
        let (_, fn_type, _) = self.resolve_fn(fn_name)?;
        Ok(
            fn_type
        )
    }

    /// Get resulting instruction code
    pub fn get_resulting_code(&mut self) -> Vec<u8> {
        let builder = self.builder.clone();
        builder.build()
    }

    /// Get reference to instruction builder
    pub fn get_builder_ref(&self) -> &Builder {
        &self.builder
    }

    /// Build and get the program
    pub fn get_program(&mut self) -> CompilerResult<Program> {
        let mut builder = self.builder.clone();
        let mut functions = HashMap::new();

        let mut data = self.data.get_bytes();
        //println!("Data length: {}", data.len());
        let pointers = self.data.get_pointers();

        for (fn_name, fn_uid) in self.function_uid_map.iter() {
            if self.foreign_function_set.contains(fn_uid) {
                continue;
            }
            let mut fn_offset = builder.get_label_offset(fn_name)
                .ok_or(CompilerError::UnknownFunction)?;

            fn_offset += data.len();
            functions.insert(*fn_uid, fn_offset);
        }

        // Update JMP Instructions
        for instr_offset in builder.jmp_instructions.iter() {
            let instr = builder.instructions.get_mut(*instr_offset)
                .ok_or(CompilerError::Unknown)?;
            let mut jmp_addr: u64 = instr.get_operand();
            jmp_addr += data.len() as u64;
            instr.clear_operands();
            instr.append_operand(&jmp_addr);
        }

        //println!("Instructions:");
        let mut offset = 0;
        for instr in builder.instructions.iter() {
            //println!("offset {}: {:?}", offset, instr);
            offset += instr.get_size();
        }

        let mut code = builder.build();
        data.append(&mut code);

        let program = Program::new()
            .with_code(data)
            .with_functions(functions)
            .with_static_pointers(pointers);

        Ok(program)
    }

    /// Get or generate a new, unique tag
    pub fn get_tag(&mut self) -> u64 {
        let mut rng = thread_rng();
        let mut tag = rng.next_u64();
        while self.tag_set.contains(&tag) {
            tag = rng.next_u64();
        }
        tag
    }

    /// Get or generate a new, unique function uid
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

    /// Get or generate a new, unique loop uid
    pub fn get_loop_uid(&mut self) -> u64 {
        let mut rng = thread_rng();
        let mut uid = rng.next_u64();
        while self.loop_uid_set.contains(&uid) {
            uid = rng.next_u64();
        }
        uid
    }

    /// Get full function name from local name, by prefixing parent modules
    pub fn get_full_function_name(&mut self, function_name: &String) -> String {
        let mut full_fn_name = String::new();

        for module in self.mod_context_stack.iter().rev() {
            full_fn_name += &module.name;
            full_fn_name += "::";
        }

        if !self.current_cont.is_empty() {
            full_fn_name += &self.current_cont;
        }

        full_fn_name += function_name;

        full_fn_name
    }

    /// Reset the internal builder
    pub fn reset_builder(&mut self) {
        self.builder = Builder::new();
    }

    /// Pre-Declare a declaration list
    pub fn decl_decl_list(&mut self, decl_list: &Vec<Declaration>) -> CompilerResult<()> {
        ////println!"Declaring decl list for current module {}...", mod_name);
        for decl in decl_list.iter() {
            self.decl_decl(decl)?;
        }
        ////println!"Done declaring decl list for current module {}.", mod_name);
        Ok(())
    }

    /// Declare declarations
    pub fn decl_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        match decl {
            Declaration::Function(_) => self.decl_fn_decl(decl)?,
            Declaration::Module(_, _) => self.decl_mod_decl(decl)?,
            Declaration::Container(_) => self.decl_cont_decl(decl)?,
            Declaration::Import(_, _) => self.decl_import_decl(decl)?,
            Declaration::Impl(_, _, _) => self.decl_impl_decl(decl)?,
            _ => {}
        };
        Ok(())
    }

    pub fn decl_impl_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let (impl_type, impl_for, decl_list) = match decl {
            Declaration::Impl(impl_type, impl_for, decl_list) => (impl_type, impl_for, decl_list),
            _ => return Err(CompilerError::Unknown)
        };

        let cont_declared = {
            self.get_current_module()?
                .containers
                .get(impl_type)
                .is_some()
        };
        if !cont_declared {
            let mod_ctx = self.get_current_module_mut()?;
            let container_def = ContainerDef::new(impl_type.clone());
            mod_ctx.containers.insert(impl_type.clone(), container_def);
        }
        self.current_cont = impl_type.clone();

        self.decl_cont_decl_list(&decl_list)?;

        self.current_cont = String::new();

        Ok(())
    }

    pub fn decl_cont_decl_list(&mut self, decl_list: &[Declaration]) -> CompilerResult<()> {
        for decl in decl_list.iter() {
            match decl {
                Declaration::Function(_) => self.decl_fn_decl(decl)?,
                _ => return Err(CompilerError::OnlyFunctionsAllowedInContImpl)
            };
        }
        Ok(())
    }

    /// Declare import declaration
    pub fn decl_import_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let (import_path, import_name) = match decl {
            Declaration::Import(import_path, import_name) => (import_path.clone(), import_name.clone()),
            _ => return Err(CompilerError::Unknown)
        };

        let mod_name = {
            self.get_current_module()?.name.clone()
        };

        ////println!"Declaring import({} as {}) for current module {}!", import_path, import_name, mod_name);

        let mod_ctx = self.get_current_module_mut()?;
        mod_ctx.imports.insert(import_name, import_path);

        Ok(())
    }

    /// Declare function declaration
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
        
        if self.current_cont.is_empty() {
            let insert_opt = front_mod_ctx.functions.insert(fn_decl_args.name.clone(), fn_tuple);
            if insert_opt.is_some() {
                return Err(CompilerError::DuplicateFunctionName);
            }
        } else {
            let container_def = front_mod_ctx.containers.get_mut(&self.current_cont)
                .ok_or(CompilerError::UnknownContainer)?;
            container_def.add_function(fn_decl_args.name.clone(), fn_tuple)?;
        }

        Ok(())
    }

    pub fn decl_cont_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let cont_decl_args = match decl {
            Declaration::Container(cont_decl_args) => cont_decl_args,
            _ => return Err(CompilerError::Unknown)
        };

        let cont_name = cont_decl_args.name.clone();

        let mod_ctx = self.get_current_module_mut()?;

        if !mod_ctx.containers.contains_key(&cont_name) {
            let container = ContainerDef::new(cont_name.clone());
            mod_ctx.containers.insert(cont_name.clone(), container);
        }

        let container = mod_ctx.containers.get_mut(&cont_name)
            .ok_or(CompilerError::UnknownContainer)?;

        for (i, (var_name, var_type)) in cont_decl_args.members.iter() {
            let member = ContainerMemberDef::new(var_name.clone(), var_type.clone());
            container.members.insert(*i, member);
        }

        Ok(())
    }

    pub fn decl_mod_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let (mod_name, decl_list) = match decl {
            Declaration::Module(mod_name, decl_list) => (mod_name, decl_list),
            _ => return Err(CompilerError::Unknown)
        };
        let old_mod_name = {
            self.get_current_module()?.name.clone()
        };
        ////println!"Declaring module {} for current module {}!", mod_name, old_mod_name);
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

    pub fn compile_root_decl_list(&mut self, decl_list: Vec<Declaration>) -> CompilerResult<()> {
        self.decl_decl_list(&decl_list)?;
        self.compile_decl_list(decl_list)?;
        Ok(())
    }

    pub fn compile_decl_list(&mut self, decl_list: Vec<Declaration>) -> CompilerResult<()> {
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
            Declaration::Module(name, decl_list) => {
                let mod_ctx =  {
                    let front_mod_ctx = self.get_current_module()?;
                    front_mod_ctx.modules.get(&name)
                        .cloned()
                        .ok_or(CompilerError::UnknownModule)?
                };
                ////println!"Compiling module {} with {} function declarations!", mod_ctx.name, mod_ctx.functions.len());
                self.mod_context_stack.push_front(mod_ctx);
                self.compile_decl_list(decl_list)?;
                self.mod_context_stack.pop_front();
            },
            Declaration::Import(_, _) => {},
            Declaration::Container(_) => {},
            Declaration::Impl(_, _, _) => {
                self.compile_impl_decl(decl)?;
            },
            _ => {
                return Err(CompilerError::Unknown);
            }
        };
        Ok(())
    }

    pub fn compile_impl_decl(&mut self, decl: Declaration) -> CompilerResult<()> {
        let (impl_type, impl_for, decl_list) = match decl {
            Declaration::Impl(impl_type, impl_for, decl_list) => (impl_type, impl_for, decl_list),
            _ => return Err(CompilerError::Unknown)
        };

        self.current_cont = impl_type.clone();

        self.compile_decl_list(decl_list)?;

        self.current_cont = String::new();

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
        
        let ret_type = fn_decl_args.returns.clone();

        context.return_type = Some(fn_decl_args.returns);

        self.fn_context_stack.push_front(context);
        

        if let Some(statements) = fn_decl_args.code_block {
            self.compile_statement_list(&statements)?;
            let mut contains_return = false;
            for stmt in statements.iter() {
                if let Statement::Return(_) = stmt {
                    contains_return = true;
                }
            }
            if ret_type == Type::Void && !contains_return {
                let return_stmt = Statement::Return(None);
                self.compile_statement(&return_stmt)?;
            }
        }

        let _ = self.fn_context_stack.pop_front();
        //println!("Fn decl context: {:?}", ctx);

        Ok(())
    }

    pub fn compile_statement_list(&mut self, stmt_list: &[Statement]) -> CompilerResult<()> {
        for stmt in stmt_list.iter() {
            self.compile_statement(stmt)?;
        }
        Ok(())
    }

    pub fn compile_statement(&mut self, stmt: &Statement) -> CompilerResult<()> {
        match stmt {
            Statement::VariableDecl(_) => {
                self.compile_var_decl_stmt(stmt)?
            },
            Statement::Return(_) => {
                self.compile_return_stmt(stmt)?
            },
            Statement::If(_, _) => {
                self.compile_if_stmt(stmt)?;  
            },
            Statement::While(_, _ ) => {
                self.compile_while_stmt(stmt)?;  
            },
            Statement::Expression(_) => {
                self.compile_stmt_expr(stmt)?;
            },
            Statement::Break => self.compile_break_stmt(stmt)?,
            Statement::Continue => self.compile_continue_stmt(stmt)?,
            _ => {
                return Err(CompilerError::NotImplemented);
            }
        };

        Ok(())
    }

    pub fn compile_stmt_expr(&mut self, stmt: &Statement) -> CompilerResult<()> {
        let expr = match stmt {
            Statement::Expression(expr) => expr,
            _ => return Err(CompilerError::Unknown)
        };

        match expr {
            Expression::Assign(_, _) => {
                self.compile_var_assign_expr(expr)?;
            },
            Expression::AddAssign(lhs, rhs) => {
                let var_expr = lhs.clone();
                let add_expr = Box::new(Expression::Addition(var_expr.clone(), rhs.clone()));
                let assign_expr = Box::new(Expression::Assign(var_expr, add_expr));
                self.compile_var_assign_expr(&assign_expr)?
            },
            Expression::SubAssign(lhs, rhs) => {
                let var_expr = lhs.clone();
                let add_expr = Box::new(Expression::Subtraction(var_expr.clone(), rhs.clone()));
                let assign_expr = Box::new(Expression::Assign(var_expr, add_expr));
                self.compile_var_assign_expr(&assign_expr)?
            },
            Expression::MulAssign(lhs, rhs) => {
                let var_expr = lhs.clone();
                let add_expr = Box::new(Expression::Multiplication(var_expr.clone(), rhs.clone()));
                let assign_expr = Box::new(Expression::Assign(var_expr, add_expr));
                self.compile_var_assign_expr(&assign_expr)?
            },
            Expression::DivAssign(lhs, rhs) => {
                let var_expr = lhs.clone();
                let add_expr = Box::new(Expression::Division(var_expr.clone(), rhs.clone()));
                let assign_expr = Box::new(Expression::Assign(var_expr, add_expr));
                self.compile_var_assign_expr(&assign_expr)?
            },
            Expression::Call(_, _) => {
                self.compile_call_stmt_expr(expr)?;
            }
            _ => {
                return Err(CompilerError::UnsupportedStatementExpression);
            }
        };

        Ok(())
    }

    pub fn compile_while_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        let (while_expr, stmt_list) = match stmt {
            Statement::While(expr, list) => (expr, list),
            _ => return Err(CompilerError::Unknown)
        };

        let instr_start = self.builder.get_current_offset();
        let tag_end = self.get_tag();

        let mut loop_context = LoopContext::new(instr_start, LoopType::While);
        self.push_loop_context(loop_context);

        let expr_type = {
            let checker = Checker::new(self);
            checker.check_expr_type(while_expr)
                .map_err(|_| CompilerError::TypeMismatch)?
        };

        if expr_type != Type::Bool {
            return Err(CompilerError::WhileOnlyAcceptsBooleanExpressions);
        }

        //println!("Compiling while expr: {:?}", while_expr);

        self.compile_expr(while_expr)?;
        self.builder.tag(tag_end);

        let jmpf_instr = Instruction::new(Opcode::JMPF)
            .with_operand(&tag_end);
        
        self.builder.push_instr(jmpf_instr);
        {
            let front_context = self.fn_context_stack.get_mut(0)
                .ok_or(CompilerError::Unknown)?;
            front_context.stack_size -= 1;
            //println!("While parent context stack size: {}", front_context.stack_size);
        }

        let mut weak_context = {
            let front_context = self.fn_context_stack.get(0)
                .ok_or(CompilerError::Unknown)?;
            FunctionContext::new_weak(&front_context)
        };
        
        self.fn_context_stack.push_front(weak_context);

        for stmt in stmt_list.iter() {
            self.compile_statement(stmt)?;
        }

        weak_context = self.fn_context_stack.pop_front()
            .ok_or(CompilerError::Unknown)?;

        //println!("End of while. While context stack size: {}", weak_context.stack_size);
        
        let popn_size = weak_context.stack_size as u64;

        let popn_instr = Instruction::new(Opcode::POPN)
            .with_operand(&popn_size);

        let jmp_instr = Instruction::new(Opcode::JMP)
            .with_operand(&instr_start);
        
        self.builder.push_instr(popn_instr);
        self.builder.push_instr(jmp_instr);

        let instr_end = self.builder.get_current_offset();

        {
            let jmpf_instr = self.builder.get_tag(&tag_end)
                .ok_or(CompilerError::Unknown)?;
            jmpf_instr.clear_operands();
            jmpf_instr.append_operand(&instr_end);
        }

        loop_context = self.pop_loop_context()?;

        for tag in loop_context.break_instr_tags {
            let jmp_instr = self.builder.get_tag(&tag)
                .ok_or(CompilerError::Unknown)?;
            jmp_instr.clear_operands();
            jmp_instr.append_operand(&instr_end);
        }
        
        Ok(())
    }
    
    pub fn compile_break_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        if *stmt != Statement::Break {
            return Err(CompilerError::ExpectedBreak);
        }

        let popn_size = {
            let front_fn_ctx = self.fn_context_stack.get(0)
                .ok_or(CompilerError::UnknownFunction)?;
            front_fn_ctx.stack_size as u64
        };

        let mut front_loop_ctx = self.pop_loop_context()?;

        let popn_instr = Instruction::new(Opcode::POPN)
            .with_operand(&popn_size);

        self.builder.push_instr(popn_instr);

        let break_tag = self.get_tag();

        front_loop_ctx.add_break_tag(break_tag);

        self.builder.tag(break_tag);

        let jmp_instr = Instruction::new(Opcode::JMP)
            .with_operand(&break_tag);
        
        self.builder.push_instr(jmp_instr);

        self.push_loop_context(front_loop_ctx);

        Ok(())
    }

    pub fn compile_continue_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        if *stmt != Statement::Continue {
            return Err(CompilerError::ExpectedContinue);
        }

        let popn_size = {
            let front_fn_ctx = self.fn_context_stack.get(0)
                .ok_or(CompilerError::UnknownFunction)?;
            front_fn_ctx.stack_size as u64
        };

        let front_loop_ctx = self.pop_loop_context()?;

        let popn_instr = Instruction::new(Opcode::POPN)
            .with_operand(&popn_size);

        self.builder.push_instr(popn_instr);

        let jmp_instr = Instruction::new(Opcode::JMP)
            .with_operand(&front_loop_ctx.instr_start);
        
        self.builder.push_instr(jmp_instr);

        self.push_loop_context(front_loop_ctx);

        Ok(())
    }

    pub fn compile_loop_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        Err(CompilerError::NotImplemented)
    }

    pub fn compile_if_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        let (if_expr, stmt_list) = match stmt {
            Statement::If(if_expr, stmt_list) => (if_expr, stmt_list),
            _ => return Err(CompilerError::Unknown)
        };

        let tag = self.get_tag();
        let expr_type = {
            let checker = Checker::new(self);
            checker.check_expr_type(if_expr)
                .map_err(|_| CompilerError::TypeMismatch)?
        };

        if expr_type != Type::Bool {
            return Err(CompilerError::IfOnlyAcceptsBooleanExpressions);
        }

        self.compile_expr(if_expr)?;

        self.builder.tag(tag);

        let jmpf_instr = Instruction::new(Opcode::JMPF)
            .with_operand(&tag);
        
        self.builder.push_instr(jmpf_instr);
        {
            let front_context = self.fn_context_stack.get_mut(0)
                .ok_or(CompilerError::Unknown)?;
            front_context.stack_size -= 1;
        }

        let mut weak_context = {
            let front_context = self.fn_context_stack.get(0)
                .ok_or(CompilerError::Unknown)?;
            FunctionContext::new_weak(&front_context)
        };

        self.fn_context_stack.push_front(weak_context);
        
        for stmt in stmt_list.iter() {
            self.compile_statement(stmt)?;
        }

        weak_context = self.fn_context_stack.pop_front()
            .ok_or(CompilerError::Unknown)?;
        
        let popn_size = weak_context.stack_size as u64;

        let popn_instr = Instruction::new(Opcode::POPN)
            .with_operand(&popn_size);
        self.builder.push_instr(popn_instr);

        let offset_end = self.builder.get_current_offset() as u64;

        let instr = self.builder.get_tag(&tag)
            .ok_or(CompilerError::Unknown)?;

        instr.clear_operands();
        instr.append_operand(&offset_end);
        
        Ok(())
    }

    pub fn compile_call_stmt_expr(&mut self, stmt: &Expression) -> CompilerResult<()> {
        let (fn_name, params) = match stmt {
            Expression::Call(fn_name, params) => (fn_name, params),
            _ => {
                return Err(CompilerError::Unknown);
            }
        };

        let (fn_uid, fn_ret_type, fn_args) = self.resolve_fn(&fn_name)?;
        
        let fn_arg_req_len = fn_args.len();
        if params.len() != fn_arg_req_len {
            return Err(CompilerError::InvalidArgumentCount);
        }

        for (i, (var_name, var_type)) in fn_args.iter() {
            let arg_type = {
                let checker = Checker::new(self);
                checker.check_expr_type(&params[*i])
                    .map_err(|_| CompilerError::TypeMismatch)?
            };
            if arg_type != *var_type {
                return Err(CompilerError::TypeMismatch);
            }
            self.compile_expr(&params[*i])?;
        }
        //println!("Compiling empty call expr: call {}", fn_name);

        let call_instr = Instruction::new(Opcode::CALL)
            .with_operand(&fn_uid);
        self.builder.push_instr(call_instr);

        let size = self.size_of_type(&fn_ret_type)?;
        

        //println!("Compiling statement call expr: fn {} discarding return stack size of {} ", fn_name, size);

        if size > 0 {
            //println!("Adding POPN instruction");
            let popn_instr = Instruction::new(Opcode::POPN)
                .with_operand(&(size as u64));
            self.builder.push_instr(popn_instr);
        }

        let front_context = self.fn_context_stack.get_mut(0)
            .ok_or(CompilerError::Unknown)?;

        //println!("Stack size after call expr: {}", front_context.stack_size);

        Ok(())
    }
    
    pub fn compile_return_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        let ret_expr_opt = match stmt {
            Statement::Return(expr) => expr,
            _ => return Err(CompilerError::Unknown)
        };

        let (fn_index, fn_ctx) = self.get_parent_fn()?;

        let fn_type = fn_ctx
            .return_type
            .as_ref()
            .ok_or(CompilerError::TypeMismatch)?
            .clone();

        let checker = Checker::new(&self);
        if ret_expr_opt.is_some() {
            let ret_expr = ret_expr_opt.as_ref().unwrap();
            let expr_type = checker.check_expr_type(ret_expr)
                .map_err(|_| CompilerError::TypeMismatch)?;
        
            if fn_type != expr_type {
                return Err(CompilerError::TypeMismatch);
            }
            self.compile_expr(ret_expr)?;
        }

        let size = self.size_of_type(&fn_type)?;
        let mut skip_swap = false;

        // Save return value to swap space
        let sv_swap_instr = match &fn_type {
            Type::Int => {
                Instruction::new(Opcode::SVSWPI)
            },
            Type::Bool => {
                Instruction::new(Opcode::SVSWPB)
            },
            Type::Float => {
                Instruction::new(Opcode::SVSWPF)
            },
            Type::Reference(_) => {
                Instruction::new(Opcode::SVSWPN)
                    .with_operand::<u64>(&8)
            },
            Type::Other(_) => {
                Instruction::new(Opcode::SVSWPN)
                    .with_operand::<u64>(&(size as u64))
            },
            Type::Void => {
                skip_swap = true;
                Instruction::new(Opcode::NOOP)
            },
            _ => {
                return Err(CompilerError::Unknown);
            }
        };

        let mut stack_size = {
            let mut ret = 0;
            for i in 0..=fn_index {
                let ctx = self.fn_context_stack.get(i)
                    .ok_or(CompilerError::Unknown)?;
                ret += ctx.stack_size;
            }
            ret
        };

        //println!("Stack size until return (including copy): {}", stack_size);

        ////println!"Stack size of current fn context: {}", stack_size);

        stack_size -= size;
        
        //println!("Stack size until return (excluding copy): {}", stack_size);

        //println!("Stack size to be popped off: {}", stack_size);

        // Pop everything off the stack
        let popn_instr = Instruction::new(Opcode::POPN)
            .with_operand::<u64>(&(stack_size as u64));
        
        //println!("POPN instr: {:?}", popn_instr);

        // Load return value from swap space
        let ld_swap_instr = match &fn_type {
            Type::Int => {
                Instruction::new(Opcode::LDSWPI)
            },
            Type::Bool => {
                Instruction::new(Opcode::LDSWPB)
            },
            Type::Float => {
                Instruction::new(Opcode::LDSWPF)
            },
            Type::Reference(_) => {
                Instruction::new(Opcode::LDSWPN)
                    .with_operand::<u64>(&8)
            },
            Type::Other(_) => {
                Instruction::new(Opcode::LDSWPN)
                    .with_operand::<u64>(&(size as u64))
            },
            Type::Void => {
                skip_swap = true;
                Instruction::new(Opcode::NOOP)
            },
            _ => {
                return Err(CompilerError::Unknown);
            }
        };
        if stack_size > 0 && !skip_swap {
            self.builder.push_instr(sv_swap_instr);
            self.builder.push_instr(popn_instr);
            self.builder.push_instr(ld_swap_instr);
        } else if stack_size > 0 && skip_swap {
            self.builder.push_instr(popn_instr);
        }
        self.builder.push_instr(Instruction::new(Opcode::RET));

        Ok(())
    }

    pub fn compile_var_decl_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        let var_decl_args = match stmt {
            Statement::VariableDecl(args) => args,
            _ => return Err(CompilerError::Unknown)
        };

        let size = self.size_of_type(&var_decl_args.var_type)?;
        let var_type = var_decl_args.var_type.clone();

        //println!"Compiling var decl: {:?}", var_decl_args);

        let checker = Checker::new(&self);
        let expr_type = checker.check_expr_type(&var_decl_args.assignment)
            .map_err(|_| CompilerError::TypeMismatch)?;
        //println!("Var type: {:?}", var_type);
        //println!("Expr type of var decl: {:?}", expr_type);

        if expr_type != var_type {
            return Err(CompilerError::TypeMismatch);
        }

        self.compile_expr(&var_decl_args.assignment)?;

        // Insert variable to context
        {
            let front_context = self.fn_context_stack.get_mut(0)
                .ok_or(CompilerError::Unknown)?;
            front_context.set_var((front_context.stack_size - size) as i64, (var_decl_args.name.clone(), var_type.clone()));
            //println!("Var decl (name: {}) at stack index: {}", var_decl_args.name, front_context.stack_size - size);
        }

        Ok(())
    }

    pub fn compile_var_assign_expr(&mut self, expr: &Expression) -> CompilerResult<()> {
        let (var_expr, assign_expr) = match expr {
            Expression::Assign(var_expr, assign_expr) => (var_expr, assign_expr),
            _ => return Err(CompilerError::Unknown)
        };

        let var_name;
        if let Expression::Variable(name) = var_expr.deref() {
            var_name = name.clone();
        } else {
            return Err(CompilerError::Unknown);
        }

        //println!("Compiling var assign: var {} = {:?}", var_name, assign_expr);

        let var_type = self.type_of_var(&var_name)?;
        let checker = Checker::new(&self);
        let expr_type = checker.check_expr_type(&assign_expr)
            .map_err(|_| CompilerError::TypeMismatch)?;

        if expr_type != var_type {
            return Err(CompilerError::TypeMismatch);
        }

        self.compile_expr(&assign_expr)?;
        
        let var_offset = {
            let front_context = self.fn_context_stack.get(0)
                .ok_or(CompilerError::Unknown)?;
            front_context.offset_of(&var_name)
                .ok_or(CompilerError::UnknownVariable)?
        };
       //println!("Var assign (name={}) offset of var: {}", var_name, var_offset);

        //println!"Var offset for var assign to {}: {}", var_name, var_offset);

        //println!("Compiling var assign to var {} which has offset {}", var_name, var_offset);

        let mov_instr = match var_type {
            Type::Int => {
                let front_context = self.fn_context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size -= 8;
                //println!"Stack size after MOVI: {}", front_context.stack_size);
                Instruction::new(Opcode::SMOVI)
                    .with_operand(&var_offset)
            },
            Type::Float => {
                let front_context = self.fn_context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size -= 4;
                //println!"Stack size after MOVI: {}", front_context.stack_size);
                Instruction::new(Opcode::SMOVF)
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
        
        let front_fn_ctx = self.fn_context_stack.get_mut(0)
            .ok_or(CompilerError::Unknown)?;

        let (fn_uid, fn_ret_type, fn_args) = self.resolve_fn(name)?;

        //println!("Calling fn {}", name);

        //println!"Compiling fn {} ({:?}) ~ {:?}", name, args, fn_ret_type);

        let mut i = 0;
        for arg_expr in args.iter() {
            let req_fn_arg = fn_args.get(&i)
                .ok_or(CompilerError::Unknown)?;
            let arg_type = {
                let checker = Checker::new(self);
                checker.check_expr_type(arg_expr)
                    .map_err(|_| CompilerError::TypeMismatch)?
            };
            if arg_type != req_fn_arg.1 {
                return Err(CompilerError::TypeMismatch);
            }
            self.compile_expr(arg_expr)?;
            i += 1;
        }

        let call_instr = Instruction::new(Opcode::CALL)
            .with_operand(&fn_uid);
        self.builder.push_instr(call_instr);

        let fn_ret_type_size = self.size_of_type(&fn_ret_type)?;

        //println!"fn_ret_type_size: {}", fn_ret_type_size);

        ////println!"Compiling expr call with args {:?} and ret_type {:?}", fn_args, fn_ret_type);

        let front_context = self.fn_context_stack.get_mut(0)
            .ok_or(CompilerError::Unknown)?;
        front_context.stack_size += fn_ret_type_size;

        //println!"front context after call expr: {:?}", front_context);

        ////println!"front context stack size: {}", front_context.stack_size);

        Ok(())
    }

    pub fn compile_member_access_expr(&mut self, expr: &Expression) -> CompilerResult<()> {
        let (lhs, rhs) = match  expr {
            Expression::MemberAccess(lhs, rhs) => (lhs, rhs),
            _ => return Err(CompilerError::Unknown)
        };

        let var_name = match lhs.deref() {
            Expression::Variable(var_name) => var_name,
            _ => return Err(CompilerError::NotImplemented)
        };

        let var_type = self.type_of_var(var_name)?;
        let is_ref;

        let cont_name = match var_type {
            Type::Other(cont_name) => {
                is_ref = false;
                cont_name.clone()
            },
            Type::Reference(t) => {
                is_ref = true;
                let ret;
                if let Type::Other(cont_name) = t.deref() {
                    ret = cont_name.clone();
                } else {
                    return Err(CompilerError::Unknown);
                }
                ret
            },
            _ => return Err(CompilerError::Unknown)
        };

        let cont_def = self.resolve_cont(&cont_name)?;

        match rhs.deref() {
            Expression::Call(name, args) => {
                let (fn_uid, fn_ret_type, fn_args) = cont_def.get_function(name)?;
                let fn_arg_len = fn_args.len();
                let (first_arg_name, first_arg_type) = fn_args.get(&0)
                    .ok_or(CompilerError::UnknownFunction)?;
                if first_arg_name == "this" {
                    if let Type::Reference(inner) = first_arg_type {
                        if let Type::Other(inner_cont_name) = inner.deref() {
                            if inner_cont_name == &cont_name {
                                if is_ref {
                                    let var_offset = {
                                        let front_context = self.fn_context_stack.get(0)
                                            .ok_or(CompilerError::Unknown)?;
                                        front_context.offset_of(var_name)
                                            .ok_or(CompilerError::UnknownVariable)?
                                    };
                                    let sdupa_instr = Instruction::new(Opcode::SDUPA)
                                        .with_operand(&var_offset);
                                    self.builder.push_instr(sdupa_instr);
                                    let front_ctx = self.fn_context_stack.get_mut(0)
                                    .   ok_or(CompilerError::Unknown)?;
                                    front_ctx.stack_size += 8;
                                } else {
                                    let var_offset = {
                                        let front_context = self.fn_context_stack.get(0)
                                            .ok_or(CompilerError::Unknown)?;
                                        front_context.offset_of(var_name)
                                            .ok_or(CompilerError::UnknownVariable)?
                                    };

                                    let sref_instr = Instruction::new(Opcode::SREF)
                                        .with_operand(&var_offset);
                                    self.builder.push_instr(sref_instr);
                                    let front_ctx = self.fn_context_stack.get_mut(0)
                                    .   ok_or(CompilerError::Unknown)?;
                                    front_ctx.stack_size += 8;
                                }
                                if args.len() != fn_arg_len - 1 {
                                    return Err(CompilerError::Unknown);
                                }
                                let mut i = 1;
                                for arg in args.iter() {
                                    let arg_type = Checker::new(self).check_expr_type(arg)
                                        .map_err(|_| CompilerError::TypeMismatch)?;
                                    let (_, req_type) = fn_args.get(&i)
                                        .ok_or(CompilerError::TypeMismatch)?;
                                    if arg_type != *req_type {
                                        return Err(CompilerError::TypeMismatch);
                                    }
                                    self.compile_expr(arg)?;
                                    i += 1;
                                }
                                let call_instr = Instruction::new(Opcode::CALL)
                                    .with_operand(&fn_uid);
                                self.builder.push_instr(call_instr);
                            } else {
                                return Err(CompilerError::UnknownMemberFunction);
                            }
                        } else {
                            return Err(CompilerError::UnknownMemberFunction);      
                        }
                    } else {
                        return Err(CompilerError::UnknownMemberFunction);
                    }
                } else {
                    return Err(CompilerError::UnknownMemberFunction);
                }
            },
            Expression::Variable(name) => {},
            _ => return Err(CompilerError::UnknownVariable)
        };
        
        Ok(())
    }

    pub fn compile_expr(&mut self, expr: &Expression) -> CompilerResult<()> {
        //println!("Compiling expression: {:?}", expr);
        match expr {
            Expression::IntLiteral(int) => {
                let pushi_instr = Instruction::new(Opcode::PUSHI)
                    .with_operand(int);
                self.builder.push_instr(pushi_instr);
                let front_context = self.fn_context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size += 8;
            },
            Expression::BoolLiteral(b) => {
                let pushb_instr = Instruction::new(Opcode::PUSHB)
                    .with_operand(b);
                self.builder.push_instr(pushb_instr);
                let front_context = self.fn_context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size += 1;
            },
            Expression::StringLiteral(string) => {
                // Trim trailing ""
                let string = String::from(&string[1..string.len()-1]);
                let (size, addr) = {
                    self.data.add_string(&string)
                };
                //println!("Got data string literal \"{}\" with size {}!", string, size);
                let size_instr = Instruction::new(Opcode::PUSHA)
                    .with_operand(&size);
                let pusha_instr = Instruction::new(Opcode::PUSHA)
                    .with_operand(&addr);
                self.builder.push_instr(size_instr);
                self.builder.push_instr(pusha_instr);
                let front_context = self.fn_context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size += 16;
            },
            Expression::FloatLiteral(float) => {
                let pushi_instr = Instruction::new(Opcode::PUSHF)
                    .with_operand(float);
                self.builder.push_instr(pushi_instr);
                let front_context = self.fn_context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size += 4;
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
                //println!("Compiling var expr. Name = {}, offset = {}", var_name, var_offset);
                let var_type = {
                    self.type_of_var(var_name)?
                };
                let dup_instr = match var_type {
                    Type::Int => {
                        Instruction::new(Opcode::SDUPI)
                            .with_operand(&var_offset)
                    },
                    Type::Float => {
                        //println!("Compiling SDUPF");
                        Instruction::new(Opcode::SDUPF)
                            .with_operand(&var_offset)
                    },
                    Type::String => {
                        Instruction::new(Opcode::SDUPN)
                            .with_operand(&var_offset)
                            .with_operand::<u64>(&16)
                    },
                    _ => return Err(CompilerError::NotImplemented)  
                };
                //println!("dup instruction for var expr: {:?}", dup_instr);
                self.builder.push_instr(dup_instr);
                let var_size = self.size_of_type(&var_type)?;
                //println!("Compiling var expr. size: {}", var_size);
                let front_context = self.fn_context_stack.get_mut(0)
                    .ok_or(CompilerError::Unknown)?;
                front_context.stack_size += var_size;
            },
            Expression::MemberAccess(_, _) => {
                self.compile_member_access_expr(expr)?;
            },
            Expression::Addition(lhs, rhs) => {
                self.compile_expr(lhs)?;
                self.compile_expr(rhs)?;
                let var_type = {
                    let checker = Checker::new(self);
                    checker.check_expr_type(lhs)
                        .map_err(|_| CompilerError::TypeMismatch)?
                };
                match var_type {
                    Type::Int => {
                        let addi_instr = Instruction::new(Opcode::ADDI);
                        self.builder.push_instr(addi_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 16;
                        front_context.stack_size += 8;
                    },
                    Type::Float => {
                        let addf_instr = Instruction::new(Opcode::ADDF);
                        self.builder.push_instr(addf_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 8;
                        front_context.stack_size += 4;
                    },
                    _ => unimplemented!("Addition for non-numbers")
                };
            },
            Expression::Subtraction(lhs, rhs) => {
                self.compile_expr(lhs)?;
                self.compile_expr(rhs)?;
                let var_type = {
                    let checker = Checker::new(self);
                    checker.check_expr_type(lhs)
                        .map_err(|_| CompilerError::TypeMismatch)?
                };
                match var_type {
                    Type::Int => {
                        let subi_instr = Instruction::new(Opcode::SUBI);
                        self.builder.push_instr(subi_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 16;
                        front_context.stack_size += 8;
                    },
                    Type::Float => {
                        let subf_instr = Instruction::new(Opcode::SUBF);
                        self.builder.push_instr(subf_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 8;
                        front_context.stack_size += 4;
                    },
                    _ => unimplemented!("Subtraction for non-numbers")
                };
            },
            Expression::Multiplication(lhs, rhs) => {
                self.compile_expr(lhs)?;
                self.compile_expr(rhs)?;
                let var_type = {
                    let checker = Checker::new(self);
                    checker.check_expr_type(lhs)
                        .map_err(|_| CompilerError::TypeMismatch)?
                };
                match var_type {
                    Type::Int => {
                        let muli_instr = Instruction::new(Opcode::MULI);
                        self.builder.push_instr(muli_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 16;
                        front_context.stack_size += 8;
                    },
                    Type::Float => {
                        let mulf_instr = Instruction::new(Opcode::MULF);
                        self.builder.push_instr(mulf_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 8;
                        front_context.stack_size += 4;
                    },
                    _ => unimplemented!("Multiplication for non-numbers")
                };
            },
            Expression::Division(lhs, rhs) => {
                self.compile_expr(lhs)?;
                self.compile_expr(rhs)?;
                let var_type = {
                    let checker = Checker::new(self);
                    checker.check_expr_type(lhs)
                        .map_err(|_| CompilerError::TypeMismatch)?
                };
                match var_type {
                    Type::Int => {
                        let divi_instr = Instruction::new(Opcode::DIVI);
                        self.builder.push_instr(divi_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 16;
                        front_context.stack_size += 8;
                    },
                    Type::Float => {
                        let divf_instr = Instruction::new(Opcode::DIVF);
                        self.builder.push_instr(divf_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 8;
                        front_context.stack_size += 4;
                    },
                    _ => unimplemented!("Division for non-numbers")
                };
            },
            Expression::Equals(lhs, rhs) => {
                let checker = Checker::new(self);
                let expr_type = checker.check_expr_type(lhs)
                    .map_err(|_| CompilerError::TypeMismatch)?;
                self.compile_expr(lhs)?;
                self.compile_expr(rhs)?;
                match expr_type {
                    Type::Int => {
                        let eqi_instr = Instruction::new(Opcode::EQI);
                        self.builder.push_instr(eqi_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 16;
                        front_context.stack_size += 1;
                    },
                    Type::Float => {
                        let eqf_instr = Instruction::new(Opcode::EQF);
                        self.builder.push_instr(eqf_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 8;
                        front_context.stack_size += 1;
                    },
                    _ => return Err(CompilerError::NotImplemented)
                };
            },
            Expression::NotEquals(lhs, rhs) => {
                let checker = Checker::new(self);
                let expr_type = checker.check_expr_type(lhs)
                    .map_err(|_| CompilerError::TypeMismatch)?;
                self.compile_expr(lhs)?;
                self.compile_expr(rhs)?;
                match expr_type {
                    Type::Int => {
                        let eqi_instr = Instruction::new(Opcode::EQI);
                        self.builder.push_instr(eqi_instr);
                        self.builder.push_instr(Instruction::new(Opcode::NOT));
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 16;
                        front_context.stack_size += 1;
                    },
                    Type::Float => {
                        let eqf_instr = Instruction::new(Opcode::EQF);
                        self.builder.push_instr(eqf_instr);
                        self.builder.push_instr(Instruction::new(Opcode::NOT));
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 8;
                        front_context.stack_size += 1;
                    },
                    _ => return Err(CompilerError::NotImplemented)
                };
            },
            Expression::Not(op) => {
                self.compile_expr(op)?;
                let not_instr = Instruction::new(Opcode::NOT);
                self.builder.push_instr(not_instr);
            },
            Expression::GreaterThan(lhs, rhs) => {
                let checker = Checker::new(self);
                let expr_type = checker.check_expr_type(lhs)
                    .map_err(|_| CompilerError::TypeMismatch)?;
                self.compile_expr(lhs)?;
                self.compile_expr(rhs)?;
                match expr_type {
                    Type::Int => {
                        let gti_instr = Instruction::new(Opcode::GTI);
                        self.builder.push_instr(gti_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 16;
                        front_context.stack_size += 1;
                    },
                    Type::Float => {
                        let gtf_instr = Instruction::new(Opcode::GTF);
                        self.builder.push_instr(gtf_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 8;
                        front_context.stack_size += 1;
                    },
                    _ => return Err(CompilerError::NotImplemented)
                };
            },
            Expression::GreaterThanEquals(lhs, rhs) => {
                let checker = Checker::new(self);
                let expr_type = checker.check_expr_type(lhs)
                    .map_err(|_| CompilerError::TypeMismatch)?;
                self.compile_expr(lhs)?;
                self.compile_expr(rhs)?;
                match expr_type {
                    Type::Int => {
                        let gteqi_instr = Instruction::new(Opcode::GTEQI);
                        self.builder.push_instr(gteqi_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 16;
                        front_context.stack_size += 1;
                    },
                    Type::Float => {
                        let gteqf_instr = Instruction::new(Opcode::GTEQF);
                        self.builder.push_instr(gteqf_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 8;
                        front_context.stack_size += 1;
                    },
                    _ => return Err(CompilerError::NotImplemented)
                };
            },
            Expression::LessThan(lhs, rhs) => {
                let checker = Checker::new(self);
                let expr_type = checker.check_expr_type(lhs)
                    .map_err(|_| CompilerError::TypeMismatch)?;
                self.compile_expr(lhs)?;
                self.compile_expr(rhs)?;
                match expr_type {
                    Type::Int => {
                        let lti_instr = Instruction::new(Opcode::LTI);
                        self.builder.push_instr(lti_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 16;
                        front_context.stack_size += 1;
                    },
                    Type::Float => {
                        let ltf_instr = Instruction::new(Opcode::LTF);
                        self.builder.push_instr(ltf_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 8;
                        front_context.stack_size += 1;
                    },
                    _ => return Err(CompilerError::NotImplemented)
                };
            },
            Expression::LessThanEquals(lhs, rhs) => {
                let checker = Checker::new(self);
                let expr_type = checker.check_expr_type(lhs)
                    .map_err(|_| CompilerError::TypeMismatch)?;
                self.compile_expr(lhs)?;
                self.compile_expr(rhs)?;
                match expr_type {
                    Type::Int => {
                        let lteqi_instr = Instruction::new(Opcode::LTEQI);
                        self.builder.push_instr(lteqi_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 16;
                        front_context.stack_size += 1;
                    },
                    Type::Float => {
                        let lteqf_instr = Instruction::new(Opcode::LTEQF);
                        self.builder.push_instr(lteqf_instr);
                        let front_context = self.fn_context_stack.get_mut(0)
                            .ok_or(CompilerError::Unknown)?;
                        front_context.stack_size -= 8;
                        front_context.stack_size += 1;
                    },
                    _ => return Err(CompilerError::NotImplemented)
                };
            },
            _ => return Err(CompilerError::NotImplemented)
        };
        Ok(())
    }
}
