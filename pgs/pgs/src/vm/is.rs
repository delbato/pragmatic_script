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
#[allow(non_camel_case_types)]
pub enum Opcode {
    NOOP = 0,
    HALT = 1,
    MOVB = 2,
    MOVF = 4,
    MOVI = 5,
    MOVA = 6,
    MOVB_A = 7,
    MOVF_A = 8,
    MOVI_A = 9,
    MOVA_A = 10,
    MOVN_A = 11,
    MOVB_AR = 12,
    MOVF_AR = 13,
    MOVI_AR = 14,
    MOVA_AR = 15,
    MOVB_RA = 16,
    MOVF_RA = 17,
    MOVI_RA = 18,
    MOVA_RA = 19,
    LDB = 20,
    LDF = 21,
    LDI = 22,
    LDA = 23,
    ADDI = 24,
    SUBI = 25,
    MULI = 26,
    DIVI = 27,
    UADDI = 28,
    USUBI = 29,
    UMULI = 30,
    UDIVI = 31,
    ADDF = 32,
    SUBF = 33,
    MULF = 34,
    DIVF = 35,
    JMP = 36,
    JMPT = 37,
    JMPF = 38,
    DJMP = 39,
    DJMPT = 40,
    DJMPF = 41,
    CALL = 42,
    RET = 43,
    NOT = 44,
    EQI = 45,
    NEQI = 46,
    LTI = 47,
    GTI = 48,
    LTEQI = 49,
    GTEQI = 50,
    EQF = 51,
    NEQF = 52,
    LTF = 53,
    GTF = 54,
    LTEQF = 55,
    GTEQF = 56
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