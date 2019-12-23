extern crate clap;
extern crate pgs;

use pgs::{
    engine::Engine,
    api::{
        function::{
            Function,
            FunctionError,
            FunctionResult
        },
        module::{
            Module
        }
    },
    parser::{
        ast::Type
    },
    vm::{
        core::Core
    }
};

use std::{
    path::Path,
    error::Error,
    boxed::Box
};

use clap::{
    App,
    SubCommand,
    Arg
};

fn bootstrap_engine(engine: &mut Engine) {
    let println_function = Function::new(String::from("println"), Vec::new())
        .with_argument(Type::String)
        .with_return_type(Type::Int)
        .with_callback(
            Box::new(move |core: &mut Core| {
                let string_addr: u64 = core.get_stack(-8)
                    .map_err(|_| FunctionError::Unknown)?;
                let string = core.get_mem_string(string_addr)
                    .map_err(|_| FunctionError::Unknown)?;
                println!("{}", string);
                core.push_stack::<i64>(69)
                    .map_err(|_| FunctionError::Unknown)
            })
        );

    let print_function = Function::new(String::from("print"), Vec::new())
        .with_argument(Type::String)
        .with_return_type(Type::Int)
        .with_callback(
            Box::new(move |core: &mut Core| {
                let string_addr: u64 = core.get_stack(-8)
                    .map_err(|_| FunctionError::Unknown)?;
                let string = core.get_mem_string(string_addr)
                    .map_err(|_| FunctionError::Unknown)?;
                print!("{}", string);
                println!("Pushing 69 on stack...");
                core.push_stack::<i64>(69)
                    .map_err(|_| FunctionError::Unknown)
            })
        );

    let printi_function = Function::new(String::from("printi"), Vec::new())
        .with_argument(Type::Int)
        .with_return_type(Type::Int)
        .with_callback(
            Box::new(move |core: &mut Core| {
                let param: i64 = core.get_stack(-8)
                    .map_err(|_| FunctionError::Unknown)?;
                print!("{}", param);
                core.push_stack::<i64>(69)
                    .map_err(|_| FunctionError::Unknown)
            })
        );
    
    let module = Module::new(String::from("std"))
        .with_function(println_function)
        .with_function(print_function)
        .with_function(printi_function);
    
    let reg_res = engine.register_module(module);
    assert!(reg_res.is_ok());
}


fn build_app<'a>() -> App<'a, 'a> {
    App::new("pgsh")
        .author("Daniel Wanner <daniel.wanner@pm.me>")
        .about("PragmaticScript shell interpreter")
        .arg(
            Arg::with_name("filename")
                .index(1)
                .takes_value(true)
                .help("Filename of the script to execute")
        )
}

fn main() -> Result<(), Box<dyn Error>> {
    let app = build_app();

    let app_matches = app.get_matches();

    let filename_opt = app_matches.value_of("filename");
    assert!(filename_opt.is_some());

    let filename = filename_opt.unwrap();

    let mut engine = Engine::new(1024);

    bootstrap_engine(&mut engine);

    engine.run_file(Path::new(filename))?;

    println!("Script run. stack size: {}", engine.get_stack_size());

    let exit_code = engine.pop_stack::<i64>()?;

    println!("Script exited. Stack size: {}, Exit code: 0x{:X}/{}", engine.get_stack_size(), exit_code, exit_code);

    Ok(())
}
