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
        instruction::{
            Instruction,
            Register
        }
    },
    parser::{
        ast::{
            Declaration,
            Statement
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
        VecDeque
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
    UnknownContainer(String)
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
    uid_generator: UIDGenerator,
    builder: Builder,
    current_cont: Option<String>
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
            uid_generator: UIDGenerator::new(),
            builder: Builder::new(),
            current_cont: None
        }
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

        let fn_ctx = FunctionContext::new(fn_def);

        let mut full_fn_name = self.get_module_path();
        full_fn_name += &fn_decl_args.name;


        self.builder.push_label(full_fn_name);
        if let Some(stmt_list) = &fn_decl_args.code_block {
            self.compile_stmt_list(stmt_list)?;
        }


        Ok(())
    }

    pub fn compile_mod_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        Ok(())
    }

    pub fn compile_impl_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        Ok(())
    }

    pub fn compile_stmt_list(&mut self, stmt_list: &[Statement]) -> CompilerResult<()> {
        Ok(())
    }

    pub fn compile_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        Ok(())
    }

    // #endregion
}