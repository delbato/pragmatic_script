extern crate logos;
extern crate serde;
extern crate byteorder;
extern crate bincode;
extern crate rand;
#[macro_use] extern crate memoffset;

pub mod parser;

pub mod vm;

pub mod codegen;

pub mod engine;