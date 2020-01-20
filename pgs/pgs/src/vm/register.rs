pub union Register {
    pub uint64: u64,
    pub int64: i64,
    pub float: f32,
    pub boolean: bool
}

impl Register {
    pub fn new() -> Register {
        Register {
            uint64: 0
        }
    }
}

pub trait RegisterAccess<T> {
    fn get(&self) -> T;
    fn set(&mut self, item: T);
}

impl RegisterAccess<i64> for Register {
    fn get(&self) -> i64 {
        unsafe {
            self.int64
        }
    }
    fn set(&mut self, item: i64) {
        self.int64 = item;
    }
}

impl RegisterAccess<u64> for Register {
    fn get(&self) -> u64 {
        unsafe {
            self.uint64
        }
    }
    fn set(&mut self, item: u64) {
        self.uint64 = item;
    }
}

impl RegisterAccess<f32> for Register {
    fn get(&self) -> f32 {
        unsafe {
            self.float
        }
    }
    fn set(&mut self, item: f32) {
        self.float = item;
    }
}

impl RegisterAccess<bool> for Register {
    fn get(&self) -> bool {
        unsafe {
            self.boolean
        }
    }
    fn set(&mut self, item: bool) {
        self.boolean = item;
    }
}