use crate::{
    api::{
        function::{
            Function,
            FunctionError,
            FunctionResult
        }
    },
    parser::{
        ast::Type
    }
};


use std::{
    collections::{
        HashMap
    }
};

pub struct Container {
    pub name: String,
    pub members: HashMap<String, ContainerMember>,
    pub member_size: usize,
    pub functions: HashMap<String, Function>
}

impl Container {
    pub fn new(name: String) -> Container {
        Container {
            name: name,
            members: HashMap::new(),
            functions: HashMap::new(),
            member_size: 0
        }
    }

    pub fn with_member(mut self, name: String, var_type: Type, var_size: usize) -> Container {
        self.members.insert(name.clone(), ContainerMember::new(name, var_type, self.member_size));
        self.member_size += var_size;
        self
    }
}

pub struct ContainerMember {
    pub name: String,
    pub var_type: Type,
    pub offset: usize,
}

impl ContainerMember {
    pub fn new(name: String, var_type: Type, offset: usize) -> ContainerMember {
        ContainerMember {
            name: name,
            var_type: var_type,
            offset: offset
        }
    }
}

pub struct ContainerInstance<'d> {
    pub data_slice: &'d mut [u8]
}

impl<'d> ContainerInstance<'d> {
    pub fn get_member(&self, name: String) -> Vec<u8> {
        Vec::new()
    }
}