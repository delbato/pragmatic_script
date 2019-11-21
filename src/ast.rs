use std::{
    collections::{
        HashMap
    }
};

#[derive(PartialEq)]
pub enum Expression {
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    Variable(String),
    Addition(Box<Expression>, Box<Expression>),
    Subtraction(Box<Expression>, Box<Expression>),
    Multiplication(Box<Expression>, Box<Expression>),
    Division(Box<Expression>, Box<Expression>)
}

#[derive(PartialEq)]
pub struct FunctionDeclArgs {
    pub name: String,
    pub arguments: HashMap<String, Type>,
    pub returns: Type,
    pub code_block: Option<Vec<Statement>>
}

#[derive(PartialEq)]
pub enum Declaration {
    Function(FunctionDeclArgs)
}

#[derive(PartialEq)]
pub struct VariableDeclArgs {
    pub var_type: Type,
    pub name: String,
    pub assignment: Box<Expression>
}

#[derive(PartialEq)]
pub enum Statement {
    VariableDecl(VariableDeclArgs),
    Assignment(String, Box<Expression>),
    Call(String, Vec<Expression>),
    Return(Box<Expression>)
}

#[derive(PartialEq)]
pub enum Type {
    Int,
    String,
    Float
}
