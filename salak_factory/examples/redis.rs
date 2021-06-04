use salak::*;
use salak_factory::redis_default::RedisPool;

fn main() -> Result<(), PropertyError> {
    let env = Salak::builder()
        .register_resource::<RedisPool>(ResourceBuilder::default())
        .register_resource::<RedisPool>(ResourceBuilder::default().namespace("secondary"))
        .build()?;
    let _pool1 = env.get_resource::<RedisPool>()?;
    let _pool2 = env.get_resource_by_namespace::<RedisPool>("secondary")?;
    Ok(())
}
