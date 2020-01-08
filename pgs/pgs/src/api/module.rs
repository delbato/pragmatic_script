use crate::{
    api::{
        function::{
            Function,
            FunctionError,
            FunctionResult
        }
    }
};

use std::{
    collections::{
        HashMap
    }  
};

pub struct Module {
    pub name: String,
    pub modules: HashMap<String, Module>,
    pub functions: Vec<Function>
}

impl Module {
    pub fn new(name: String) ->  Module {
        Module {
            name: name,
            modules: HashMap::new(),
            functions: Vec::new()
        }
    }

    pub fn with_module(mut self, module: Module) -> Module {
        self.modules.insert(module.name.clone(), module);
        self
    }

    pub fn with_function(mut self, function: Function) -> Module {
        self.functions.push(function);
        self
    }
}