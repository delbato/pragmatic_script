use crate::{
    vm::{
        core::Core
    },
    parser::{
        parser::Parser,
        ast::{
            Declaration,
            Statement
        }
    },
    codegen::{
        compiler::Compiler
    },
};

use std::{
    io::{
        Read
    },
    fs::{
        File
    },
    path::{
        Path
    }
};

use serde::{
    de::DeserializeOwned,
    Serialize
};

pub struct Engine {
    core: Core,
    compiler: Compiler
}

pub type EngineResult<T> = Result<T, EngineError>;

#[derive(Debug)]
pub enum EngineError {
    Unknown,
    CoreError,
    ParseError,
    CompileError,
}


impl Engine {
    pub fn new(stack_size: usize) -> Engine {
        let mut compiler = Compiler::new();
        compiler.push_default_module_context();
        Engine {
            core: Core::new(stack_size),
            compiler: compiler
        }
    }

    pub fn run_code(&mut self, code: &str) -> EngineResult<()> {
        self.load_code(code)?;
        self.run_fn(&String::from("root::main"))
    }

    pub fn load_code(&mut self, code: &str) -> EngineResult<()> {
        let mut parser = Parser::new(String::from(code));
        let decl_list = parser.parse_root_decl_list()
            .map_err(|_| EngineError::ParseError)?;
        self.compiler.compile_decl_list(decl_list)
            .map_err(|_| EngineError::CompileError)?;
        let program = self.compiler.get_program()
            .map_err(|_| EngineError::CompileError)?;
        self.core.load_program(program);
        Ok(())
    }

    pub fn run_file(&mut self, path: &Path) -> EngineResult<()> {
        let mut file = File::open(path)
            .map_err(|_| EngineError::Unknown)?;

        let mut file_content = String::new();
        file.read_to_string(&mut file_content)
            .map_err(|_| EngineError::Unknown)?;

       self.run_code(&file_content)
    }

    pub fn run_stream(&mut self, readable: Box<dyn Read>) -> EngineResult<()> {
        Err(EngineError::Unknown)
    }

    pub fn push_stack<T: Serialize>(&mut self, item: T) -> EngineResult<()> {
        self.core.push_stack(item)
            .map_err(|_| EngineError::CoreError)
    }

    pub fn pop_stack<T: DeserializeOwned>(&mut self) -> EngineResult<T> {
        self.core.pop_stack()
            .map_err(|_| EngineError::CoreError)
    }

    pub fn run_fn(&mut self, name: &String) -> EngineResult<()> {
        let fn_uid = self.compiler.get_function_uid(name);
        self.core.run_fn(fn_uid)
            .map_err(|_| EngineError::CoreError)
    }
}
