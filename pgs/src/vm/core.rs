use super::{
    is::{
        Opcode
    }
};
use crate::{
    codegen::{
        program::Program
    }
};

use std::{
    collections::{
        VecDeque,
        HashMap
    },
    mem::{
        size_of,
        size_of_val
    },
    cell::{
        RefCell
    }
};

use serde::{
    de::{
        DeserializeOwned
    },
    Serialize
};

use bincode::{
    serialize,
    deserialize
};

pub type CoreResult<T> = Result<T, CoreError>;

pub const STACK_GROW_INCREMENT: usize = 1024;
pub const STACK_GROW_THRESHOLD: usize = 64;
pub const SWAP_SPACE_SIZE: usize = 64;

pub struct Core {
    stack: Vec<u8>,
    swap: Vec<u8>,
    program: Option<Program>,
    stack_frames: VecDeque<usize>,
    call_stack: VecDeque<usize>,
    ip: usize,
    sp: usize
}

#[derive(Debug)]
pub enum CoreError {
    Unknown,
    NoProgram,
    UnimplementedOpcode(Opcode),
    OperatorDeserialize,
    OperatorSerialize,
    EmptyCallStack,
    UnknownFunctionUid,
    InvalidStackPointer
}

impl Core {
    pub fn new(stack_size: usize) -> Core {
        let mut stack = Vec::new();
        stack.resize(stack_size, 0);
        let mut swap = Vec::new();
        swap.resize(SWAP_SPACE_SIZE, 0);
        Core {
            program: None,
            swap: swap,
            stack: stack,
            stack_frames: VecDeque::new(),
            call_stack: VecDeque::new(),
            ip: 0,
            sp: 0
        }
    }

    pub fn load_program(&mut self, program: Program) {
        self.program = Some(program);
    }

    pub fn program_len(&self) -> CoreResult<usize> {
        let program = self.program.as_ref()
            .ok_or(CoreError::Unknown)?;
        Ok(
            program.code.len()
        )
    }

    pub fn get_opcode(&self) -> CoreResult<Opcode> {
        let program = self.program.as_ref()
            .ok_or(CoreError::NoProgram)?;
        println!("Getting opcode {:X} ...", program.code[self.ip]);
        println!("Opcode: {:?}", Opcode::from(program.code[self.ip]));
        Ok(
            Opcode::from(program.code[self.ip])
        )
    }

    pub fn run(&mut self) -> CoreResult<()> {
        self.run_at(0)
    }

    pub fn run_fn(&mut self, uid: u64) -> CoreResult<()> {
        let fn_offset = {
            let program = self.program.as_ref()
                .ok_or(CoreError::NoProgram)?;
            program.functions.get(&uid)
                .ok_or(CoreError::NoProgram)?
                .clone()
        };

        self.run_at(fn_offset)
    }

    pub fn run_at(&mut self, offset: usize) -> CoreResult<()> {
        self.ip = offset;

        let program_len = {
            let program = self.program.as_ref()
                .ok_or(CoreError::NoProgram)?;
            program.get_size()
        };

        while self.ip < program_len {
            println!("ip: {}", self.ip);
            let opcode = self.get_opcode()?;
            self.ip += 1;

            match opcode {
                Opcode::NOOP => {
                    continue;
                },
                Opcode::PUSHI => {
                    let op: i64 = self.get_op()?;
                    self.push_stack(op)?;
                },
                Opcode::POPI => {
                    let _: i64 = self.pop_stack()?;
                },
                Opcode::POPN => {
                    let op: u64 = self.get_op()?;
                    self.pop_n(op)?;
                },
                Opcode::DUPI => {
                    let op: i64 = self.get_op()?;
                    self.dupn_stack(op, 8)?;
                },
                Opcode::ADDI => {
                    let rhs: i64 = self.pop_stack()?;
                    let lhs: i64 = self.pop_stack()?;
                    self.push_stack(lhs + rhs)?;
                },
                Opcode::SUBI => {
                    let rhs: i64 = self.pop_stack()?;
                    let lhs: i64 = self.pop_stack()?;
                    self.push_stack(lhs - rhs)?;
                },
                Opcode::MULI => {
                    let rhs: i64 = self.pop_stack()?;
                    let lhs: i64 = self.pop_stack()?;
                    self.push_stack(lhs * rhs)?;
                },
                Opcode::DIVI => {
                    let rhs: i64 = self.pop_stack()?;
                    let lhs: i64 = self.pop_stack()?;
                    self.push_stack(lhs / rhs)?;
                },
                Opcode::CALL => {
                    self.call()?;
                },
                Opcode::RET => {
                    if self.call_stack.len() == 0 {
                        println!("Call stack is empty. Halting the core...");
                        break;
                    }
                    self.ret()?;
                },
                Opcode::MOVI => {
                    let op: i64 = self.get_op()?;
                    let target_index = (self.sp as i64 + op) as usize;
                    self.movn(target_index, 8)?;
                },
                Opcode::SVSWPI => {
                    let op: i64 = self.pop_stack()?;
                    println!("Swapping out int {}", op);
                    self.save_swap(op)?;
                },
                Opcode::LDSWPI => {
                    let op: i64 = self.load_swap()?;
                    println!("Swapping in int {}", op);
                    self.push_stack(op)?;
                },
                _ => {
                    return Err(CoreError::UnimplementedOpcode(opcode));
                }
            };
        }
        Ok(())
    }

    fn call(&mut self) -> CoreResult<()> {
        let fn_uid: u64 = self.get_op()?;
        self.call_stack.push_front(self.ip);

        let program = self.program.as_ref()
            .ok_or(CoreError::NoProgram)?;

        let new_ip = program.functions.get(&fn_uid)
            .ok_or(CoreError::UnknownFunctionUid)?;

        self.ip = *new_ip;

        Ok(())
    }

