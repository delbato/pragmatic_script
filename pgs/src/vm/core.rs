use super::{
    is::{
        Opcode
    },
    address::{
        Address,
        AddressType
    }
};
use crate::{
    codegen::{
        program::Program
    },
    api::{
        module::Module,
        function::*
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
    },
    ops::Range,
    fmt::{
        Debug,
        Display,
        Formatter,
        Result as FmtResult
    },
    error::Error
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

use rand::{
    Rng,
    RngCore,
    thread_rng
};

pub type CoreResult<T> = Result<T, CoreError>;

pub const STACK_GROW_INCREMENT: usize = 1024;
pub const STACK_GROW_THRESHOLD: usize = 64;
pub const SWAP_SPACE_SIZE: usize = 64;

pub struct Core {
    stack: Vec<u8>,
    heap: Vec<u8>,
    heap_pointers: Vec<Range<usize>>,
    foreign_functions: HashMap<u64, Box<dyn FnMut(&mut Core) -> FunctionResult<()>>>,
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

impl Display for CoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}

impl Error for CoreError {
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
            heap: Vec::new(),
            heap_pointers: Vec::new(),
            foreign_functions: HashMap::new(),
            stack_frames: VecDeque::new(),
            call_stack: VecDeque::new(),
            ip: 0,
            sp: 0
        }
    }

    #[inline]
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

    #[inline]
    pub fn get_opcode(&mut self) -> CoreResult<Opcode> {
        let program = self.program.as_ref()
            .ok_or(CoreError::NoProgram)?;
        //println!("Getting opcode {:X} ...", program.code[self.ip]);
        //println!("Opcode: {:?}", Opcode::from(program.code[self.ip]));
        let opcode = Opcode::from(program.code[self.ip]);
        self.ip += 1;
        
        Ok(
            opcode
        )
    }

    #[inline]
    pub fn run(&mut self) -> CoreResult<()> {
        self.run_at(0)
    }
    
    #[inline]
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

        //println!("Program length: {}", program_len);

        while self.ip < program_len {
            //println!("ip: {}", self.ip);
            let opcode = self.get_opcode()?;
            //println!("Stack values: {:?}", &self.stack[0..self.sp]);
            //println!("IP: {}", self.ip);

            match opcode {
                Opcode::NOOP => {
                    continue;
                },
                Opcode::PUSHI => {
                    let op: i64 = self.get_op()?;
                    self.push_stack(op)?;
                },
                Opcode::PUSHB => {
                    let op: bool = self.get_op()?;
                    self.push_stack(op)?;
                },
                Opcode::POPI => {
                    let _: i64 = self.pop_stack()?;
                },
                Opcode::POPN => {
                    let op: u64 = self.get_op()?;
                    //println!("Stack size before POPN: {}", self.sp);
                    //println!("Attempting to pop {} bytes off the stack", op);
                    //println!("Stack pointer: {}", self.sp);
                    self.pop_n(op)?;
                    //println!("Stack size after POPN: {}", self.sp);
                },
                Opcode::SDUPI => {
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
                        //println!("Call stack is empty. Halting the core...");
                        break;
                    }
                    self.ret()?;
                },
                Opcode::SMOVI => {
                    let op: i64 = self.get_op()?;
                    let target_index = (self.sp as i64 + op) as usize;
                    self.movn(target_index, 8)?;
                },
                Opcode::SVSWPI => {
                    let op: i64 = self.pop_stack()?;
                    //println!("Swapping out int {}", op);
                    self.save_swap(op)?;
                },
                Opcode::LDSWPI => {
                    let op: i64 = self.load_swap()?;
                    //println!("Swapping in int {}", op);
                    self.push_stack(op)?;
                },
                Opcode::JMP => {
                    let op: u64 = self.get_op()?;
                    self.ip = op as usize;
                },
                Opcode::JMPF => {
                    let op: u64 = self.get_op()?;
                    let jump: bool = self.pop_stack()?;
                    if !jump {
                        self.ip = op as usize;
                    }
                },
                Opcode::EQI => {
                    let rhs: i64 = self.pop_stack()?;
                    let lhs: i64 = self.pop_stack()?;
                    self.push_stack(lhs == rhs)?;
                },
                Opcode::LTI => {
                    let rhs: i64 = self.pop_stack()?;
                    let lhs: i64 = self.pop_stack()?;
                    self.push_stack(lhs < rhs)?;
                },
                Opcode::SDUPA => {
                    let op_offset: i64 = self.get_op()?;
                    //println!("SDUPA offset: {}", op_offset);
                    //println!("Stack size: {}", self.stack.len());
                    self.dupn_stack(op_offset, 8)?;
                },
                Opcode::PUSHA => {
                    let op: u64 = self.get_op()?;
                    //println!("pushing addresss {}! ", op);
                    self.push_stack(op)?;
                    //println!("stack pointer: {}", self.sp);
                },
                _ => {
                    return Err(CoreError::UnimplementedOpcode(opcode));
                }
            };
        }
        Ok(())
    }

    #[inline]
    fn call(&mut self) -> CoreResult<()> {
        let fn_uid: u64 = self.get_op()?;
        if let Some(mut closure) = self.foreign_functions.remove(&fn_uid) {
            //println!("Executing foreign function...");
            closure(self)
                .map_err(|_| CoreError::Unknown)?;
            self.foreign_functions.insert(fn_uid, closure);
            return Ok(());
        }

        let program = self.program.as_ref()
            .ok_or(CoreError::NoProgram)?;

        let new_ip = program.functions.get(&fn_uid)
            .ok_or(CoreError::UnknownFunctionUid)?;

        
        let old_ip = self.ip;
        self.call_stack.push_front(old_ip);
        self.ip = *new_ip;

        Ok(())
    }

    #[inline]
    fn ret(&mut self) -> CoreResult<()> {
        let old_ip = self.call_stack.pop_front()
            .ok_or(CoreError::EmptyCallStack)?;
        self.ip = old_ip;
        Ok(())
    }

    #[inline]
    fn movn(&mut self, target_index: usize, size: usize) -> CoreResult<()> {
        self.sp -= size;

        for i in 0..size {
            self.stack[target_index + i] = self.stack[self.sp + i];
        }

        Ok(())
    }

    #[inline]
    fn sp_offset_to_address(&self, offset: i64) -> CoreResult<i64> {
        let sp = self.sp as i64;

        let addr = ((sp - offset) + 1) * -1;

        Ok(addr)
    }

    pub fn get_mem_string(&self, address: u64) -> CoreResult<String> {
        let address = Address::from(address);
        if address.address_type != AddressType::Program {
            return Err(CoreError::Unknown)?;
        }

        let program = self.program.as_ref()
            .ok_or(CoreError::Unknown)?;

        let string_range = program
            .static_pointers
            .get(&(address.real_address as usize))
            .cloned()
            .ok_or(CoreError::Unknown)?;

        let mut bytes = Vec::new();

        for i in string_range {
            bytes.push(program.code[i]);
        }

        let string = unsafe {
            String::from_utf8_unchecked(bytes)
        };
        Ok(string)
    }

    #[inline]
    fn get_mem<T: DeserializeOwned>(&mut self, address: i64) -> CoreResult<T> {
        let op_size = size_of::<T>();

        let mut raw_bytes = Vec::with_capacity(op_size);
        raw_bytes.resize(op_size, 0);

        // If accessing the stack
        if address < 0 {
            let addr_usize = (i64::abs(address) as usize) - 1;

            for i in 0..op_size {
                raw_bytes[i] = self.stack[addr_usize + i];
            }
        } else { // If accessing the heap
            let addr_usize = address as usize;

            for i in 0..op_size {
                raw_bytes[i] = self.heap[addr_usize + i];
            }
        }

        deserialize(&raw_bytes)
            .map_err(|_| CoreError::OperatorDeserialize)
    }

    #[inline]
    fn set_mem<T: Serialize>(&mut self, address: i64, item: T) -> CoreResult<()> {
        let op_size = size_of::<T>();

        let raw_bytes = serialize(&item)
            .map_err(|_| CoreError::OperatorSerialize)?;

        if address < 0 {
            let addr_usize = (i64::abs(address) as usize) - 1;

            for i in 0..op_size {
                self.stack[addr_usize + i] = raw_bytes[i];
            }
        } else {
            let addr_usize = address as usize;
            
            for i in 0..op_size {
                self.heap[addr_usize + i] = raw_bytes[i];
            }
        }

        Ok(())
    }

    #[inline]
    fn get_op<T: DeserializeOwned>(&mut self) -> CoreResult<T> {
        let op_size = size_of::<T>();

        let program = &self.program.as_ref().unwrap().code;

        let raw_bytes: &[u8] = &program[self.ip..self.ip + op_size];
        //println!("get_op raw bytes: {:?}", raw_bytes);

        let ret: T = deserialize(raw_bytes)
            .map_err(|_| CoreError::OperatorDeserialize)?;

        self.ip += op_size;

        Ok(ret)
    }

    #[inline]
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

    #[inline]
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

    #[inline]
    pub fn dupn_stack(&mut self, offset: i64, size: usize) -> CoreResult<()> {
        if self.stack.len() - (self.sp + size) <= STACK_GROW_THRESHOLD {
            self.stack.resize(self.stack.len() + STACK_GROW_INCREMENT, 0);
        }
        
        let tmp_sp = (self.sp as i64 + offset) as usize;
        
        //println!("Duplicating stack from {} to {}", tmp_sp, tmp_sp + size);

        for i in 0..size {
            self.stack[self.sp + i] = self.stack[tmp_sp + i];
        }

        self.sp += size;

        Ok(())
    }

    #[inline]
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

    #[inline]
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

    #[inline]
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

    #[inline]
    pub fn get_stack<T: DeserializeOwned>(&self, offset: i64) -> CoreResult<T> {
        let sp;
        if offset < 0 {
            sp = self.sp - i64::abs(offset) as usize;
        } else {
            sp = self.sp + i64::abs(offset) as usize;
        }

        let type_size = size_of::<T>();
        let mut raw_bytes = Vec::new();
        for i in sp..sp + type_size {
            raw_bytes.push(self.stack[i]);
        }

        deserialize(&raw_bytes)
            .map_err(|_| CoreError::OperatorDeserialize)
    }

    #[inline]
    pub fn get_stack_size(&self) -> usize {
        self.sp
    }

    pub fn register_foreign_module(&mut self, module: Module) -> CoreResult<()> {
        for function in module.functions {
            let raw_callback = function.raw_callback
                .ok_or(CoreError::Unknown)?;
            let uid = function.uid
                .ok_or(CoreError::UnknownFunctionUid)?;
            self.foreign_functions.insert(uid, raw_callback);
        }
        for (_, module) in module.modules {
            self.register_foreign_module(module)?;
        }
        Ok(())
    }
}
