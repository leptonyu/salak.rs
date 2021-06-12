use salak::*;
use salak_factory::{metric::Metric, postgresql::PostgresPool};

fn main() -> Result<(), PropertyError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()?;
    Salak::builder()
        .register_default_resource::<Metric>()?
        .register_default_resource::<PostgresPool>()?
        .configure_args(app_info!())
        .build()?
        .run()
}
