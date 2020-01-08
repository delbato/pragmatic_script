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
    PUSHA = 0x25,
    POPI = 0x26,
    POPF = 0x27,
    POPB = 0x28,
    POPN = 0x29,
    LDI = 0x2A,
    LDF = 0x2B,
    LDB = 0x2C,
    LDN = 0x2D,
    SDUPI = 0x2E,
    SDUPF = 0x2F,
    SDUPB = 0x30,
    SDUPN = 0x31,
    SMOVI = 0x32,
    SMOVF = 0x33,
    SMOVB = 0x34,
    SMOVN = 0x35,
    SVSWPI = 0x36,
    SVSWPF = 0x37,
    SVSWPB = 0x38,
    SVSWPN = 0x39,
    LDSWPI = 0x3A,
    LDSWPF = 0x3B,
    LDSWPB = 0x3C,
    LDSWPN = 0x3D,
    SREF = 0x3E,
    MOVI = 0x3F,
    MOVF = 0x40,
    MOVB = 0x41,
    MOVN = 0x42,
    DUPI = 0x43,
    DUPF = 0x44,
    DUPB = 0x45,
    DUPN = 0x46,
    SDUPA = 0x47
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