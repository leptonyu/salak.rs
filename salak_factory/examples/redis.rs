use std::sync::Arc;

use salak::*;
use salak_factory::redis_default::RedisPool;

#[derive(Service)]
struct RedisService {
    redis: Arc<RedisPool>,
    #[allow(dead_code)]
    #[salak(namespace = "hello", access = "pub")]
    redis2: Option<Arc<RedisPool>>,
}

fn main() -> Result<(), PropertyError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()?;
    let env = Salak::builder()
        .register_default_resource::<RedisService>()?
        .register_default_resource::<RedisPool>()?
        .configure_args(app_info!())
        .build()?;
    let _service = env.get_resource::<RedisService>()?;
    let _conn = _service.as_redis().get()?;
    Ok(())
}
