use super::{
    is::{
        Opcode
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

pub struct Core {
    stack: VecDeque<u8>,
    program: Vec<u8>,
    stack_frames: VecDeque<usize>,
    call_stack: VecDeque<usize>,
    ip: usize,
    function_uid_map: HashMap<u64, usize>
}

#[derive(Debug)]
pub enum CoreError {
    Unknown,
    UnimplementedOpcode(Opcode),
    OperatorDeserialize,
    OperatorSerialize,
    EmptyCallStack,
    UnknownFunctionUid
}

impl Core {
    pub fn new(program: Vec<u8>, stack_size: usize) -> Core {
        Core {
            program: program,
            stack: VecDeque::with_capacity(stack_size),
            stack_frames: VecDeque::new(),
            call_stack: VecDeque::new(),
            function_uid_map: HashMap::new(),
            ip: 0
        }
    }

    pub fn run(&mut self) -> CoreResult<()> {
        self.ip = 0;

        while self.ip < self.program.len() {
            let opcode = Opcode::from(self.program[self.ip]);
            self.ip += 1;

            match opcode {
                Opcode::NOOP => {
                    continue;
                },
                Opcode::PUSHI => {
                    let op: i64 = self.get_op()?;
                    self.ip += size_of::<i64>();
                    self.push_stack(op)?;
                },
                Opcode::POPI => {
                    let _: i64 = self.pop_stack()?;
                },
                Opcode::POPN => {
                    let op: u64 = self.get_op()?;
                    self.pop_n(op)?;
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
                    self.ret()?;
                },
                _ => {
                    return Err(CoreError::UnimplementedOpcode(opcode));
                }
            };
        }
        Ok(())
    }

    pub fn run_at(&mut self, offset: usize) -> CoreResult<()> {
        Err(CoreError::Unknown)
    }

    fn call(&mut self) -> CoreResult<()> {
        let fn_uid: u64 = self.get_op()?;
        self.call_stack.push_front(self.ip);

        let new_ip = self.function_uid_map.get(&fn_uid)
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

    fn get_op<T: DeserializeOwned>(&mut self) -> CoreResult<T> {
        let op_size = size_of::<T>();

        let raw_bytes: &[u8] = &self.program[self.ip..self.ip + op_size];

        let ret: T = deserialize(raw_bytes)
            .map_err(|_| CoreError::OperatorDeserialize)?;

        Ok(ret)
    }

    pub fn push_stack<T: Serialize>(&mut self, item: T) -> CoreResult<()> {
        let op_size = size_of::<T>();

        let raw_bytes = serialize(&item)
            .map_err(|_| CoreError::OperatorSerialize)?;
        
        for i in 0..op_size {
            self.stack.push_front(raw_bytes[i]);
        }

        Ok(())
    }

    pub fn pop_stack<T: DeserializeOwned>(&mut self) -> CoreResult<T> {
        let op_size = size_of::<T>();

        let mut raw_bytes = Vec::with_capacity(op_size);
        raw_bytes.resize(op_size, 0);
        for i in (0..op_size).rev() {
            let stack_val = self.stack.pop_front()
                .ok_or(CoreError::Unknown)?;
            raw_bytes[i] = stack_val;
        }

        deserialize(&raw_bytes)
            .map_err(|_| CoreError::Unknown)
    }

    pub fn pop_n(&mut self, n: u64) -> CoreResult<Vec<u8>> {
        let mut ret = Vec::new();

        for i in 0..n {
            let val = self.stack.pop_front()
                .ok_or(CoreError::Unknown)?;
            ret.push(val);
        }

        ret.reverse(); // Reverse due to how a stack works
        Ok(ret)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        vm::{
            core::*,
            is::Opcode
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

        let mut core = Core::new(code, 1024);
        let run_res = core.run();
        assert!(run_res.is_ok());
        let stack_res = core.pop_stack::<i64>();
        assert!(stack_res.is_ok());
        assert_eq!(stack_res.unwrap(), 10);
    }
}