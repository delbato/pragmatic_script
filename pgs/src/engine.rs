use crate::{
    vm::{
        core::Core
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
    }
};

pub struct Engine {
    pub core: Core
}

pub type EngineResult<T> = Result<T, EngineError>;

pub enum EngineError {
    Unknown,
    CoreError
}


impl Engine {
    pub fn new(stack_size: usize) -> Engine {
        Engine {
            core: Core::new(stack_size)
        }
    }

    pub fn run_code(&mut self, code: &str) -> EngineResult<()> {
        Err(EngineError::Unknown)
    }

    pub fn run_file(&mut self, path: &Path) -> EngineResult<()> {
        Err(EngineError::Unknown)
    }

    pub fn run_stream(&mut self, readable: Box<dyn Read>) -> EngineResult<()> {
        Err(EngineError::Unknown)
    }
}