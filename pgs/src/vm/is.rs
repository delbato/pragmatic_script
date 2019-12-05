use std::{
    convert::{
        From,
        Into
    },
    fmt::{
        UpperHex
    }
};

#[derive(PartialEq, Debug, Clone)]
pub enum Opcode {
    /// 0x00
    NOOP, // 0x00
    PUSHI, // 0x01
    POPI, // 0x02
    POPN, // 0x03
    ADDI, // 0x04
    SUBI, // 0x05
    MULI, // 0x06
    DIVI, // 0x07
    CALL, // 0x08
    RET, // 0x09
    JMP, // 0x0A
    JMPT, // 0x0B
    NOT, // 0x0C
    EQ, // 0x0D
    NEQ, // 0x0E
    GTI, // 0x0F
    LTI, // 0x10
    GTEQI, // 0x11
    LTEQI, // 0x12
    DUPI, // 0x13
    DUPN, // 0x14
    MOVI, // 0x15
    MOVF, // 0x16
}

impl From<u8> for Opcode {
    fn from(val: u8) -> Opcode {
        match val {
            0x00 => {
                Opcode::NOOP
            },
            0x01 => {
                Opcode::PUSHI
            },
            0x02 => {
                Opcode::POPI
            },
            0x03 => {
                Opcode::POPN
            },
            0x04 => {
                Opcode::ADDI
            },
            0x05 => {
                Opcode::SUBI
            },
            0x06 => {
                Opcode::MULI
            },
            0x07 => {
                Opcode::DIVI
            },
            0x08 => {
                Opcode::CALL
            },
            0x09 => {
                Opcode::RET
            },
            0x0A => {
                Opcode::JMP
            },
            0x0B => {
                Opcode::JMPT
            },
            0x0C => {
                Opcode::NOT
            },
            0x0D => {
                Opcode::EQ
            },
            0x0E => {
                Opcode::NEQ
            },
            0x0F => {
                Opcode::GTI
            },
            0x10 => {
                Opcode::LTI
            },
            0x11 => {
                Opcode::GTEQI
            },
            0x12 => {
                Opcode::LTEQI
            },
            0x13 => {
                Opcode::DUPI
            },
            0x14 => {
                Opcode::DUPN
            },
            0x15 => {
                Opcode::MOVI
            },
            0x16 => {
                Opcode::MOVF
            },
            _ => panic!("{:X} is not a valid opcode!", val)
        }
    }
}

impl Into<u8> for Opcode {
    fn into(self) -> u8 {
        self as u8
    }
}