use salak::*;
use salak_factory::redis_default::RedisPool;

define_resource!(
    Env {
        redis1: RedisPool, ResourceBuilder::default()
        redis2: RedisPool, ResourceBuilder::default()
    }
);
fn main() -> Result<(), PropertyError> {
    let (_env, _res) = init()?;
    Ok(())
}
