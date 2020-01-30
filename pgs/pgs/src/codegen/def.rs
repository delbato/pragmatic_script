use crate::{
    parser::{
        ast::{
            Type,
            FunctionDeclArgs,
            ContainerDeclArgs
        }
    },
    codegen::{
        compiler::{
            CompilerResult,
            CompilerError,
            Compiler
        }
    }
};

use std::{
    collections::{
        HashMap
    },
    convert::{
        From
    }
};

/// A function definition
#[derive(Clone, PartialEq, Debug)]
pub struct FunctionDef {
    pub name: String,
    pub uid: u64,
    pub ret_type: Type,
    pub arguments: Vec<(String, Type)>
}

impl FunctionDef {
    /// Creates a new function definition with no return type or arguments
    pub fn new(name: String) -> FunctionDef {
        FunctionDef {
            name: name,
            uid: 0,
            ret_type: Type::Void,
            arguments: Vec::new()
        }
    }

    /// With a specific return type
    pub fn with_ret_type(mut self, ret_type: Type) -> FunctionDef {
        self.ret_type = ret_type;
        self
    }

    /// With decl args arguments
    pub fn with_arguments(mut self, arguments: &[(String, Type)]) -> FunctionDef {
        for argument in arguments.iter() {
            self.arguments.push(argument.clone());
        }
        self
    }

    /// With a uid
    pub fn with_uid(mut self, uid: u64) -> FunctionDef {
        self.uid = uid;
        self
    }
}

impl From<&FunctionDeclArgs> for FunctionDef {
    fn from(item: &FunctionDeclArgs) -> FunctionDef {
        FunctionDef::new(item.name.clone())
            .with_ret_type(item.returns.clone())
            .with_arguments(&item.arguments)
    }
}

/// A container definition
#[derive(Clone)]
pub struct ContainerDef {
    pub name: String,
    pub member_variables: HashMap<String, Type>,
    pub member_functions: HashMap<String, FunctionDef>
}

impl ContainerDef {
    /// Creates a new container definition
    pub fn new(name: String) -> ContainerDef {
        ContainerDef {
            name: name,
            member_functions: HashMap::new(),
            member_variables: HashMap::new()
        }
    }

    /// Adds a member variable
    pub fn add_member_variable(&mut self, var: (String, Type)) -> CompilerResult<()> {
        if self.member_variables.contains_key(&var.0) {
            return Err(CompilerError::DuplicateMember(var.0));
        }
        self.member_variables.insert(var.0, var.1);
        Ok(())
    }

    /// Adds a member function
    pub fn add_member_function(&mut self, fn_def: FunctionDef) -> CompilerResult<()> {
        if self.member_functions.contains_key(&fn_def.name) {
            return Err(CompilerError::DuplicateFunction(fn_def.name));
        }
        self.member_functions.insert(fn_def.name.clone(), fn_def);
        Ok(())
    }

    /// Returns the byte size of this container
    pub fn get_size(&self, compiler: &Compiler) -> CompilerResult<usize> {
        let mut size = 0;
        for (_, var_type) in self.member_variables.iter() {
            size += compiler.get_size_of_type(var_type)?;
        }
        Ok(size)
    }
}

impl From<&ContainerDeclArgs> for ContainerDef {
    fn from(item: &ContainerDeclArgs) -> ContainerDef {
        let mut def = ContainerDef::new(item.name.clone());
        for member in item.members.iter() {
            def.add_member_variable(member.clone()).unwrap();
        }
        def
    }
}