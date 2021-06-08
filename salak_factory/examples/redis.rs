use std::sync::Arc;

use salak::*;
use salak_factory::redis_default::RedisPool;

struct Service {
    _redis: Arc<RedisPool>,
}

impl Resource for Service {
    type Config = ();

    type Customizer = ();

    fn create(
        _: Self::Config,
        factory: &FactoryContext<'_>,
        _: impl FnOnce(&mut Self::Customizer, &Self::Config) -> Result<(), PropertyError>,
    ) -> Result<Self, PropertyError> {
        Ok(Self {
            _redis: factory.get_resource()?,
        })
    }

    fn register_dependent_resources(builder: &mut FactoryBuilder<'_>) {
        builder.register_default_resource::<RedisPool>();
    }
}

fn main() -> Result<(), PropertyError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()?;
    let env = Salak::builder()
        .register_default_resource::<Service>()
        .register_resource::<RedisPool>(ResourceBuilder::default().namespace("secondary"))
        .configure_args(app_info!())
        .build()?;
    let _pool1 = env.get_resource::<RedisPool>()?;
    let _pool2 = env.get_resource_by_namespace::<RedisPool>("secondary")?;
    // let conn = _pool1.get()?;
    Ok(())
}
