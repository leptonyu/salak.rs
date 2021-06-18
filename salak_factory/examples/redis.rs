use std::sync::Arc;

use salak::*;
use salak_factory::env_log::Logger;
use salak_factory::redis_default::RedisPool;

generate_service!(RedisService {
  redis: RedisPool,
  #[salak(namespace = "hello", access = "pub")]
  back: Option<RedisPool>
});

fn main() -> Result<(), PropertyError> {
    let env = Salak::builder()
        .register_default_resource::<Logger>()?
        .register_default_resource::<RedisPool>()?
        .configure_args(app_info!())
        .build()?;
    let _service = env.get_service::<RedisService>()?;
    let _conn = _service.as_redis().get()?;
    Ok(())
}
