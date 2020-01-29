use crate::{
    vm::{
        core::{
            Core
        }
    },
    parser::{
        ast::Type
    },
    api::{
        error::{
            APIResult,
            APIError
        }
    },
    codegen::{
        register::{
            Register
        }
    }
};

use std::{
    collections::{
        HashMap
    }
};

use serde::{
    de::{
        DeserializeOwned
    },
    Serialize
};

pub struct Adapter<'c> {
    core: &'c mut Core,
    fn_signature: Option<HashMap<usize, i64>>
}

impl<'c> Adapter<'c> {
    /// Creates a new Adapter instance
    pub fn new(core: &'c mut Core) -> Adapter {
        Adapter {
            core: core,
            fn_signature: None
        }
    }

    /// With a function signature
    pub fn with_fn_signature(mut self, fn_signature: Vec<usize>) -> Adapter<'c> {
        let mut signature = HashMap::new();
        let mut stack_index = 0;
        let mut arg_index = fn_signature.len();
        for arg_size in fn_signature.into_iter().rev() {
            stack_index -= arg_size as isize;
            arg_index -= 1;
            signature.insert(arg_index, stack_index as i64);
        }
        self.fn_signature = Some(signature);
        self
    }

    pub fn get_arg<T: DeserializeOwned>(&mut self, arg_index: usize) -> APIResult<T> {
        let arg_offset = {
            let fn_sig = self.fn_signature.as_ref()
                .ok_or(APIError::NoFnSignature)?;
            fn_sig.get(&arg_index)
                .cloned()
                .ok_or(APIError::Unknown)?
        };

        let sp = {
            self.core.reg(Register::SP.into())
                .map_err(|_| APIError::Unknown)?
                .get::<u64>()
        };

        self.core.mem_get((sp, arg_offset as i16))
            .map_err(|_| APIError::ArgDeserializeError)
    }

    pub fn push_stack<T: Serialize>(&mut self, item: T) -> APIResult<()> {
        self.core.push_stack(item)
            .map_err(|_| APIError::ArgSerializeError)
    }
}