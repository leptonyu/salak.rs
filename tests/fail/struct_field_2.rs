use salak::*;

#[derive(Debug, FromEnvironment)]
pub struct FailStruct {
    value: std::cell::RefCell<u8>,
}

fn main() {}
