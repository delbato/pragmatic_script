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
            var:int y = 0;
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