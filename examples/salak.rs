use std::collections::HashMap;

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
    #[salak(desc = "must at least have one")]
    brr: NonEmptyVec<u8>,
    #[salak(desc = "map desc")]
    map: HashMap<String, u8>,
}

fn main() -> Result<(), PropertyError> {
    let _ = Salak::builder()
        .add_config_desc::<Config>()
        .enable_args(app_info!())
        .build()?;
    Ok(())
}
