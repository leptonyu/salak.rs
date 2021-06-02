use salak::*;
use salak_factory::redis_default::{RedisConfig, RedisPool};

fn main() -> Result<(), PropertyError> {
    let env = Salak::builder()
        .configure_description::<RedisConfig>()
        .configure_args(app_info!())
        .build()?;
    let _p = env.init::<RedisPool>()?;
    // _p.get()?;
    Ok(())
}
