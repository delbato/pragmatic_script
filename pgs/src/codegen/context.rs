use std::{
    collections::{
        BTreeMap,
        HashMap
    }
};

use crate::{
    parser::{
        ast::{
            Type,
            FunctionDeclArgs
        }
    },
    codegen::{
        container::Container
    }
};

#[derive(Clone)]
pub struct FunctionContext {
    pub variable_indices: HashMap<String, i64>,
    pub variable_types: HashMap<String, Type>,
    pub functions: HashMap<String, FunctionDeclArgs>,
    pub return_type: Option<Type>,
    pub stack_size: usize,
    pub weak: bool
}

impl FunctionContext {
    pub fn new() -> FunctionContext {
        FunctionContext {
            variable_indices: HashMap::new(),
            variable_types: HashMap::new(),
            functions: HashMap::new(),
            return_type: None,
            stack_size: 0,
            weak: false
        }
    }

    pub fn new_weak(other: &FunctionContext) -> FunctionContext {
        let other_size = other.stack_size as i64;
        
        let mut context = FunctionContext {
            variable_indices: HashMap::new(),
            variable_types: HashMap::new(),
            functions: HashMap::new(),
            return_type: None,
            stack_size: 0,
            weak: true
        };

        for (var_name, var_index) in other.variable_indices.iter() {
            context.variable_indices.insert(var_name.clone(), var_index - other_size);    
        }
        context.variable_types = other.variable_types.clone();
        
        context
    }

    pub fn type_of(&self, var_name: &String) -> Option<Type> {
        self.variable_types.get(var_name).cloned()
    }

    pub fn index_of(&self, var_name: &String) -> Option<i64> {
        self.variable_indices.get(var_name).cloned()
    }

    pub fn offset_of(&self, var_name: &String) -> Option<i64> {
        let var_index_opt = self.variable_indices.get(var_name);
        if var_index_opt.is_none() {
            return None;
        }
        let var_index = var_index_opt.unwrap();
        Some(
            (self.stack_size as i64 - var_index) * -1
        )
    }

    pub fn push_var(&mut self, (var_name, var_type): (String, Type)) {
        let index = self.stack_size as i64;
        self.variable_indices.insert(var_name.clone(), index);
        self.variable_types.insert(var_name, var_type);
    }

    pub fn set_var(&mut self, index: i64, (var_name, var_type): (String, Type)) {
        self.variable_indices.insert(var_name.clone(), index);
        self.variable_types.insert(var_name, var_type);
    }
}

#[derive(Clone)]
pub struct ModuleContext {
    pub name: String,
    pub modules: HashMap<String, ModuleContext>,
    pub functions: HashMap<String, (u64, Type, BTreeMap<usize, (String, Type)>)>,
    pub containers: HashMap<String, Container>,
    pub imports: HashMap<String, String>
}

impl ModuleContext {
    pub fn new(name: String) -> ModuleContext {
        ModuleContext {
            name: name,
            modules: HashMap::new(),
            containers: HashMap::new(),
            functions: HashMap::new(),
            imports: HashMap::new()
        }
    }
}

#[derive(Clone)]
pub struct LoopContext {
    pub instr_offset: usize,
    pub instr_end: usize
}

#[derive(Clone)]
pub struct IfContext {
    pub instr_offset: usize,
    pub instr_end: usize
}