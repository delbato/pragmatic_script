use std::{
    collections::HashMap
};

use crate::{
    parser::{
        ast::Type
    }
};

#[derive(Clone)]
pub struct Context {
    pub variable_indices: HashMap<String, i64>,
    pub variable_types: HashMap<String, Type>,
    pub functions: HashMap<String, Type>,
    pub return_type: Option<Type>,
    pub stack_size: usize,
    pub weak: bool
}

impl Context {
    pub fn new() -> Context {
        Context {
            variable_indices: HashMap::new(),
            variable_types: HashMap::new(),
            functions: HashMap::new(),
            return_type: None,
            stack_size: 0,
            weak: false
        }
    }

    pub fn new_weak(other: &Context) -> Context {
        let other_size = other.stack_size as i64;
        
        let mut context = Context {
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