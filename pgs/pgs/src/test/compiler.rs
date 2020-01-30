use crate::{
    codegen::{
        compiler::{
            Compiler
        },
        program::{
            Program
        },
        instruction::{
            Instruction
        }
    },
    parser::{
        parser::Parser,
        lexer::Token
    }
};

use pglex::prelude::Lexable;

#[test]
fn test_compile_stmt_var_decl() {
    let code = String::from("
        fn: main() {
            var x: int = (4 + 4) * 2;
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let decl_list_res = parser.parse_decl_list(&mut lexer, &[]);
    assert!(decl_list_res.is_ok());

    let decl_list = decl_list_res.unwrap();

    for stmt in decl_list.iter() {
        println!("{:?}", stmt);
    }

    let mut compiler = Compiler::new();
    let compile_res = compiler.compile_root(&decl_list);
    println!("{:?}", compile_res);
    assert!(compile_res.is_ok());

    let builder = compiler.get_builder();

    for instr in builder.instructions.iter() {
        println!("{:?}", instr);
    }
}


#[test]
fn test_compile_if() {
    let code = String::from("
        fn: main() {
            var x: int = (4 + 4) * 2;
            if x < 8 {
                var z: int = 4;
            }
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let decl_list_res = parser.parse_decl_list(&mut lexer, &[]);
    assert!(decl_list_res.is_ok());

    let decl_list = decl_list_res.unwrap();

    for stmt in decl_list.iter() {
        println!("{:?}", stmt);
    }

    let mut compiler = Compiler::new();
    let compile_res = compiler.compile_root(&decl_list);
    println!("{:?}", compile_res);
    assert!(compile_res.is_ok());

    let builder = compiler.get_builder();

    for instr in builder.instructions.iter() {
        println!("{:?}", instr);
    }
}

#[test]
fn test_compile_var_assign() {
    let code = String::from("
        fn: main() {
            var x: int = 0;
            if x < 1 {
                x += 1;
            }
        }
    ");

    let parser = Parser::new(code.clone());
    let mut lexer = Token::lexer(code.as_str());

    let decl_list_res = parser.parse_decl_list(&mut lexer, &[]);
    assert!(decl_list_res.is_ok());

    let decl_list = decl_list_res.unwrap();

    let mut compiler = Compiler::new();
    let compile_res = compiler.compile_root(&decl_list);
    println!("{:?}", compile_res);
    assert!(compile_res.is_ok());

    let builder = compiler.get_builder();

    for instr in builder.instructions.iter() {
        println!("{:?}", instr);
    }
}