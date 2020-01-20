use super::{
    is::{
        Opcode
    },
    address::{
        Address,
        AddressType
    },
    register::{
        Register,
        RegisterAccess
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
    registers: [Register; 16],
    ip: Register,
    sp: Register,
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
    InvalidStackPointer,
    InvalidRegister
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
            registers: [Register::new(); 16],
            ip: Register::new(),
            sp: Register::new()
        }
    }

    #[inline]
    pub fn load_program(&mut self, program: Program) {
        self.program = Some(program);
    }

    #[inline]
    pub fn program_len(&self) -> CoreResult<usize> {
        let program = self.program.as_ref()
            .ok_or(CoreError::Unknown)?;
        Ok(
            program.code.len()
        )
    }

    #[inline]
    pub fn get_stack_size(&self) -> usize {
        self.sp.uint64 as usize
    }

    #[inline]
    pub fn get_opcode(&mut self) -> CoreResult<Opcode> {
        let program = self.program.as_ref()
            .ok_or(CoreError::NoProgram)?;
        //println!("Getting opcode {:X} ...", program.code[self.ip]);
        //println!("Opcode: {:?}", Opcode::from(program.code[self.ip])),

        let opcode = Opcode::from(program.code[self.ip.uint64 as usize]);
        self.ip.uint64 += 1;
        
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
        self.ip.uint64 = offset as u64;

        let program_len = {
            let program = self.program.as_ref()
                .ok_or(CoreError::NoProgram)?;
            program.get_size() as u64
        };

        //println!("Program length: {}", program_len);

        while self.ip.uint64 < program_len {
            //println!("ip: {}", self.ip);
            let opcode = self.get_opcode()?;
            //println!("Stack values: {:?}", &self.stack[0..self.sp]);
            //println!("IP: {}", self.ip);

            match opcode {
                Opcode::NOOP => {},
                Opcode::HALT => { break },
                Opcode::MOVB => {
                    let lhs: u8 = self.get_op()?;
                    let rhs: u8 = self.get_op()?;
                    let boolean: bool = {
                        self.get_reg(lhs)?.get()
                    };
                    self.get_reg(rhs)?.set(boolean);
                },
                Opcode::MOVF => {
                    let lhs: u8 = self.get_op()?;
                    let rhs: u8 = self.get_op()?;
                    let float: f32 = {
                        self.get_reg(lhs)?.get()
                    };
                    self.get_reg(rhs)?.set(float);
                },
                Opcode::MOVI => {
                    let lhs: u8 = self.get_op()?;
                    let rhs: u8 = self.get_op()?;
                    let int64: i64 = {
                        self.get_reg(lhs)?.get()
                    };
                    self.get_reg(rhs)?.set(int64);
                },
                Opcode::MOVA => {
                    let lhs: u8 = self.get_op()?;
                    let rhs: u8 = self.get_op()?;
                    let uint64: u64 = {
                        self.get_reg(lhs)?.get()
                    };
                    self.get_reg(rhs)?.set(uint64);
                },
                Opcode::MOVB_A => {
                    let lhs_reg: u8 = self.get_op()?;
                    let lhs_offset: i16 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let rhs_offset: i16 = self.get_op()?;
                    let lhs_addr: u64 = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs_addr: u64 = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.mem_mov_n((lhs_addr, lhs_offset), (rhs_addr, rhs_offset), 1)?;
                },
                Opcode::MOVF_A => {
                    let lhs_reg: u8 = self.get_op()?;
                    let lhs_offset: i16 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let rhs_offset: i16 = self.get_op()?;
                    let lhs_addr: u64 = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs_addr: u64 = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.mem_mov_n((lhs_addr, lhs_offset), (rhs_addr, rhs_offset), 4)?;
                },
                Opcode::MOVI_A => {
                    let lhs_reg: u8 = self.get_op()?;
                    let lhs_offset: i16 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let rhs_offset: i16 = self.get_op()?;
                    let lhs_addr: u64 = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs_addr: u64 = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.mem_mov_n((lhs_addr, lhs_offset), (rhs_addr, rhs_offset), 8)?;
                },
                Opcode::MOVA_A => {
                    let lhs_reg: u8 = self.get_op()?;
                    let lhs_offset: i16 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let rhs_offset: i16 = self.get_op()?;
                    let lhs_addr: u64 = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs_addr: u64 = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.mem_mov_n((lhs_addr, lhs_offset), (rhs_addr, rhs_offset), 8)?;
                },
                Opcode::MOVN_A => {
                    let lhs_reg: u8 = self.get_op()?;
                    let lhs_offset: i16 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let rhs_offset: i16 = self.get_op()?;
                    let n: usize = self.get_op::<u32>()? as usize;
                    let lhs_addr: u64 = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs_addr: u64 = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.mem_mov_n((lhs_addr, lhs_offset), (rhs_addr, rhs_offset), n)?;
                },
                Opcode::MOVB_AR => {
                    let lhs_reg: u8 = self.get_op()?;
                    let lhs_offset: i16 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let lhs_addr: u64 = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let boolean: bool = self.mem_get((lhs_addr, lhs_offset))?;
                    self.get_reg(rhs_reg)?.set(boolean);
                },
                Opcode::MOVF_AR => {
                    let lhs_reg: u8 = self.get_op()?;
                    let lhs_offset: i16 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let lhs_addr: u64 = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let float: f32 = self.mem_get((lhs_addr, lhs_offset))?;
                    self.get_reg(rhs_reg)?.set(float)
                },
                Opcode::MOVI_AR => {
                    let lhs_reg: u8 = self.get_op()?;
                    let lhs_offset: i16 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let lhs_addr: u64 = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let int64: i64 = self.mem_get((lhs_addr, lhs_offset))?;
                    self.get_reg(rhs_reg)?.set(int64)
                },
                Opcode::MOVA_AR => {
                    let lhs_reg: u8 = self.get_op()?;
                    let lhs_offset: i16 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let lhs_addr: u64 = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let uint64: u64 = self.mem_get((lhs_addr, lhs_offset))?;
                    self.get_reg(rhs_reg)?.set(uint64)
                },
                Opcode::MOVB_RA => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let rhs_offset: i16 = self.get_op()?;
                    let rhs_addr: u64 = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    let boolean: bool = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    self.mem_set((rhs_addr, rhs_offset), boolean)?;
                },
                Opcode::MOVF_RA => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let rhs_offset: i16 = self.get_op()?;
                    let rhs_addr: u64 = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    let float: f32 = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    self.mem_set((rhs_addr, rhs_offset), float)?;
                },
                Opcode::MOVI_RA => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let rhs_offset: i16 = self.get_op()?;
                    let rhs_addr: u64 = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    let int64 = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    self.mem_set((rhs_addr, rhs_offset), int64)?;
                },
                Opcode::MOVA_RA => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let rhs_offset: i16 = self.get_op()?;
                    let rhs_addr: u64 = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    let uint64: u64 = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    self.mem_set((rhs_addr, rhs_offset), uint64)?;
                },
                Opcode::LDB => {
                    let lhs_reg: u8 = self.get_op()?;
                    let boolean: bool = self.get_op()?;
                    self.get_reg(lhs_reg)?.set(boolean);
                },
                Opcode::LDF => {
                    let lhs_reg: u8 = self.get_op()?;
                    let float: f32 = self.get_op()?;
                    self.get_reg(lhs_reg)?.set(float);
                },
                Opcode::LDI => {
                    let lhs_reg: u8 = self.get_op()?;
                    let int64: i64 = self.get_op()?;
                    self.get_reg(lhs_reg)?.set(int64);
                },
                Opcode::LDA => {
                    let lhs_reg: u8 = self.get_op()?;
                    let uint64: u64 = self.get_op()?;
                    self.get_reg(lhs_reg)?.set(uint64)
                },
                Opcode::ADDI => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.get_reg(target_reg)?.int64 = lhs + rhs;
                },
                Opcode::SUBI => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.get_reg(target_reg)?.int64 = lhs - rhs;
                },
                Opcode::MULI => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.get_reg(target_reg)?.int64 = lhs * rhs;
                },
                Opcode::DIVI => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.get_reg(target_reg)?.int64 = lhs / rhs;
                },
                Opcode::UADDI => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.get_reg(target_reg)?.uint64 = lhs + rhs;
                },
                Opcode::USUBI => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.get_reg(target_reg)?.uint64 = lhs - rhs;
                },
                Opcode::UMULI => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.get_reg(target_reg)?.uint64 = lhs * rhs;
                },
                Opcode::UDIVI => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.get_reg(target_reg)?.uint64 = lhs / rhs;
                },
                Opcode::ADDF => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.get_reg(target_reg)?.float: f32 = lhs + rhs;
                },
                Opcode::SUBF => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.get_reg(target_reg)?.float: f32 = lhs - rhs;
                },
                Opcode::MULF => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.get_reg(target_reg)?.float: f32 = lhs * rhs;
                },
                Opcode::DIVF => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    self.get_reg(target_reg)?.float: f32 = lhs / rhs;
                },
                Opcode::JMP => {
                    let target_ip: u64 = self.get_op()?;
                    self.ip.uint64 = target_ip;
                },
                Opcode::JMPT => {
                    let lhs_reg: u8 = self.get_op()?;
                    let target_ip: u64 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    if lhs {
                        self.ip.uint64 = target_ip;
                    }
                },
                Opcode::JMPF => {
                    let lhs_reg: u8 = self.get_op()?;
                    let target_ip: u64 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    if !lhs {
                        self.ip.uint64 = target_ip;
                    }
                },
                Opcode::DJMP => {
                    let lhs_reg: u8 = self.get_op()?;
                    let target_ip = {
                        self.get_reg(lhs_reg)?.uint64
                    };
                    self.ip.uint64 = target_ip;
                },
                Opcode::DJMPT => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_ip = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    let lhs = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    if lhs {
                        self.ip.uint64 = target_ip;
                    }
                },
                Opcode::DJMPF => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_ip: u64 = {
                        self.get_reg(rhs_reg)?.get()
                    };
                    let lhs: bool = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    if !lhs {
                        self.ip.uint64 = target_ip;
                    }
                },
                Opcode::CALL => {
                    self.call()?;
                },
                Opcode::RET => {
                    self.ret()?;
                },
                Opcode::NOT => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.get()
                    };
                    self.get_reg(rhs_reg)?.boolean = !lhs;
                },
                Opcode::EQI => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.int64
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.int64
                    };
                    self.get_reg(target_reg)?.boolean = lhs == rhs;
                },
                Opcode::NEQI => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.int64
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.int64
                    };
                    self.get_reg(target_reg)?.boolean = lhs != rhs;
                },
                Opcode::LTI => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.int64
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.int64
                    };
                    self.get_reg(target_reg)?.boolean = lhs < rhs;
                },
                Opcode::GTI => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.int64
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.int64
                    };
                    self.get_reg(target_reg)?.boolean = lhs > rhs;
                },
                Opcode::LTEQI => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.int64
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.int64
                    };
                    self.get_reg(target_reg)?.boolean = lhs <= rhs;
                },
                Opcode::GTEQI => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.int64
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.int64
                    };
                    self.get_reg(target_reg)?.boolean = lhs >= rhs;
                },
                Opcode::EQF => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.float
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.float
                    };
                    self.get_reg(target_reg)?.boolean = lhs == rhs;
                },
                Opcode::NEQF => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.float
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.float
                    };
                    self.get_reg(target_reg)?.boolean = lhs != rhs;
                },
                Opcode::LTF => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.float
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.float
                    };
                    self.get_reg(target_reg)?.boolean = lhs < rhs;
                },
                Opcode::GTF => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.float
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.float
                    };
                    self.get_reg(target_reg)?.boolean = lhs > rhs;
                },
                Opcode::LTEQF => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.float
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.float
                    };
                    self.get_reg(target_reg)?.boolean = lhs <= rhs;
                },
                Opcode::GTEQF => {
                    let lhs_reg: u8 = self.get_op()?;
                    let rhs_reg: u8 = self.get_op()?;
                    let target_reg: u8 = self.get_op()?;
                    let lhs = {
                        self.get_reg(lhs_reg)?.float
                    };
                    let rhs = {
                        self.get_reg(rhs_reg)?.float
                    };
                    self.get_reg(target_reg)?.boolean = lhs >= rhs;
                },
                _ => {
                    return Err(CoreError::UnimplementedOpcode(opcode));
                }
            };
        }
        Ok(())
    }

    fn mem_mov_n(&mut self, lhs: (u64, i16), rhs: (u64, i16), n: usize) -> CoreResult<()> {
        let lhs_addr: u64 = Address::from(lhs.0).with_offset(lhs.1);
        let rhs_addr: u64 = Address::from(rhs.0).with_offset(rhs.1);

        let source_addr = lhs_addr.real_address as usize;
        let target_addr = rhs_addr.real_address as usize;

        let source: &[u8] = match lhs_addr.address_type {
            AddressType::Stack => {
                &self.stack
            },
            AddressType::Program => {
                let program = self.program.as_ref()
                    .ok_or(CoreError::Unknown)?;
                &program.code
            },
            AddressType::Swap => {
                &self.swap
            },
            _ => return Err(CoreError::Unknown)
        };

        match rhs_addr.address_type {
            AddressType::Stack => {
                for i in 0..n {
                    self.stack[target_addr + i] = source[source_addr + i];
                }
            },
            AddressType::Program => {
                let program = self.program.as_mut()
                    .ok_or(CoreError::Unknown)?;
                for i in 0..n {
                    program.code[target_addr + i] = source[source_addr + i];
                }
            },
            AddressType::Swap => {
                for i in 0..n {
                    self.swap[target_addr + i] = source[source_addr + i];
                }
            },
            _ => return Err(CoreError::Unknown)
        };

        Ok(())
    }

    fn mem_get_n(&self, addr: (u64, i16), n: usize) -> CoreResult<Vec<u8>> {
        let mut data = Vec::with_capacity(n);
        data.resize(n, 0);

        let lhs_addr: u64 = Address::from(addr.0).with_offset(addr.1);

        let source_addr = lhs_addr.real_address as usize;

        let source: &[u8] = match lhs_addr.address_type {
            AddressType::Stack => {
                &self.stack
            },
            AddressType::Program => {
                let program = self.program.as_ref()
                    .ok_or(CoreError::Unknown)?;
                &program.code
            },
            AddressType::Swap => {
                &self.swap
            },
            _ => return Err(CoreError::Unknown)
        };

        for i in 0..n {
            data[i] = source[source_addr + i];
        }

        Ok(data)
    }
    
    #[inline]
    pub fn mem_get_string(&self, addr: u64) -> CoreResult<String> {
        let string_size: u64 = self.mem_get((addr, 0))?;
        let string_addr: u64 = self.mem_get((addr + 8, 0))?;
        let string_data = self.mem_get_n((string_addr, 0), string_size as usize)?;
        String::from_utf8(string_data)
            .map_err(|_| CoreError::OperatorDeserialize)
    }

    #[inline]
    pub fn mem_get<T: DeserializeOwned>(&self, addr: (u64, i16)) -> CoreResult<T> {
        let n = size_of::<T>();

        let data = self.mem_get_n(addr, n)?;

        deserialize(&data)
            .map_err(|_| CoreError::OperatorDeserialize)
    }
    
    #[inline]
    pub fn mem_set<T: Serialize>(&mut self, addr: (u64, i16), item: T) -> CoreResult<()> {
        let n = size_of::<T>();

        let lhs_addr: u64 = Address::from(addr.0).with_offset(addr.1);

        let data = serialize(&item)
            .map_err(|_| CoreError::OperatorSerialize)?;

        let target_addr = lhs_addr.real_address as usize;
        
        match lhs_addr.address_type {
            AddressType::Stack => {
                for i in 0..n {
                    self.stack[target_addr + i] = data[i];
                }
            },
            AddressType::Program => {
                let program = self.program.as_mut()
                    .ok_or(CoreError::Unknown)?;
                for i in 0..n {
                    program.code[target_addr + i] = data[i];
                }
            },
            _ => return Err(CoreError::Unknown)
        };

        Ok(())
    }

    #[inline]
    pub fn get_reg(&self, reg: u8) -> CoreResult<&mut Register> {
        if reg == 16 {
            return Ok(&mut self.sp);
        }
        if reg == 17 {
            return Ok(&mut self.ip);
        }
        else if reg < 16 {
            return Ok(&mut self.registers[reg as usize]);
        }
        else {
            return Err(CoreError::InvalidRegister);
        }
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

        
        let old_ip = self.ip.uint64 as usize;
        self.call_stack.push_front(old_ip);
        self.ip.uint64 = *new_ip as u64;

        Ok(())
    }

    #[inline]
    fn ret(&mut self) -> CoreResult<()> {
        let old_ip = self.call_stack.pop_front()
            .ok_or(CoreError::EmptyCallStack)?;
        self.ip.uint64 = old_ip as u64;
        Ok(())
    }

    #[inline]
    fn get_op<T: DeserializeOwned>(&mut self) -> CoreResult<T> {
        let op_size = size_of::<T>();

        let program = &self.program.as_ref().unwrap().code;

        let tmp_ip = self.ip.uint64 as usize;

        let raw_bytes: &[u8] = &program[tmp_ip..tmp_ip + op_size];
        //println!("get_op raw bytes: {:?}", raw_bytes);

        let ret: T = deserialize(raw_bytes)
            .map_err(|_| CoreError::OperatorDeserialize)?;

        self.ip.uint64 += op_size as u64;

        Ok(ret)
    }

    #[inline]
    pub fn push_stack<T: Serialize>(&mut self, item: T) -> CoreResult<()> {
        let op_size = size_of::<T>();

        let raw_bytes = serialize(&item)
            .map_err(|_| CoreError::OperatorSerialize)?;

        let tmp_sp = self.sp.uint64 as usize;

        if self.stack.len() - (tmp_sp + op_size) <= STACK_GROW_THRESHOLD {
            self.stack.resize(self.stack.len() + STACK_GROW_INCREMENT, 0);
        } 
        
        for i in 0..op_size {
            self.stack[tmp_sp + i] = raw_bytes[i];
        }
        
        self.sp.uint64 += op_size as u64;

        Ok(())
    }

    #[inline]
    pub fn pop_stack<T: DeserializeOwned>(&mut self) -> CoreResult<T> {
        let op_size = size_of::<T>();

        let mut raw_bytes = Vec::with_capacity(op_size);
        raw_bytes.resize(op_size, 0);

        let mut tmp_sp = self.sp.uint64 as usize;

        if op_size > tmp_sp {
            return Err(CoreError::InvalidStackPointer);
        }

        tmp_sp -= op_size;

        for i in 0..op_size {
            raw_bytes[i] = self.stack[tmp_sp + i];
        }

        self.sp.uint64 = tmp_sp as u64;

        deserialize(&raw_bytes)
            .map_err(|_| CoreError::Unknown)
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
