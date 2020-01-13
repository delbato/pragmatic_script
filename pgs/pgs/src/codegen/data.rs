use std::{
    collections::{
        BTreeMap,
        HashMap
    },
    ops::{
        Range
    }
};

pub struct Data {
    raw_data: Vec<u8>,
    pointers: BTreeMap<usize, Range<usize>>,
    strings: HashMap<String, (usize, usize)>
}

impl Data {
    pub fn new() -> Data {
        Data {
            raw_data: Vec::new(),
            pointers: BTreeMap::new(),
            strings: HashMap::new()
        }
    }

    pub fn add_string(&mut self, string: &String) -> (usize, usize) {
        if let Some(tup) = self.strings.get(string) {
            return tup.clone();
        }
        let addr = self.raw_data.len();
        let mut data = Vec::from(string.as_bytes());
        let len = data.len();
        self.raw_data.append(&mut data);
        self.pointers.insert(addr, addr..addr+len);
        self.strings.insert(string.clone(), (len, addr));
        (len, addr)
    }

    pub fn get_bytes(&self) -> Vec<u8> {
        self.raw_data.clone()
    }

    pub fn get_pointers(&self) -> BTreeMap<usize, Range<usize>> {
        self.pointers.clone()
    }
}