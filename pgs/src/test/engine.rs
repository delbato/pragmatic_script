use crate::{
    engine::Engine
};

#[test]
fn test_engine_run() {
    let mut engine = Engine::new(1024);
    // PUSHI 0
    // DUPI -16
    // PUSHI 10
    // MULI
    // MOVI -16
    // DUPI -8
    // SVSWPI
    // POPN 8
    // LDSWPI
    let code = "
        fn: add(lhs: int, rhs: int) ~ int {
            return lhs + rhs;
        }
        fn: main(argc: int) ~ int {
            var y: int = 0;
            y = argc * 10;
            return y;
        }
    ";

    let load_res = engine.load_code(code);
    assert!(load_res.is_ok());

    let push_res = engine.push_stack::<i64>(5);
    assert!(push_res.is_ok());
    
    let run_res = engine.run_fn(&String::from("root::main"));
    assert!(run_res.is_ok());

    let pop_res = engine.pop_stack::<i64>();
    assert!(pop_res.is_ok());

    assert_eq!(pop_res.unwrap(), 50);
}

#[test]
fn test_engine_call() {
    let code = "
        fn: ten() ~ int {
            return 10;
        }
        fn: main(argc: int) ~ int {
            var y: int = 0;
            y = argc * ten();
            return y;
        }
    ";

    let mut engine = Engine::new(1024);

    let load_res = engine.load_code(code);
    assert!(load_res.is_ok());

    let push_res = engine.push_stack::<i64>(5);
    assert!(push_res.is_ok());
    
    let run_res = engine.run_fn(&String::from("root::main"));
    assert!(run_res.is_ok());

    let pop_res = engine.pop_stack::<i64>();
    assert!(pop_res.is_ok());

    assert_eq!(pop_res.unwrap(), 50);
}

#[test]
fn test_engine_mod_call() {
    let code = "
        mod: other {
            fn: ten() ~ int {
                return 10;
            }
        }

        import other::ten = TEN;

        fn: main(argc: int) ~ int {
            var y: int = 0;
            y = argc * TEN();
            return y;
        }
    ";

    let mut engine = Engine::new(1024);

    let load_res = engine.load_code(code);
    assert!(load_res.is_ok());

    let push_res = engine.push_stack::<i64>(5);
    assert!(push_res.is_ok());
    
    let run_res = engine.run_fn(&String::from("root::main"));
    assert!(run_res.is_ok());

    let pop_res = engine.pop_stack::<i64>();
    assert!(pop_res.is_ok());

    assert_eq!(pop_res.unwrap(), 50);
}

#[test]
fn test_engine_while() {
    let code = "
        fn: main() ~ int {
            var i: int = 0;

            while i < 10 {
                i = i + 1;
            }

            return i;
        }
    ";

    let mut engine = Engine::new(64);

    let load_res = engine.load_code(code);
    assert!(load_res.is_ok());

    let run_res = engine.run_fn(&String::from("root::main"));
    assert!(run_res.is_ok());

    let pop_res = engine.pop_stack();
    assert!(pop_res.is_ok());

    let ret: i64 = pop_res.unwrap();

    assert_eq!(ret, 9);
}

use crate::{
    api::function::*,
    api::module::*,
    vm::core::{
        Core,
        CoreError
    },
    parser::ast::Type
};

#[test]
fn test_engine_foreign_function() {
    let mut engine = Engine::new(128);

    let function = Function::new(String::from("geti"), Vec::new())
        .with_return_type(Type::Int)
        .with_callback(
            Box::new(move |core: &mut Core| {
                core.push_stack::<i64>(-127)
                    .map_err(|_| FunctionError::Unknown)
            })
        );
    
    let module = Module::new(String::from("ext"))
        .with_function(function);
    
    let reg_res = engine.register_module(module);
    assert!(reg_res.is_ok());

    let code = "
        import ext::geti;

        fn: main() ~ int {
            return geti();
        }
    ";

    let load_res = engine.load_code(code);
    assert!(load_res.is_ok());

    let run_res = engine.run_fn(&String::from("root::main"));
    assert!(run_res.is_ok());

    let pop_res = engine.pop_stack::<i64>();
    assert!(pop_res.is_ok());

    assert_eq!(pop_res.unwrap(), -127);
}