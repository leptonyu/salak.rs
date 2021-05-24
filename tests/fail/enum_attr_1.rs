use salak::*;

#[derive(Debug, FromEnvironment)]
#[salak(prefix = "xxx")]
pub enum FailEnum {
    Fail,
}

fn main() {}
