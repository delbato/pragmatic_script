use super::{
    instruction::{
        Instruction
    }
};

use std::{
    collections::{
        HashMap
    }
};

use serde::{
    Serialize
};
use bincode::serialize;

#[derive(Clone)]
pub struct Builder {
    data: Vec<u8>,
    instructions: Vec<Instruction>,
    labels: HashMap<String, usize>
}

impl Builder {
    pub fn new() -> Builder {
        Builder {
            data: Vec::new(),
            instructions: Vec::new(),
            labels: HashMap::new()
        }
    }

    pub fn push_label(&mut self, label: String) {
        self.labels.insert(label, self.instructions.len());
    }

    pub fn push_instr(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    pub fn push_data<T: Serialize>(&mut self, data: T) {
        let mut data = serialize(&data).expect("Could not serialize builder data!");
        self.data.append(&mut data);
    }

    pub fn build(mut self) -> Vec<u8> {
        let mut code = Vec::new();

        code.append(&mut self.data);

        for instruction in self.instructions {
            let mut instr_code = instruction.get_code();
            code.append(&mut instr_code);
        }

        code
    }
}