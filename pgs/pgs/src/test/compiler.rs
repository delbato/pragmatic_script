use crate::{
    parser::{
        lexer::Token,
        parser::Parser,
        ast::Type
    },
    vm::{
        is::Opcode            
    },
    codegen::{
        instruction::Instruction,
        builder::Builder,
        context::FunctionContext,
        program::Program,
        compiler::Compiler
    }
};

use std::collections::HashMap;

use pglex::prelude::Lexable;

#[test]
fn test_compile_addi() {
    let code = String::from("
        var x: int = 4;
        var y: int = x + 4;
    ");

    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let stmt_list_res = parser.parse_statement_list(&mut lexer);

    assert!(stmt_list_res.is_ok());
    let stmt_list = stmt_list_res.unwrap();

    let mut compiler = Compiler::new();
    compiler.reset_builder();
    compiler.push_empty_context();

    for stmt in stmt_list {
        let cmp_res = compiler.compile_statement(&stmt);
        assert!(cmp_res.is_ok());
    }

    let mut comp_builder = Builder::new();

    let pushi_instr = Instruction::new(Opcode::PUSHI)
        .with_operand::<i64>(&4);
    let dupi_instr = Instruction::new(Opcode::SDUPI)
        .with_operand::<i64>(&-8);
    let pushi2_instr = Instruction::new(Opcode::PUSHI)
        .with_operand::<i64>(&4);
    let addi_instr = Instruction::new(Opcode::ADDI);

    comp_builder.push_instr(pushi_instr);
    comp_builder.push_instr(dupi_instr);
    comp_builder.push_instr(pushi2_instr);
    comp_builder.push_instr(addi_instr);

    let comp_code = comp_builder.build();
    let code = compiler.get_resulting_code();

    assert_eq!(comp_code, code);
}

#[test]
fn test_compile_addi_assign() {
    let code = String::from("
        var x: int = 4;
        x = x + 4;
    ");

    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let stmt_list_res = parser.parse_statement_list(&mut lexer);

    assert!(stmt_list_res.is_ok());
    let stmt_list = stmt_list_res.unwrap();

    let mut compiler = Compiler::new();
    compiler.reset_builder();
    compiler.push_empty_context();

    for stmt in stmt_list {
        let cmp_res = compiler.compile_statement(&stmt);
        assert!(cmp_res.is_ok());
    }

    let mut comp_builder = Builder::new();

    let pushi_instr = Instruction::new(Opcode::PUSHI)
        .with_operand::<i64>(&4);
    let dupi_instr = Instruction::new(Opcode::SDUPI)
        .with_operand::<i64>(&-8);
    let pushi2_instr = Instruction::new(Opcode::PUSHI)
        .with_operand::<i64>(&4);
    let addi_instr = Instruction::new(Opcode::ADDI);
    let movi_instr = Instruction::new(Opcode::SMOVI)
        .with_operand::<i64>(&-16);

    comp_builder.push_instr(pushi_instr);
    comp_builder.push_instr(dupi_instr);
    comp_builder.push_instr(pushi2_instr);
    comp_builder.push_instr(addi_instr);
    comp_builder.push_instr(movi_instr);

    let comp_code = comp_builder.build();
    let code = compiler.get_resulting_code();

    assert_eq!(comp_code, code);
}

#[test]
fn test_compile_muli_assign() {
    let code = String::from("
        var x: int = 4;
        x = x + 4;
        var z: int = x * 2;
        x = z;
        var w: int = 4;
        x = w;
    ");

    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let stmt_list_res = parser.parse_statement_list(&mut lexer);

    assert!(stmt_list_res.is_ok());
    let stmt_list = stmt_list_res.unwrap();

    let mut compiler = Compiler::new();
    compiler.reset_builder();
    compiler.push_empty_context();

    for stmt in stmt_list {
        let cmp_res = compiler.compile_statement(&stmt);
        assert!(cmp_res.is_ok());
    }

    let mut comp_builder = Builder::new();

    let pushi_instr = Instruction::new(Opcode::PUSHI) // 4
        .with_operand::<i64>(&4);
    let dupi_instr = Instruction::new(Opcode::SDUPI) // 4,4
        .with_operand::<i64>(&-8);
    let pushi2_instr = Instruction::new(Opcode::PUSHI) // 4,4,4
        .with_operand::<i64>(&4);
    let addi_instr = Instruction::new(Opcode::ADDI); // 4,8
    let movi_instr = Instruction::new(Opcode::SMOVI) // 8
        .with_operand::<i64>(&-16);
    let dupi2_instr = Instruction::new(Opcode::SDUPI) // 8,8
        .with_operand::<i64>(&-8);
    let pushi3_instr = Instruction::new(Opcode::PUSHI) // 8,8,2
        .with_operand::<i64>(&2);
    let muli_instr = Instruction::new(Opcode::MULI); // 8, 16
    let dupi3_instr = Instruction::new(Opcode::SDUPI) // 8, 16, 16
        .with_operand::<i64>(&-8);
    let movi2_instr = Instruction::new(Opcode::SMOVI) // 16, 16
        .with_operand::<i64>(&-24);
    let pushi4_instr = Instruction::new(Opcode::PUSHI) // 16, 16, 4
        .with_operand::<i64>(&4);
    let dupi4_instr = Instruction::new(Opcode::SDUPI) // 16, 16, 4, 4
        .with_operand::<i64>(&-8);
    let movi3_instr = Instruction::new(Opcode::SMOVI) // 4, 16, 4
        .with_operand::<i64>(&-32);

    comp_builder.push_instr(pushi_instr);
    comp_builder.push_instr(dupi_instr);
    comp_builder.push_instr(pushi2_instr);
    comp_builder.push_instr(addi_instr);
    comp_builder.push_instr(movi_instr);
    comp_builder.push_instr(dupi2_instr);
    comp_builder.push_instr(pushi3_instr);
    comp_builder.push_instr(muli_instr);
    comp_builder.push_instr(dupi3_instr);
    comp_builder.push_instr(movi2_instr);
    comp_builder.push_instr(pushi4_instr);
    comp_builder.push_instr(dupi4_instr);
    comp_builder.push_instr(movi3_instr);

    let comp_code = comp_builder.build();
    let code = compiler.get_resulting_code();

    assert_eq!(comp_code, code);
}

#[test]
fn test_compile_return() {
    let code = String::from("
        var x: int = 4;
        var y: int= x + 4;
        return y - 4;
    ");

    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let stmt_list_res = parser.parse_statement_list(&mut lexer);

    assert!(stmt_list_res.is_ok());
    let stmt_list = stmt_list_res.unwrap();

    let mut compiler = Compiler::new();
    compiler.reset_builder();
    let mut context = FunctionContext::new();
    context.return_type = Some(Type::Int);
    compiler.push_new_context(context);

    for stmt in stmt_list {
        let cmp_res = compiler.compile_statement(&stmt);
        assert!(cmp_res.is_ok());
    }

    let mut comp_builder = Builder::new();

    let pushi_instr = Instruction::new(Opcode::PUSHI) // 4
        .with_operand::<i64>(&4);
    let dupi_instr = Instruction::new(Opcode::SDUPI) // 4, 4
        .with_operand::<i64>(&-8);
    let pushi2_instr = Instruction::new(Opcode::PUSHI) // 4, 4, 4
        .with_operand::<i64>(&4);
    let addi_instr = Instruction::new(Opcode::ADDI); // 4, 8
    let dupi2_instr = Instruction::new(Opcode::SDUPI) // 4, 8, 8
        .with_operand::<i64>(&-8);
    let pushi3_instr = Instruction::new(Opcode::PUSHI) // 4, 8, 8, 4
        .with_operand::<i64>(&4);
    let subi_instr = Instruction::new(Opcode::SUBI); // 4, 8, 4
    let svswp_instr = Instruction::new(Opcode::SVSWPI); // 4, 8
    let popn_instr = Instruction::new(Opcode::POPN) // 
        .with_operand::<u64>(&16);
    let ldswp_instr = Instruction::new(Opcode::LDSWPI); // 4
    let ret_instr = Instruction::new(Opcode::RET);

    comp_builder.push_instr(pushi_instr);
    comp_builder.push_instr(dupi_instr);
    comp_builder.push_instr(pushi2_instr);
    comp_builder.push_instr(addi_instr);
    comp_builder.push_instr(dupi2_instr);
    comp_builder.push_instr(pushi3_instr);
    comp_builder.push_instr(subi_instr);
    comp_builder.push_instr(svswp_instr);
    comp_builder.push_instr(popn_instr);
    comp_builder.push_instr(ldswp_instr);
    comp_builder.push_instr(ret_instr);

    //println!("{:?}", compiler.builder.instructions);

    let comp_code = comp_builder.build();
    let code = compiler.get_resulting_code();

    assert_eq!(comp_code, code);
}


#[test]
pub fn test_compile_fn_decl() {
    let code = String::from("
        fn: main(arg: int) ~ int {
            var x: int = arg * 4;
            var y: int = x + 4;

            return y - 4;
        }
    ");

    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let decl_list_res = parser.parse_root_decl_list();

    assert!(decl_list_res.is_ok());
    let decl_list = decl_list_res.unwrap();

    let mut compiler = Compiler::new();
    compiler.reset_builder();
    compiler.push_default_module_context();
    
    let comp_res = compiler.compile_root_decl_list(decl_list);
    assert!(comp_res.is_ok());
    

    let mut comp_builder = Builder::new();

    let dupi0_instr = Instruction::new(Opcode::SDUPI) // x
        .with_operand::<i64>(&-8);
    let pushi0_instr = Instruction::new(Opcode::PUSHI) // x, 4
        .with_operand::<i64>(&4);
    let mul_instr = Instruction::new(Opcode::MULI); // 4x
    let dupi_instr = Instruction::new(Opcode::SDUPI) // 4x, 4x
        .with_operand::<i64>(&-8);
    let pushi_instr = Instruction::new(Opcode::PUSHI) // 4x, 4x, 4
        .with_operand::<i64>(&4);
    let addi_instr = Instruction::new(Opcode::ADDI); // 4x, 4x+4
    let dupi2_instr = Instruction::new(Opcode::SDUPI) // 4x, 4x+4, 4x+4
        .with_operand::<i64>(&-8);
    let pushi2_instr = Instruction::new(Opcode::PUSHI) // 4x, 4x+4, 4x+4, 4
        .with_operand::<i64>(&4);
    let subi_instr = Instruction::new(Opcode::SUBI); // 4x, 4x+4, 4x
    let svswp_instr = Instruction::new(Opcode::SVSWPI); // 4x, 4x+4
    let popn_instr = Instruction::new(Opcode::POPN) // 
        .with_operand::<u64>(&16);
    let ldswp_instr = Instruction::new(Opcode::LDSWPI); // 4x
    let ret_instr = Instruction::new(Opcode::RET);

    comp_builder.push_instr(dupi0_instr);
    comp_builder.push_instr(pushi0_instr);
    comp_builder.push_instr(mul_instr);
    comp_builder.push_instr(dupi_instr);
    comp_builder.push_instr(pushi_instr);
    comp_builder.push_instr(addi_instr);
    comp_builder.push_instr(dupi2_instr);
    comp_builder.push_instr(pushi2_instr);
    comp_builder.push_instr(subi_instr);
    comp_builder.push_instr(svswp_instr);
    comp_builder.push_instr(popn_instr);
    comp_builder.push_instr(ldswp_instr);
    comp_builder.push_instr(ret_instr);

    //println!("{:?}", compiler.builder.instructions);

    let main_uid = compiler.get_function_uid(&String::from("root::main"));

    let comp_code = comp_builder.build();
    let mut fn_map = HashMap::new();
    fn_map.insert(main_uid, 0);
    let comp_prog = Program::new()
        .with_code(comp_code)
        .with_functions(fn_map);
    let program_res = compiler.get_program();
    assert!(program_res.is_ok());
    let program = program_res.unwrap();
    assert_eq!(program, comp_prog);
}

#[test]
fn test_compile_expr_call() {
    let code = String::from("
        fn: five() ~ int {
            return 5;
        }
        fn: main() ~ int {
            var x: int = five();
            return x;
        }
    ");
    
    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let decl_list_res = parser.parse_root_decl_list();

    assert!(decl_list_res.is_ok());
    let decl_list = decl_list_res.unwrap();
    

    let mut compiler = Compiler::new();
    compiler.reset_builder();
    compiler.push_default_module_context();
    
    let comp_res = compiler.compile_root_decl_list(decl_list);
    assert!(comp_res.is_ok());
    

    let mut comp_builder = Builder::new();

    let five_uid = compiler.get_function_uid(&String::from("root::five"));
    let main_uid = compiler.get_function_uid(&String::from("root::main"));

    // five()
    {
        let pushi_instr = Instruction::new(Opcode::PUSHI)
            .with_operand::<i64>(&5);
        let svswp_instr = Instruction::new(Opcode::SVSWPI);
        let popn_instr = Instruction::new(Opcode::POPN)
            .with_operand::<u64>(&0);
        let ldswp_instr = Instruction::new(Opcode::LDSWPI);
        let ret_instr = Instruction::new(Opcode::RET);

        comp_builder.push_instr(pushi_instr);
        //comp_builder.push_instr(svswp_instr);
        //comp_builder.push_instr(popn_instr);
        //comp_builder.push_instr(ldswp_instr);
        comp_builder.push_instr(ret_instr);
    }
    // main()
    {
        let call_instr = Instruction::new(Opcode::CALL)
            .with_operand::<u64>(&five_uid);
        let sdupi_instr = Instruction::new(Opcode::SDUPI)
            .with_operand::<i64>(&-8);
        let svswp_instr = Instruction::new(Opcode::SVSWPI);
        let popn_instr = Instruction::new(Opcode::POPN)
            .with_operand::<u64>(&8);
        let ldswp_instr = Instruction::new(Opcode::LDSWPI);
        let ret_instr = Instruction::new(Opcode::RET);

        comp_builder.push_instr(call_instr);
        comp_builder.push_instr(sdupi_instr);
        comp_builder.push_instr(svswp_instr);
        comp_builder.push_instr(popn_instr);
        comp_builder.push_instr(ldswp_instr);
        comp_builder.push_instr(ret_instr);
    }

    //println!("Comparison builder instructions:");
    for instr in comp_builder.instructions.iter() {
        //println!("{:?}", instr);
    }

    //println!("Compiler builder instructions:");
    for instr in compiler.get_builder_ref().instructions.iter() {
        //println!("{:?}", instr);
    }

    let comp_code = comp_builder.build();
    let mut fn_map = HashMap::new();
    fn_map.insert(main_uid, 10);
    fn_map.insert(five_uid, 0);
    let comp_prog = Program::new()
        .with_code(comp_code)
        .with_functions(fn_map);
    let program_res = compiler.get_program();
    assert!(program_res.is_ok());
    let program = program_res.unwrap();
    assert_eq!(program, comp_prog);
}

#[test]
fn test_compile_expr_call_mod() {
    let code = String::from("
        mod: other {
            fn: five() ~ int {
                return 5;
            }
        }
        import other::five;
        fn: main() ~ int {
            var x: int = five();
            return x;
        }
    ");
    
    let mut lexer = Token::lexer(code.as_str());
    let parser = Parser::new(code.clone());
    let decl_list_res = parser.parse_root_decl_list();

    assert!(decl_list_res.is_ok());
    let decl_list = decl_list_res.unwrap();
    

    let mut compiler = Compiler::new();
    compiler.reset_builder();
    compiler.push_default_module_context();
    
    let comp_res = compiler.compile_root_decl_list(decl_list);
    assert!(comp_res.is_ok());
    

    let mut comp_builder = Builder::new();

    let five_uid = compiler.get_function_uid(&String::from("root::other::five"));
    let main_uid = compiler.get_function_uid(&String::from("root::main"));

    // five()
    {
        let pushi_instr = Instruction::new(Opcode::PUSHI)
            .with_operand::<i64>(&5);
        let svswp_instr = Instruction::new(Opcode::SVSWPI);
        let popn_instr = Instruction::new(Opcode::POPN)
            .with_operand::<u64>(&0);
        let ldswp_instr = Instruction::new(Opcode::LDSWPI);
        let ret_instr = Instruction::new(Opcode::RET);

        comp_builder.push_instr(pushi_instr);
        //comp_builder.push_instr(svswp_instr);
        //comp_builder.push_instr(popn_instr);
        //comp_builder.push_instr(ldswp_instr);
        comp_builder.push_instr(ret_instr);
    }
    // main()
    {
        let call_instr = Instruction::new(Opcode::CALL)
            .with_operand::<u64>(&five_uid);
        let sdupi_instr = Instruction::new(Opcode::SDUPI)
            .with_operand::<i64>(&-8);
        let svswp_instr = Instruction::new(Opcode::SVSWPI);
        let popn_instr = Instruction::new(Opcode::POPN)
            .with_operand::<u64>(&8);
        let ldswp_instr = Instruction::new(Opcode::LDSWPI);
        let ret_instr = Instruction::new(Opcode::RET);

        comp_builder.push_instr(call_instr);
        comp_builder.push_instr(sdupi_instr);
        comp_builder.push_instr(svswp_instr);
        comp_builder.push_instr(popn_instr);
        comp_builder.push_instr(ldswp_instr);
        comp_builder.push_instr(ret_instr);
    }

    //println!("Comparison builder instructions:");
    for instr in comp_builder.instructions.iter() {
        //println!("{:?}", instr);
    }

    //println!("Compiler builder instructions:");
    for instr in compiler.get_builder_ref().instructions.iter() {
        //println!("{:?}", instr);
    }

    let comp_code = comp_builder.build();
    let mut fn_map = HashMap::new();
    fn_map.insert(main_uid, 10);
    fn_map.insert(five_uid, 0);
    let comp_prog = Program::new()
        .with_code(comp_code)
        .with_functions(fn_map);
    let program_res = compiler.get_program();
    assert!(program_res.is_ok());
    let program = program_res.unwrap();
    assert_eq!(program, comp_prog);
}

#[test]
fn test_compile_stmt_call() {

}

use crate::parser::ast::Statement;

#[test]
fn test_compile_if() {
    let code = String::from("
        if true {
            var x: int = 1;
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let if_stmt_res = parser.parse_if(&mut lexer);
    assert!(if_stmt_res.is_ok());

    let mut compiler = Compiler::new();
    compiler.reset_builder();
    compiler.push_empty_context();
    compiler.push_default_module_context();

    let if_stmt = if_stmt_res.unwrap();

    let cmp_res = compiler.compile_if_stmt(&if_stmt);

    assert!(cmp_res.is_ok());

    //println!("Instructions:");
    for instr in compiler.get_builder_ref().instructions.iter() {
        //println!("{:?}", instr);
    }

    //println!("Code length: {}", compiler.get_builder_ref().get_current_offset());
}

use bincode::deserialize;

#[test]
fn test_compile_if_stmt_list() {
    let code = String::from("
        var x: int = 0;
        if true {
            x = 1;
        }
        var y: int = 4;
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let if_stmt_res = parser.parse_statement_list(&mut lexer);
    assert!(if_stmt_res.is_ok());

    let mut compiler = Compiler::new();
    compiler.reset_builder();
    compiler.push_empty_context();
    compiler.push_default_module_context();

    let if_stmt = if_stmt_res.unwrap();

    for stmt in if_stmt {
        let cmp_res = compiler.compile_statement(&stmt);
        assert!(cmp_res.is_ok());
    }

    //println!("Instructions:");
    for instr in compiler.get_builder_ref().instructions.iter() {
        //println!("{:?}", instr);
        if instr.opcode == Opcode::SMOVI {
            let operand: i64 = bincode::deserialize(&instr.operands).expect("Should be i64!");
            //println!("Operand for SMOVI: {}", operand);
        }
    }

    //println!("Code length: {}", compiler.get_builder_ref().get_current_offset());
}

#[test]
fn test_compile_if_neg() {
    let code = String::from("
        if 1 + 1 {
            var x: int = 1;
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let if_stmt_res = parser.parse_if(&mut lexer);
    assert!(if_stmt_res.is_ok());

    let mut compiler = Compiler::new();
    compiler.reset_builder();
    compiler.push_empty_context();
    compiler.push_default_module_context();

    let if_stmt = if_stmt_res.unwrap();

    let cmp_res = compiler.compile_if_stmt(&if_stmt);

    assert!(cmp_res.is_err());
}

#[test]
fn test_compile_while_stmt_list() {
    let code = String::from("
        var x: int = 0;
        while true {
            x = x + 1;
            if x > 10 {
                break;
            }
        }
        var y: int = 4;
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let while_stmt_res = parser.parse_statement_list(&mut lexer);
    assert!(while_stmt_res.is_ok());

    let mut compiler = Compiler::new();
    compiler.reset_builder();
    compiler.push_empty_context();
    compiler.push_default_module_context();

    let while_stmt = while_stmt_res.unwrap();

    for stmt in while_stmt {
        let cmp_res = compiler.compile_statement(&stmt);
        assert!(cmp_res.is_ok());
    }

    //println!("Instructions:");
    let mut size = 0;
    for instr in compiler.get_builder_ref().instructions.iter() {
        print!("Code pos: {}; ", size);
        //println!("{:?}", instr);
        if instr.opcode == Opcode::SMOVI {
            let operand: i64 = bincode::deserialize(&instr.operands).expect("Should be i64!");
            //println!("Operand for SMOVI: {}", operand);
        }
        if instr.opcode == Opcode::SDUPI {
            let operand: i64 = bincode::deserialize(&instr.operands).expect("Should be i64!");
            //println!("Operand for SDUPI: {}", operand);
        }
        size += instr.get_size();
    }

    //println!("Code length: {}", compiler.get_builder_ref().get_current_offset());
}