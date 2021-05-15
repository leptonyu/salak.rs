use salak::*;
use salak_factory::{redis_default::RedisConfig, Factory};

fn main() -> Result<(), PropertyError> {
    let env = Salak::new()?;
    let _p = env.build::<RedisConfig>()?;
    // _p.get()?;
    Ok(())
}
