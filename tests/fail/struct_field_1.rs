use salak::*;

#[derive(Debug, FromEnvironment)]
pub struct FailStruct {
    #[salak(prefix = "xxx")]
    value: u8,
}

fn main() {}
