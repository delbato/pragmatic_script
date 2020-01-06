use crate::{
    parser::{
        ast::{
            Type
        }
    },
    vm::{
        core::{
            Core
        }
    }
};

use std::{
    marker::{
        Sized
    }
};

pub type FunctionResult<T> = Result<T, FunctionError>;

#[derive(Clone)]
pub enum FunctionError {
    Unknown,
}

pub struct Function {
    pub name: String,
    pub uid: Option<u64>,
    pub arguments: Vec<Type>,
    pub return_type: Option<Type>,
    pub raw_callback: Option<Box<dyn FnMut(&mut Core) -> FunctionResult<()>>>
}

impl Function {
    pub fn new(name: String) -> Function {
        Function {
            name: name, 
            uid: None,
            arguments: Vec::new(),
            return_type: None,
            raw_callback: None
        }
    }

    pub fn with_argument(mut self, arg_type: Type) -> Function {
        self.arguments.push(arg_type);
        self
    }

    pub fn with_return_type(mut self, ret_type: Type) -> Function {
        self.return_type = Some(ret_type);
        self
    }

    pub fn with_callback(mut self, raw_callback: Box<dyn FnMut(&mut Core) -> FunctionResult<()>>) -> Function {
        self.raw_callback = Some(raw_callback);
        self
    }
}

impl PartialEq for Function {
    fn eq(&self, rhs: &Function) -> bool {
        self.name == rhs.name
    }
}

impl std::fmt::Debug for Function {
    fn fmt(&self, form: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(form, "Function: {{ name = {}, args = \n", self.name)?;
        
        for i in 0..self.arguments.len() {
            let arg_type = &self.arguments[i];
            write!(form, "\targ#{}: {:?}\n", i, arg_type)?;
        }

        write!(form, "\n")
    }
}