    fn ret(&mut self) -> CoreResult<()> {
        let old_ip = self.call_stack.pop_front()
            .ok_or(CoreError::EmptyCallStack)?;
        self.ip = old_ip;
        Ok(())
    }

    fn movn(&mut self, target_index: usize, size: usize) -> CoreResult<()> {
        self.sp -= size;

        for i in 0..size {
            self.stack[target_index + i] = self.stack[self.sp + i];
        }

        Ok(())
    }

    fn get_op<T: DeserializeOwned>(&mut self) -> CoreResult<T> {
        let op_size = size_of::<T>();

        let program = &self.program.as_ref().unwrap().code;

        let raw_bytes: &[u8] = &program[self.ip..self.ip + op_size];

        let ret: T = deserialize(raw_bytes)
            .map_err(|_| CoreError::OperatorDeserialize)?;

        self.ip += op_size;

        Ok(ret)
    }

    pub fn push_stack<T: Serialize>(&mut self, item: T) -> CoreResult<()> {
        let op_size = size_of::<T>();

        let raw_bytes = serialize(&item)
            .map_err(|_| CoreError::OperatorSerialize)?;

        if self.stack.len() - (self.sp + op_size) <= STACK_GROW_THRESHOLD {
            self.stack.resize(self.stack.len() + STACK_GROW_INCREMENT, 0);
        } 
        
        for i in 0..op_size {
            self.stack[self.sp + i] = raw_bytes[i];
        }
        
        self.sp += op_size;

        Ok(())
    }

    pub fn pop_stack<T: DeserializeOwned>(&mut self) -> CoreResult<T> {
        let op_size = size_of::<T>();

        let mut raw_bytes = Vec::with_capacity(op_size);
        raw_bytes.resize(op_size, 0);

        self.sp -= op_size;
        if self.sp < 0 {
            return Err(CoreError::InvalidStackPointer);
        }

        for i in 0..op_size {
            raw_bytes[i] = self.stack[self.sp + i];
        }

        deserialize(&raw_bytes)
            .map_err(|_| CoreError::Unknown)
    }

    pub fn dupn_stack(&mut self, offset: i64, size: usize) -> CoreResult<()> {
        if self.stack.len() - (self.sp + size) <= STACK_GROW_THRESHOLD {
            self.stack.resize(self.stack.len() + STACK_GROW_INCREMENT, 0);
        }
        
        let tmp_sp = (self.sp as i64 + offset) as usize;

        for i in 0..size {
            self.stack[self.sp + i] = self.stack[tmp_sp + i];
        }

        self.sp += size;

        Ok(())
    }

    pub fn pop_n(&mut self, n: u64) -> CoreResult<Vec<u8>> {
        let mut ret = Vec::new();

        self.sp -= n as usize;
        if self.sp < 0 {
            return Err(CoreError::InvalidStackPointer);
        }

        for i in 0..n {
            ret.push(self.stack[self.sp + i as usize]);
        }
        
        Ok(ret)
    }

    fn load_swap<T: DeserializeOwned>(&mut self) -> CoreResult<T> {
        let op_size = size_of::<T>();

        let mut raw_bytes = Vec::new();
        raw_bytes.resize(op_size, 0);

        for i in 0..op_size {
            raw_bytes[i] = self.swap[i];
        }

        let ret = deserialize(&raw_bytes)
            .map_err(|_| CoreError::OperatorDeserialize)?;
        Ok(ret)
    }

    fn save_swap<T: Serialize>(&mut self, item: T) -> CoreResult<()> {
        let op_size = size_of::<T>();

        if self.swap.len() < op_size {
            self.swap.resize(self.swap.len() + op_size, 0);
        }

        let raw_bytes = serialize(&item)
            .map_err(|_| CoreError::OperatorSerialize)?;

        for i in 0..op_size {
            self.swap[i] = raw_bytes[i];
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::{
        vm::{
            core::*,
            is::Opcode
        },
        codegen::{
            program::Program
        }
    };

    use bincode::serialize;
    #[test]
    fn test_addi() {
        let mut code: Vec<u8> = Vec::new();
        code.push(Opcode::PUSHI.into());
        let x: i64 = 4;
        let y: i64 = 6;
        let mut x_bytes = serialize(&x).unwrap();
        let mut y_bytes = serialize(&y).unwrap();
        code.append(&mut x_bytes);
        code.push(Opcode::PUSHI.into());
        code.append(&mut y_bytes);
        code.push(Opcode::ADDI.into());

        let program = Program::new().with_code(code);

        let mut core = Core::new(1024);
        core.load_program(program);
        let run_res = core.run();
        assert!(run_res.is_ok());
        let stack_res = core.pop_stack::<i64>();
        assert!(stack_res.is_ok());
        assert_eq!(stack_res.unwrap(), 10);
    }

    #[test]
    fn test_push_pop_stack() {
        let mut code: Vec<u8> = Vec::new();
        code.push(Opcode::PUSHI.into());
        let x: i64 = 4;
        let y: i64 = 6;
        let mut x_bytes = serialize(&x).unwrap();
        let mut y_bytes = serialize(&y).unwrap();
        code.append(&mut x_bytes);

        let program = Program::new().with_code(code);

        let mut core = Core::new(1024);
        core.load_program(program);
        let run_res = core.run();
        assert!(run_res.is_ok());
        let stack_res = core.pop_stack::<i64>();
        assert!(stack_res.is_ok());
        assert_eq!(stack_res.unwrap(), 4);
    }
}