use crate::{
    codegen::{
        def::{
            ContainerDef,
            FunctionDef
        },
        instruction::{
            Register
        },
        compiler::{
            CompilerResult,
            CompilerError
        }
    },
    parser::{
        ast::{
            Type
        }
    }
};

use std::{
    collections::{
        HashMap
    }
};

pub struct ModuleContext {
    pub name: String,
    pub modules: HashMap<String, ModuleContext>,
    pub functions: HashMap<String, FunctionDef>,
    pub containers: HashMap<String, ContainerDef>,
    pub imports: HashMap<String, String>
}

impl ModuleContext {
    /// Creates a new module context
    pub fn new(name: String) -> ModuleContext {
        ModuleContext {
            name: name,
            modules: HashMap::new(),
            functions: HashMap::new(),
            containers: HashMap::new(),
            imports: HashMap::new()
        }
    }

    /// Adds a function definition to a module context.
    /// Throws a DuplicateFunctionError if a function with the 
    /// same name already exists.
    pub fn add_function(&mut self, def: FunctionDef) -> CompilerResult<()> {
        if self.functions.contains_key(&def.name) {
            return Err(CompilerError::DuplicateFunction(def.name));
        }
        self.functions.insert(def.name.clone(), def);
        Ok(())
    }

    /// Adds a module context to a module context.
    /// Throws a DuplicateModuleError if a module with the
    /// same name already exists.
    pub fn add_module(&mut self, mod_ctx: ModuleContext) -> CompilerResult<()> {
        if self.modules.contains_key(&mod_ctx.name) {
            return Err(CompilerError::DuplicateModule(mod_ctx.name));
        }
        self.modules.insert(mod_ctx.name.clone(), mod_ctx);
        Ok(())
    }

    /// Adds a container definition to a module context.
    /// Throws a DuplicateContainerError if a container with the
    /// same name already exists.
    pub fn add_container(&mut self, cont_def: ContainerDef) -> CompilerResult<()> {
        if self.containers.contains_key(&cont_def.name) {
            return Err(CompilerError::DuplicateContainer(cont_def.name));
        }
        self.containers.insert(cont_def.name.clone(), cont_def);
        Ok(())
    }

    /// Adds an import declaration to a module context
    /// Throws a DuplicateImportError if an import with the same
    /// "import_as" name already exists.
    pub fn add_import(&mut self, import_as: String, import_path: String) -> CompilerResult<()> {
        if self.imports.contains_key(&import_as) {
            return Err(CompilerError::DuplicateImport(import_as));
        }
        self.imports.insert(import_as, import_path);
        Ok(())
    }

    /// Gets a mutable reference to a container definition, given the name
    pub fn get_container(&mut self, name: &String) -> CompilerResult<&mut ContainerDef> {
        self.containers.get_mut(name)
            .ok_or(CompilerError::UnknownContainer(name.clone()))
    }

    /// Gets a reference to the function definition, given the name
    pub fn get_function(&self, name: &String) -> CompilerResult<&FunctionDef> {
        self.functions.get(name)
            .ok_or(CompilerError::UnknownFunction(name.clone()))
    }
}

pub enum VariableLocation {
    Stack(i64),
    Register(Register)
}

pub struct FunctionContext {
    pub def: Option<FunctionDef>,
    pub weak: bool,
    pub stack_size: usize,
    variable_types: HashMap<String, Type>,
    variable_locations: HashMap<String, Vec<VariableLocation>>
}

impl FunctionContext {
    pub fn new(def: FunctionDef) -> FunctionContext {
        FunctionContext {
            def: Some(def),
            weak: false,
            stack_size: 0,
            variable_locations: HashMap::new(),
            variable_types: HashMap::new()
        }
    }

    pub fn set_stack_var(name: String, var_type: Type, stack_pos: usize) -> CompilerResult<()> {
        Ok(())
    }
}