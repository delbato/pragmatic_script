use std::{
    convert::{
        From,
        Into
    },
    fmt::{
        UpperHex
    }
};

use epd::*;

use num_traits::FromPrimitive;

#[derive(PartialEq, Debug, Clone, Primitive)]
pub enum Opcode {
    NOOP = 0x00, 
    ADDI = 0x01, 
    SUBI = 0x02, 
    MULI = 0x03, 
    DIVI = 0x04,
    ADDF = 0x05,
    SUBF = 0x06,
    MULF = 0x07,
    DIVF = 0x08,
    ITOF = 0x09,
    FTOI = 0x0A,
    EQI = 0x0B,
    GTI = 0x0C,
    LTI = 0x0D,
    GTEQI = 0x0E,
    LTEQI = 0x0F,
    EQF = 0x10,
    GTF = 0x11,
    LTF = 0x12,
    GTEQF = 0x13,
    LTEQF = 0x14,
    NOT = 0x15,
    JMP = 0x16,
    JMPT = 0x17,
    JMPF = 0x18,
    CALL = 0x19,
    RET = 0x20,
    PUSHI = 0x21,
    PUSHF = 0x22,
    PUSHB = 0x23,
    PUSHN = 0x24,
    POPI = 0x25,
    POPF = 0x26,
    POPB = 0x27,
    POPN = 0x28,
    LDI = 0x29,
    LDF = 0x30,
    LDB = 0x31,
    LDN = 0x32,
    DUPI = 0x33,
    DUPF = 0x34,
    DUPB = 0x35,
    DUPN = 0x36,
    MOVI = 0x37,
    MOVF = 0x38,
    MOVB = 0x39,
    MOVN = 0x40,
    SVSWPI = 0x41,
    SVSWPF = 0x42,
    SVSWPB = 0x43,
    SVSWPN = 0x44,
    LDSWPI = 0x45,
    LDSWPF = 0x46,
    LDSWPB = 0x47,
    LDSWPN = 0x48
}

impl From<u8> for Opcode {
    fn from(val: u8) -> Opcode {
        Opcode::from_u8(val).unwrap()
    }
}

impl Into<u8> for Opcode {
    fn into(self) -> u8 {
        self as u8
    }
}