use salak::*;

#[derive(Debug, FromEnvironment)]
pub enum FailEnum {
    #[salak(default = "xxx")]
    Fail,
}

fn main() {}
