use std::{
    collections::{
        HashMap,
        BTreeMap
    }
};

#[derive(PartialEq, Debug, Clone)]
pub enum Expression {
    IntLiteral(i64),
    FloatLiteral(f32),
    StringLiteral(String),
    BoolLiteral(bool),
    Variable(String),
    MemberAccess(Box<Expression>, Box<Expression>),
    Deref(Box<Expression>),
    Ref(Box<Expression>),
    Call(String, Vec<Expression>),
    Addition(Box<Expression>, Box<Expression>),
    Subtraction(Box<Expression>, Box<Expression>),
    Multiplication(Box<Expression>, Box<Expression>),
    Division(Box<Expression>, Box<Expression>),
    Not(Box<Expression>),
    Equals(Box<Expression>, Box<Expression>),
    NotEquals(Box<Expression>, Box<Expression>),
    GreaterThan(Box<Expression>, Box<Expression>),
    LessThan(Box<Expression>, Box<Expression>),
    GreaterThanEquals(Box<Expression>, Box<Expression>),
    LessThanEquals(Box<Expression>, Box<Expression>),
    Assign(Box<Expression>, Box<Expression>),
    AddAssign(Box<Expression>, Box<Expression>),
    SubAssign(Box<Expression>, Box<Expression>),
    MulAssign(Box<Expression>, Box<Expression>),
    DivAssign(Box<Expression>, Box<Expression>),
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
            Expression::MemberAccess(lhs, rhs) => {
                println!("{} Member access:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1);
            },
            Expression::Call(fn_name, args) => {
                println!("{} Call \"{}\":", baseline, fn_name);
                println!("{} Arguments:", baseline);
                for arg in args.iter() {
                    arg.print(n + 1);
                }
            },
            Expression::Assign(lhs, rhs) => {
                println!("{} Assign:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            Expression::AddAssign(lhs, rhs) => {
                println!("{} AddAssign:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            Expression::SubAssign(lhs, rhs) => {
                println!("{} SubAssign:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            Expression::MulAssign(lhs, rhs) => {
                println!("{} MulAssign:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            Expression::DivAssign(lhs, rhs) => {
                println!("{} DivAssign:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            _ => {
                println!("{} Other:", baseline);
            }
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
    Divide,
    Equals,
    NotEquals,
    GreaterThan,
    GreaterThanEquals,
    LessThan,
    LessThanEquals,
    Not
}

#[derive(PartialEq, Debug, Clone)]
pub struct FunctionDeclArgs {
    pub name: String,
    pub arguments: BTreeMap<usize, (String, Type)>,
    pub returns: Type,
    pub code_block: Option<Vec<Statement>>
}

#[derive(PartialEq, Debug, Clone)]
pub struct ContainerDeclArgs {
    pub name: String,
    pub members: BTreeMap<usize, (String, Type)>
}

#[derive(PartialEq, Debug)]
pub enum Declaration {
    Function(FunctionDeclArgs),
    Module(String, Vec<Declaration>),
    Container(ContainerDeclArgs),
    Import(String, String),
    Impl(String, String, Vec<Declaration>)
}

#[derive(PartialEq, Debug, Clone)]
pub struct VariableDeclArgs {
    pub var_type: Type,
    pub name: String,
    pub assignment: Box<Expression>
}

#[derive(PartialEq, Debug, Clone)]
pub enum Statement {
    VariableDecl(VariableDeclArgs),
    Assignment(String, Box<Expression>),
    Call(String, Vec<Expression>),
    Return(Box<Expression>),
    Loop(Vec<Statement>),
    While(Box<Expression>, Vec<Statement>),
    Break,
    Continue,
    Expression(Expression),
    If(Box<Expression>, Vec<Statement>),
    IfElse(Box<Expression>, Vec<Statement>, Vec<Statement>),
    IfElseIf(Box<Expression>, Vec<Statement>, Vec<(Box<Expression>, Vec<Statement>)>)
}

#[derive(PartialEq, Debug, Clone)]
pub enum Type {
    Int,
    String,
    Float,
    Bool,
    Auto,
    Array(Box<Type>, usize),
    AutoArray(Box<Type>),
    Other(String),
    Tuple(Vec<Type>),
    Reference(Box<Type>)
}
