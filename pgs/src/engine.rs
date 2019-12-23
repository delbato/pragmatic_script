use crate::{
    vm::{
        core::{
            Core,
            CoreError
        }
    },
    parser::{
        parser::{
            ParseError,
            Parser
        },
        ast::{
            Declaration,
            Statement
        }
    },
    codegen::{
        compiler::{
            Compiler,
            CompilerError
        }
    },
    api::{
        module::Module
    }
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
    },
    error::Error,
    fmt::{
        Display,
        Debug,
        Formatter,
        Result as FmtResult
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

pub type EngineResult<T> = Result<T, Box<EngineError>>;

#[derive(Debug)]
pub enum EngineError {
    Unknown,
    CoreError(CoreError),
    ParseError(ParseError),
    CompileError(CompilerError),
}

impl Display for EngineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}

impl Error for EngineError {
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
        let parser = Parser::new(String::from(code));
        let decl_list = parser.parse_root_decl_list()
            .map_err(|p| Box::new(EngineError::ParseError(p)))?;
        self.compiler.compile_root_decl_list(decl_list)
            .map_err(|c| Box::new(EngineError::CompileError(c)))?;
        let program = self.compiler.get_program()
            .map_err(|c| Box::new(EngineError::CompileError(c)))?;
        self.core.load_program(program);
        Ok(())
    }

    pub fn run_file(&mut self, path: &Path) -> EngineResult<()> {
        let mut file = File::open(path)
            .map_err(|_| Box::new(EngineError::Unknown))?;

        let mut file_content = String::new();
        file.read_to_string(&mut file_content)
            .map_err(|_| Box::new(EngineError::Unknown))?;

       self.run_code(&file_content)
    }

    pub fn run_stream(&mut self, readable: Box<dyn Read>) -> EngineResult<()> {
        Err(Box::new(EngineError::Unknown))
    }

    pub fn push_stack<T: Serialize>(&mut self, item: T) -> EngineResult<()> {
        self.core.push_stack(item)
            .map_err(|c| Box::new(EngineError::CoreError(c)))
    }

    pub fn pop_stack<T: DeserializeOwned>(&mut self) -> EngineResult<T> {
        self.core.pop_stack()
            .map_err(|c| Box::new(EngineError::CoreError(c)))
    }

    pub fn get_stack_size(&self) -> usize {
        self.core.get_stack_size()
    }

    pub fn run_fn(&mut self, name: &String) -> EngineResult<()> {
        let fn_uid = self.compiler.get_function_uid(name);
        self.core.run_fn(fn_uid)
            .map_err(|c| Box::new(EngineError::CoreError(c)))
    }

    pub fn register_module(&mut self, mut module: Module) -> EngineResult<()> {
        self.compiler.register_foreign_module(&mut module, String::new())
            .map_err(|c| Box::new(EngineError::CompileError(c)))?;
        self.core.register_foreign_module(module)
            .map_err(|c| Box::new(EngineError::CoreError(c)))
    }
}
