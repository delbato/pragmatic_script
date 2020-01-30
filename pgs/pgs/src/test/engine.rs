use crate::{
    codegen::{
        compiler::Compiler,
        register::Register
    },
    parser::{
        parser::Parser
    },
    engine::Engine
};

#[test]
fn test_engine_simple_function() {
    let code = String::from("
        fn: main() ~ int {
            return 4;
        }
    ");

    let mut engine = Engine::new(1024);
    let load_res = engine.load_code(&code);
    assert!(load_res.is_ok());

    let builder = engine.compiler.get_builder();
    for instr in builder.instructions.iter() {
        println!("{:?}", instr);
    }

    let run_res = engine.run_fn(&String::from("root::main"));
    println!("{:?}", run_res);
    assert!(run_res.is_ok());

    let result_res = engine.get_register_value::<i64>(Register::R0);
    assert!(result_res.is_ok());

    assert_eq!(4, result_res.unwrap());
}

#[test]
fn test_engine_if_else() {
    let code = String::from("
        fn: main() ~ int {
            var x: int = 4;
            if x == 5 {
                x = 2;
            } else {
                x = 1;
            }
            return x;
        }
    ");

    let mut engine = Engine::new(1024);
    let load_res = engine.load_code(&code);
    println!("{:?}", load_res);
    assert!(load_res.is_ok());

    /*
    let builder = engine.compiler.get_builder();
    for instr in builder.instructions.iter() {
        println!("{:?}", instr);
    }*/

    let run_res = engine.run_fn(&String::from("root::main"));
    println!("{:?}", run_res);
    assert!(run_res.is_ok());

    let result_res = engine.get_register_value::<i64>(Register::R0);
    assert!(result_res.is_ok());

    assert_eq!(1, result_res.unwrap());
}

#[test]
fn test_engine_if_else_if() {
    let code = String::from("
        fn: main() ~ int {
            var x: int = 4;
            if x == 5 {
                x = 2;
            } else if x == 4 {
                x = 3;
            } else {
                x = 1;
            }
            return x;
        }
    ");

    let mut engine = Engine::new(1024);
    let load_res = engine.load_code(&code);
    println!("{:?}", load_res);
    assert!(load_res.is_ok());

    let mut offset = 0;
    let builder = engine.compiler.get_builder();
    for instr in builder.instructions.iter() {
        println!("{}: {:?}", offset, instr);
        offset += instr.get_size();
    }

    let run_res = engine.run_fn(&String::from("root::main"));
    println!("{:?}", run_res);
    assert!(run_res.is_ok());

    let result_res = engine.get_register_value::<i64>(Register::R0);
    assert!(result_res.is_ok());

    assert_eq!(3, result_res.unwrap());
    assert_eq!(0, engine.get_stack_size());
}