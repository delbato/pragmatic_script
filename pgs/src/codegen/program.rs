use std::{
    collections::HashMap
};

pub struct Program {
    pub code: Vec<u8>,
    pub functions: HashMap<u64, usize>
}

impl Program {
    pub fn new() -> Program {
        Program {
            code: Vec::new(),
            functions: HashMap::new()
        }
    }

    pub fn with_code(mut self, code: Vec<u8>) -> Program {
        self.code = code;
        self
    }

    pub fn with_functions(mut self, functions: HashMap<u64, usize>) -> Program {
        self.functions = functions;
        self
    }

    pub fn get_size(&self) -> usize {
        self.code.len()
    }
}