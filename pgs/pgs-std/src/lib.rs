extern crate pgs;

use pgs::{
    engine::{
        Engine,
        EngineResult
    },
    api::{
        function::{
            Function,
            FunctionError
        },
        module::{
            Module
        }
    },
    parser::{
        ast::{
            Type
        }
    },
    vm::{
        core::Core
    }
};

fn register_std_print(engine: &mut Engine) -> EngineResult<()> {

    let fn_print = Function::new(String::from("print"))
        .with_argument(Type::String)
        .with_return_type(Type::Int)
        .with_callback(
            Box::new(move |core: &mut Core| {
                let string_addr: u64 = core.get_stack(-8)
                    .map_err(|_| FunctionError::Unknown)?;
                let string = core.get_mem_string(string_addr)
                    .map_err(|_| FunctionError::Unknown)?;
                print!("{}", string);
                core.push_stack::<i64>(0)
                    .map_err(|_| FunctionError::Unknown)
            })
        );

    let fn_println = Function::new(String::from("println"))
        .with_argument(Type::String)
        .with_return_type(Type::Int)
        .with_callback(
            Box::new(move |core: &mut Core| {
                let string_addr: u64 = core.get_stack(-8)
                    .map_err(|_| FunctionError::Unknown)?;
                let string = core.get_mem_string(string_addr)
                    .map_err(|_| FunctionError::Unknown)?;
                println!("{}", string);
                core.push_stack::<i64>(0)
                    .map_err(|_| FunctionError::Unknown)
            })
        );

    let fn_printi = Function::new(String::from("printi"))
        .with_argument(Type::Int)
        .with_return_type(Type::Int)
        .with_callback(
            Box::new(move |core: &mut Core| {
                let int: i64 = core.get_stack(-8)
                    .map_err(|_| FunctionError::Unknown)?;
                print!("{}", int);
                core.push_stack::<i64>(0)
                    .map_err(|_| FunctionError::Unknown)
            })
        );

    let module = Module::new(String::from("std"))
        .with_function(fn_print)
        .with_function(fn_printi)
        .with_function(fn_println);

    engine.register_module(module)
}

#[no_mangle]
pub extern fn register_extension(engine: &mut Engine) -> EngineResult<()> {
    register_std_print(engine)?;
    Ok(())
}