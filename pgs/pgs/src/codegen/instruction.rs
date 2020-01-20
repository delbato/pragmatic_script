use crate::{
    vm::{
        is::Opcode
    }
};



use serde::{
    Serialize,
    de::DeserializeOwned
};
use bincode::{
    deserialize,
    serialize
};

#[derive(Clone, Debug)]
pub struct Instruction {
    pub opcode: Opcode,
    pub operands: Vec<u8>,
}

impl Instruction {
    pub fn new(opcode: Opcode) -> Instruction {
        Instruction {
            opcode: opcode,
            operands: Vec::new()
        }
    }

    pub fn new_inc_stack(inc: usize) -> Vec<Instruction> {
        let mut ret = Vec::new();
        let lda_instr = Instruction::new(Opcode::LDA)
            .with_operand(15u8)
            .with_operand(inc as u64);
        let add_instr = Instruction::new(Opcode::UADDI)
            .with_operand(16u8)
            .with_operand(15u8)
            .with_operand(16u8);
        ret.push(lda_instr);
        ret.push(add_instr);
        ret
    }

    pub fn new_dec_stack(dec: usize) -> Vec<Instruction> {
        let mut ret = Vec::new();
        let lda_instr = Instruction::new(Opcode::LDA)
            .with_operand(15u8)
            .with_operand(dec as u64);
        let add_instr = Instruction::new(Opcode::USUBI)
            .with_operand(16u8)
            .with_operand(15u8)
            .with_operand(16u8);
        ret.push(lda_instr);
        ret.push(add_instr);
        ret
    }

    pub fn with_operand<T: Serialize>(mut self, operand: T) -> Instruction {
        let mut data = serialize(&operand).expect("ERROR Serializing operand!");
        self.operands.append(&mut data);
        self
    }

    pub fn append_operand<T: Serialize>(&mut self, operand: T) {
        let mut data = serialize(&operand).expect("ERROR Serializing operand!");
        self.operands.append(&mut data);
    }

    pub fn clear_operands(&mut self) {
        self.operands.clear();
    }

    pub fn get_code(mut self) -> Vec<u8> {
        let mut code = Vec::new();

        // Get binary for opcode
        let opcode: u8 = self.opcode.into();
        code.push(opcode);

        // Append the operands
        code.append(&mut self.operands);
        
        code
    }

    pub fn get_size(&self) -> usize {
        self.operands.len() + 1
    }

    pub fn get_operand<T: DeserializeOwned>(&self) -> T {
        let t = deserialize(&self.operands).expect("ERROR Deserializing operand!");
        t
    }
}