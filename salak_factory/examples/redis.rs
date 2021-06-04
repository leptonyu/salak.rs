use salak::*;
use salak_factory::redis_default::RedisPool;

fn main() -> Result<(), PropertyError> {
    let env = Salak::builder()
        .register_resource::<RedisPool>(ResourceBuilder::default())
        .register_resource::<RedisPool>(ResourceBuilder::default().namespace("secondary"))
        .configure_args(app_info!())
        .build()?;
    let _pool1 = env.get_resource::<RedisPool>()?;
    let _pool2 = env.get_resource_by_namespace::<RedisPool>("secondary")?;
    // let conn = _pool1.get()?;
    Ok(())
}
