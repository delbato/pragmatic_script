use std::{
    collections::{
        HashMap,
        BTreeMap
    }
};

#[derive(PartialEq, Debug)]
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

impl Expression {
    pub fn print(&self, n: u8) {
        let mut baseline = String::new();
        for i in 0..n {
            baseline += "----";
        }
        match self {
            Expression::IntLiteral(int) => {
                println!("{} Int:{}", baseline, int);
            },
            Expression::FloatLiteral(float) => {
                println!("{} Float:{}", baseline, float);
            },
            Expression::StringLiteral(string) => {
                println!("{} String:{}", baseline, string);
            },
            Expression::Variable(variable) => {
                println!("{} Variable:{}", baseline, variable);
            },
            Expression::Addition(lhs, rhs) => {
                println!("{} Addition:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            Expression::Subtraction(lhs, rhs) => {
                println!("{} Subtraction:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            Expression::Multiplication(lhs, rhs) => {
                println!("{} Multiplication:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            Expression::Division(lhs, rhs) => {
                println!("{} Division:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum Operator {
    OpenParan,
    CloseParan,
    Plus,
    Minus,
    Times,
    Divide
}

#[derive(PartialEq, Debug)]
pub struct FunctionDeclArgs {
    pub name: String,
    pub arguments: BTreeMap<usize, (String, Type)>,
    pub returns: Type,
    pub code_block: Option<Vec<Statement>>
}

#[derive(PartialEq, Debug)]
pub enum Declaration {
    Function(FunctionDeclArgs)
}

#[derive(PartialEq, Debug)]
pub struct VariableDeclArgs {
    pub var_type: Type,
    pub name: String,
    pub assignment: Box<Expression>
}

#[derive(PartialEq, Debug)]
pub enum Statement {
    VariableDecl(VariableDeclArgs),
    Assignment(String, Box<Expression>),
    Call(String, Vec<Expression>),
    Return(Box<Expression>)
}

#[derive(PartialEq, Debug)]
pub enum Type {
    Int,
    String,
    Float,
    Custom(String)
}
