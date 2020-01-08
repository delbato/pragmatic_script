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
pub struct ContainerDef {
    pub name: String,
    pub members: BTreeMap<usize, ContainerMemberDef> 
}

#[derive(Debug, Clone)]
pub struct ContainerMemberDef {
    pub name: String,
    pub var_type: Type
}

impl ContainerDef {
    pub fn new(name: String) -> ContainerDef {
        ContainerDef {
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

    pub fn add_member(&mut self, member: ContainerMemberDef) {
        let index = self.members.len();
        self.members.insert(index, member);
    }
}

impl ContainerMemberDef {
    pub fn new(name: String, var_type: Type) -> ContainerMemberDef {
        ContainerMemberDef {
            name: name, 
            var_type: var_type
        }
    }
}