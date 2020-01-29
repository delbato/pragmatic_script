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
    ADDI_I = 28,
    SUBI_I = 29,
    MULI_I = 30,
    DIVI_I = 31,
    ADDU = 32,
    SUBU = 33,
    MULU = 34,
    DIVU = 35,
    ADDU_I = 36,
    SUBU_I = 37,
    MULU_I = 38,
    DIVU_I = 39,
    ADDF = 40,
    SUBF = 41,
    MULF = 42,
    DIVF = 43,
    ADDF_I = 44,
    SUBF_I = 45,
    MULF_I = 46,
    DIVF_I = 47,
    JMP = 48,
    JMPT = 49,
    JMPF = 50,
    DJMP = 51,
    DJMPT = 52,
    DJMPF = 53,
    CALL = 54,
    RET = 55,
    NOT = 56,
    EQI = 57,
    NEQI = 58,
    LTI = 59,
    GTI = 60,
    LTEQI = 61,
    GTEQI = 62,
    EQF = 63,
    NEQF = 64,
    LTF = 65,
    GTF = 66,
    LTEQF = 67,
    GTEQF = 68
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