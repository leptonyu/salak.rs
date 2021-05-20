use salak::*;
use salak_factory::{redis_default::RedisConfig, Factory};

fn main() -> Result<(), PropertyError> {
    let env = Salak::builder()
        .add_config_desc::<RedisConfig>()
        .enable_args(app_info!())
        .build()?;
    let _p = env.build::<RedisConfig>()?;
    // _p.get()?;
    Ok(())
}
