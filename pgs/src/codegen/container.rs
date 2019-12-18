use crate::{
    parser::{
        ast::{
            Type
        }
    },
    codegen::{
        compiler::{
            Compiler,
            CompilerError,
            CompilerResult
        }
    }
};

use std::{
    collections::{
        BTreeMap
    }
};

#[derive(Debug, Clone)]
pub struct Container {
    pub name: String,
    pub members: BTreeMap<usize, ContainerMember> 
}

#[derive(Debug, Clone)]
pub struct ContainerMember {
    pub name: String,
    pub var_type: Type
}

impl Container {
    pub fn new(name: String) -> Container {
        Container {
            name: name,
            members: BTreeMap::new()
        }
    }

    pub fn offset_of(&self, compiler: &Compiler, member_name: &String) -> CompilerResult<usize> {
        let mut byte_offset = 0;
        let mut found = false;
        for (_, container_member) in self.members.iter() {
            if container_member.name == *member_name {
                found = true;
                break;
            }
            byte_offset += compiler.size_of_type(&container_member.var_type)?;
        }
        if !found {
            return Err(CompilerError::UnknownVariable);
        }
        Ok(byte_offset)
    }

    pub fn size(&self, compiler: &Compiler) -> CompilerResult<usize> {
        let mut byte_size = 0;
        for (_, container_member) in self.members.iter() {
            byte_size += compiler.size_of_type(&container_member.var_type)?;
        }
        Ok(byte_size)
    }
}

impl ContainerMember {
    pub fn new(name: String, var_type: Type) -> ContainerMember {
        ContainerMember {
            name: name, 
            var_type: var_type
        }
    }
}