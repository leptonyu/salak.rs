use std::sync::Arc;

use salak::*;
use salak_factory::redis_default::RedisPool;

#[derive(Service)]
struct RedisService {
    _redis: Arc<RedisPool>,
    #[salak(namespace = "secondary")]
    _redi2: Arc<RedisPool>,
}

fn main() -> Result<(), PropertyError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()?;
    let env = Salak::builder()
        .register_default_resource::<RedisService>()?
        .register_default_resource::<RedisPool>()?
        .register_resource::<RedisPool>(ResourceBuilder::new("secondary"))?
        .configure_args(app_info!())
        .build()?;
    let _service = env.get_resource::<RedisService>()?;
    // let conn = _service._redis.get()?;
    Ok(())
}
