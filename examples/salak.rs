use std::collections::HashMap;

use salak::wrapper::NonEmptyVec;
use salak::*;

#[derive(FromEnvironment, Debug)]
#[salak(prefix = "salak")]
struct Config {
    #[salak(default = "world")]
    hello: String,
    world: Option<String>,
    #[salak(name = "hello")]
    hey: Option<String>,
    #[salak(default = 123)]
    num: u8,
    arr: Vec<u8>,
    #[salak(desc = "Non empty u8")]
    brr: NonEmptyVec<u8>,
    #[salak(desc = "map desc")]
    map: HashMap<String, u8>,
}

fn main() -> Result<(), PropertyError> {
    let _ = Salak::builder()
        .configure_description::<Config>()
        .configure_args(app_info!())
        .build()?;
    Ok(())
}
