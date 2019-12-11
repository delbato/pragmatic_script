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