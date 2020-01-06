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
    pub functions: HashMap<String, Function>
}

impl Container {
    pub fn new(name: String) -> Container {
        Container {
            name: name,
            members: HashMap::new(),
            functions: HashMap::new()
        }
    }
}

pub struct ContainerMember {
    pub name: String,
    pub var_type: Type
}

impl ContainerMember {
    pub fn new(name: String, var_type: Type) -> ContainerMember {
        ContainerMember {
            name: name,
            var_type: var_type
        }
    }
}